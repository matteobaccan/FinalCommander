//! Writing a dBase table from the viewer's table mode: the dialogs in front of
//! `fcmd_view::dbf_write` (SPEC 05 §1.11).
//!
//! The rules are the author's, of 25 November 2026, and they are about tables that Clipper
//! and Harbour programs still open every day:
//!
//! * before the first change to a table in a session a dialog says the table is about to be
//!   altered, and makes a dated copy of it and of its memo file — or the change is not made;
//! * a table with index files beside it — files named after it, `PIPPO*.NTX` for
//!   `PIPPO.DBF` — is written only once the indexes have been named and the answer was to
//!   go on: this program does not keep them up to date, and whoever owns them has to
//!   rebuild them afterwards (the same day's second rule, which took the place of the
//!   refusal the first one made). Asked once for a table in a session, right after the
//!   backup, which copies the indexes too; an index that turns up later is asked about
//!   again;
//! * `Pack` says how many records it removes before it removes them, and deleting or
//!   recalling the marked records says how many those are.
//!
//! Nothing here touches the disk but one look at the folder for index files: every write,
//! the backup, the count for `Pack` and an export are a [`Request::Dbf`] done on a worker,
//! which opens the table for that write alone and answers with an [`Event::Dbf`].
//!
//! What only reads is here as well, because it is the same table and the same keys: the
//! whole text of a memo, the sums of the numeric columns, the code page of a table that
//! names none.
//!
//! Left out of the compact build with the reader (`--no-default-features`), where the
//! commands say that dBase tables are not in it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use fcmd_view::dbf::{Dbf, Page};
use fcmd_view::dbf_write::{self, Exported, Packed, Refused, Stamp, WriteError};
use fcmd_view::MemoShown;

use super::*;

/// What the session remembers about the tables it has written.
#[derive(Debug, Default)]
pub(super) struct DbfSession {
    /// The tables whose backup was made: the dialog is asked once for each, and the changes
    /// after the first go straight through.
    backed_up: HashSet<PathBuf>,
    /// The index files of each table that were named and agreed to be left stale. An index
    /// that is not among them is asked about before the next write.
    agreed: HashMap<PathBuf, Vec<PathBuf>>,
    /// The code page each table that names none was said to be in: it holds for the table
    /// for as long as the program runs, however often the table is opened.
    pages: HashMap<PathBuf, Page>,
    /// The moment a test wants every write and every backup to carry, in place of the clock.
    #[cfg(test)]
    pub(super) clock: Option<Stamp>,
}

/// What was asked of a table, by key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbfAction {
    Edit,
    Append,
    /// Delete the record under the cursor, or recall it.
    Toggle,
    Pack,
    /// Delete every marked record, or recall every one.
    Marked {
        deleted: bool,
    },
}

/// What a confirmation about a table does on `Enter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbfStep {
    /// Make the backup, and then do what was asked.
    Backup { table: PathBuf, then: DbfAction },
    /// Go on although these index files will be left stale, and then do what was asked —
    /// or nothing more, when the question came back from a write that found a new index.
    Indexes {
        table: PathBuf,
        files: Vec<PathBuf>,
        then: Option<DbfAction>,
    },
    /// Take the deleted records out.
    Pack { table: PathBuf },
    /// Delete these records, or recall them.
    Marked {
        table: PathBuf,
        records: Vec<usize>,
        deleted: bool,
    },
}

/// One piece of work on a table, for the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbfWork {
    Backup {
        then: DbfAction,
    },
    Cell {
        record: usize,
        field: usize,
        text: String,
    },
    Deleted {
        record: usize,
        deleted: bool,
    },
    DeletedMany {
        records: Vec<usize>,
        deleted: bool,
    },
    Append,
    /// Count the deleted records, which is what `Pack` asks about before it runs.
    Count,
    Pack,
    /// Write these records to a new file: comma-separated text, or a table like this one.
    Export {
        records: Vec<usize>,
        target: PathBuf,
        csv: bool,
    },
}

/// The work, the table it is for, the header the viewer has of it — a write is refused when
/// the file no longer has those fields — and the moment it is asked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbfAsk {
    pub table: PathBuf,
    pub shown: Dbf,
    pub at: Stamp,
    /// The index files somebody agreed to leave stale: a write goes on past these and no
    /// others.
    pub agreed: Vec<PathBuf>,
    /// The table's memo file, where it has one.
    pub memo: Option<PathBuf>,
    pub work: DbfWork,
}

/// What the worker did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbfDone {
    BackedUp {
        files: Vec<PathBuf>,
        then: DbfAction,
    },
    Cell {
        record: usize,
        field: usize,
    },
    Deleted {
        record: usize,
        deleted: bool,
    },
    DeletedMany {
        changed: usize,
        deleted: bool,
    },
    Appended {
        record: usize,
    },
    Exported {
        target: PathBuf,
        done: Exported,
    },
    Counted {
        deleted: usize,
    },
    Packed(Packed),
}

/// Why it did not.
#[derive(Debug)]
pub enum DbfFailed {
    /// The backup could not be made, so the change it was for was not made either.
    Backup(std::io::Error),
    Write(WriteError),
    /// `Pack` failed. Apart from the others because the viewer let go of the file for it and
    /// has to open it again whatever happened.
    Pack(WriteError),
}

/// The worker's side: do the work, on whatever thread this is called from.
pub fn perform(ask: &DbfAsk) -> Result<DbfDone, DbfFailed> {
    let (table, shown, at) = (ask.table.as_path(), &ask.shown, ask.at);
    let write = DbfFailed::Write;
    // Asked here, at the moment of the write and not when the key was pressed: an index
    // another program made in between is one nobody has agreed to leave stale.
    let indexes = || dbf_write::check_indexes(table, &ask.agreed);
    match &ask.work {
        DbfWork::Backup { then } => dbf_write::backup(table, at)
            .map(|files| DbfDone::BackedUp { files, then: *then })
            .map_err(DbfFailed::Backup),
        DbfWork::Cell {
            record,
            field,
            text,
        } => indexes()
            .and_then(|()| dbf_write::set_cell(table, shown, *record, *field, text, at))
            .map(|()| DbfDone::Cell {
                record: *record,
                field: *field,
            })
            .map_err(write),
        DbfWork::Deleted { record, deleted } => indexes()
            .and_then(|()| dbf_write::set_deleted(table, shown, *record, *deleted, at))
            .map(|()| DbfDone::Deleted {
                record: *record,
                deleted: *deleted,
            })
            .map_err(write),
        DbfWork::DeletedMany { records, deleted } => indexes()
            .and_then(|()| dbf_write::set_deleted_many(table, shown, records, *deleted, at))
            .map(|changed| DbfDone::DeletedMany {
                changed,
                deleted: *deleted,
            })
            .map_err(write),
        DbfWork::Append => indexes()
            .and_then(|()| dbf_write::append(table, shown, at))
            .map(|record| DbfDone::Appended { record })
            .map_err(write),
        DbfWork::Count => dbf_write::count_deleted(table, shown)
            .map(|deleted| DbfDone::Counted { deleted })
            .map_err(write),
        DbfWork::Pack => indexes()
            .and_then(|()| dbf_write::pack(table, shown, at))
            .map(DbfDone::Packed)
            .map_err(DbfFailed::Pack),
        // The table is only read, so its indexes have nothing to fear from this.
        DbfWork::Export {
            records,
            target,
            csv,
        } => {
            let memo = ask.memo.as_deref();
            if *csv {
                dbf_write::export_csv(table, memo, shown, records, target)
            } else {
                dbf_write::export_dbf(table, memo, shown, records, target, at)
            }
            .map(|done| DbfDone::Exported {
                target: target.clone(),
                done,
            })
            .map_err(write)
        }
    }
}

/// The memo file beside a dBase table, if the table has one and it is there.
///
/// dBase writes `.dbt`, FoxPro writes `.fpt`, and Visual FoxPro gives each of the tables it
/// keeps its own things in a memo file of its own — `.dct` beside a `.dbc`, `.sct` beside
/// an `.scx`. Either may be spelled in capitals beside a table spelled in capitals — a DOS
/// habit that outlived DOS. Nothing else is guessed at: a file that is no table has no memo
/// file, and a name that is there is used as it is.
pub(super) fn memo_file(table: &std::path::Path) -> Option<std::path::PathBuf> {
    let name = table.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = name.rsplit_once('.')?;
    let upper = ext.chars().all(|c| c.is_ascii_uppercase());
    for memo in dbf_write::memo_extensions(&ext.to_ascii_lowercase()) {
        let memo = if upper {
            memo.to_uppercase()
        } else {
            memo.to_string()
        };
        let beside = table.with_file_name(format!("{stem}.{memo}"));
        if beside.is_file() {
            return Some(beside);
        }
        // And the other spelling, for a table whose two files disagree about it.
        let beside = table.with_file_name(format!("{stem}.{}", flip_case(&memo)));
        if beside.is_file() {
            return Some(beside);
        }
    }
    None
}

/// `dbt` ↔ `DBT`: the other spelling of an extension, for the case above.
fn flip_case(ext: &str) -> String {
    if ext.chars().all(|c| c.is_ascii_uppercase()) {
        ext.to_lowercase()
    } else {
        ext.to_uppercase()
    }
}

/// A field's type as a dBase programmer writes it: `C 10`, `N 7.2`, `D`.
fn type_label(kind: char, len: usize, decimals: u8) -> String {
    match kind {
        'D' | 'L' | 'M' => kind.to_string(),
        'N' | 'F' if decimals > 0 => format!("{kind} {len}.{decimals}"),
        _ => format!("{kind} {len}"),
    }
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A sentence as the lines of a dialog. The dialogs here explain themselves, and a dialog
/// draws its lines as they are given.
fn lines_of(text: &str) -> Vec<String> {
    const WIDTH: usize = 60;
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > WIDTH {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

impl App {
    /// The moment a write is asked at, in local time: the header's date and a backup's name.
    fn dbf_now(&self) -> Stamp {
        #[cfg(test)]
        if let Some(fixed) = self.dbf.clock {
            return fixed;
        }
        Stamp::parse(&fcmd_ui::format_stamp(std::time::SystemTime::now())).unwrap_or(Stamp {
            year: 1980,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        })
    }

    /// The dBase table the viewer in front is showing: where it is on disk, and its header.
    /// Otherwise the reason there is nothing to write, already said.
    fn dbf_table(&mut self) -> Option<(PathBuf, Dbf)> {
        let found = match &mut self.mode {
            Mode::Viewer(viewer) => {
                let table = viewer.mode == ViewerMode::Table;
                // A table inside an archive or fetched from a server is a copy in a
                // temporary folder: writing it would change nothing anybody keeps.
                let local = viewer.path.is_local() && !viewer.path.in_container();
                let path = viewer.path.as_path().to_path_buf();
                match viewer.dbf().cloned() {
                    Some(dbf) if local => Ok((path, dbf)),
                    _ if table => Err("dbf-only"),
                    _ => Err("viewer-not-table"),
                }
            }
            _ => return None,
        };
        match found {
            Ok(found) => Some(found),
            Err(id) => {
                let m = self.i18n.t(id);
                self.set_error(m);
                None
            }
        }
    }

    /// The viewer that is showing `table`, in front or waiting under one of its dialogs.
    fn dbf_viewer(&mut self, table: &Path) -> Option<&mut Viewer> {
        let viewer = match &mut self.mode {
            Mode::Viewer(viewer) => Some(viewer),
            _ => self.viewer_under_prompt.as_mut(),
        }?;
        (viewer.path.is_local() && viewer.path.as_path() == table).then_some(&mut **viewer)
    }

    /// Put the viewer under the dialog that is about to open, where `restore_viewer` finds it.
    fn dbf_park(&mut self) {
        if let Mode::Viewer(_) = &self.mode {
            if let Mode::Viewer(viewer) = std::mem::replace(&mut self.mode, Mode::Normal) {
                self.viewer_under_prompt = Some(viewer);
            }
        }
    }

    /// The dBase table the viewer in front is showing, wherever its bytes came from: what
    /// the commands that only read ask. Otherwise the reason, already said.
    fn dbf_shown(&mut self) -> bool {
        let found = match &mut self.mode {
            Mode::Viewer(viewer) => {
                let table = viewer.mode == ViewerMode::Table;
                match viewer.dbf() {
                    Some(_) => Ok(()),
                    None if table => Err("dbf-not-dbase"),
                    None => Err("viewer-not-table"),
                }
            }
            _ => return false,
        };
        if let Err(id) = found {
            let m = self.i18n.t(id);
            self.set_error(m);
        }
        found.is_ok()
    }

    /// The keys of a dBase table in table mode: the ones that write it, and the ones that
    /// read more of it than a row of cells.
    pub(super) fn dbf_command(&mut self, cmd: Command) {
        let action = match cmd {
            Command::ViewerDbfMemo => return self.dbf_memo(),
            Command::ViewerDbfSums => return self.dbf_sums(),
            Command::ViewerDbfCodepage => return self.dbf_codepage(),
            Command::ViewerDbfExport => return self.dbf_export(),
            Command::ViewerDbfEdit => DbfAction::Edit,
            Command::ViewerDbfAppend => DbfAction::Append,
            Command::ViewerDbfDelete => DbfAction::Toggle,
            Command::ViewerDbfDeleteMarked => DbfAction::Marked { deleted: true },
            Command::ViewerDbfRecallMarked => DbfAction::Marked { deleted: false },
            _ => DbfAction::Pack,
        };
        let Some((_, dbf)) = self.dbf_table() else {
            return;
        };
        // What cannot be done at all is said before anybody is asked about a backup.
        if matches!(action, DbfAction::Edit | DbfAction::Toggle) {
            let here = self.mode.viewer_mut().and_then(|v| v.dbf_here());
            let Some(here) = here else {
                let m = self.i18n.t("dbf-no-record");
                self.set_error(m);
                return;
            };
            if action == DbfAction::Edit {
                if let Err(why @ Refused::ReadOnly { .. }) =
                    dbf_write::encode(&dbf, here.field.unwrap_or(0), "")
                {
                    let m = self.dbf_refused(&why);
                    self.set_error(m);
                    return;
                }
            }
        }
        if matches!(action, DbfAction::Marked { .. }) && self.dbf_marked().is_empty() {
            let m = self.i18n.t("table-none-marked");
            self.set_error(m);
            return;
        }
        self.dbf_proceed(action);
    }

    /// The marked records the view is showing, in its order; none when none is marked.
    fn dbf_marked(&mut self) -> Vec<usize> {
        match self.mode.viewer_mut() {
            Some(viewer) if viewer.table_marked_count() > 0 => {
                viewer.table_chosen().unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    /// The questions a table is owed before it is written, in their order — the backup,
    /// then its indexes — and, once each has its answer, the thing that was asked.
    fn dbf_proceed(&mut self, action: DbfAction) {
        let Some((table, _)) = self.dbf_table() else {
            return;
        };
        if !self.dbf.backed_up.contains(&table) {
            // The first change of the session to this table: say so, and offer the copy.
            let copy = name_of(&dbf_write::backup_name(&table, self.dbf_now()));
            let body = self.i18n.t_args(
                "dbf-backup-body",
                &[("name", name_of(&table).into()), ("backup", copy.into())],
            );
            self.dbf_park();
            self.mode = Mode::Confirm(Confirm {
                title: self.i18n.t("dbf-title"),
                lines: lines_of(&body),
                action: ConfirmAction::Dbf(DbfStep::Backup {
                    table,
                    then: action,
                }),
                button: 0,
            });
            return;
        }
        // One look at the folder, which is all this does on its own.
        let indexes = dbf_write::index_files(&table);
        let agreed = self.dbf.agreed.get(&table);
        if indexes
            .iter()
            .any(|index| !agreed.is_some_and(|agreed| agreed.contains(index)))
        {
            self.dbf_ask_indexes(table, indexes, Some(action));
            return;
        }
        self.dbf_go(action);
    }

    /// The question about a table's indexes: which files, that this program leaves them as
    /// they are, and that they have to be rebuilt by the program they belong to.
    fn dbf_ask_indexes(&mut self, table: PathBuf, files: Vec<PathBuf>, then: Option<DbfAction>) {
        let mut names: Vec<String> = files.iter().take(6).map(|f| name_of(f)).collect();
        if files.len() > names.len() {
            names.push("…".into());
        }
        let body = self.i18n.t_args(
            "dbf-indexes",
            &[
                ("name", name_of(&table).into()),
                ("files", names.join(", ").into()),
            ],
        );
        self.dbf_park();
        self.mode = Mode::Confirm(Confirm {
            title: self.i18n.t("dbf-title"),
            lines: lines_of(&body),
            action: ConfirmAction::Dbf(DbfStep::Indexes { table, files, then }),
            button: 0,
        });
    }

    /// The answer to one of the questions was yes.
    pub(super) fn dbf_confirmed(&mut self, step: DbfStep) {
        self.restore_viewer();
        let (table, work) = match step {
            DbfStep::Backup { table, then } => (table, DbfWork::Backup { then }),
            DbfStep::Pack { table } => (table, DbfWork::Pack),
            DbfStep::Marked {
                table,
                records,
                deleted,
            } => (table, DbfWork::DeletedMany { records, deleted }),
            DbfStep::Indexes { table, files, then } => {
                // The folder as it is now, not as it was when the question was drawn: an
                // index that arrived while it stood there has not been agreed to.
                let now = dbf_write::index_files(&table);
                if now.iter().any(|index| !files.contains(index)) {
                    self.dbf_ask_indexes(table, now, then);
                    return;
                }
                self.dbf.agreed.insert(table, files);
                if let Some(then) = then {
                    self.dbf_proceed(then);
                }
                return;
            }
        };
        let Some(shown) = self.dbf_viewer(&table).and_then(|v| v.dbf().cloned()) else {
            return;
        };
        if work == DbfWork::Pack {
            // The new table is renamed over the one the viewer holds open, and not every
            // system replaces a file somebody is reading: the viewer lets go until then.
            if let Some(viewer) = self.dbf_viewer(&table) {
                viewer.dbf_release();
            }
        }
        self.dbf_ask(table, shown, work);
    }

    fn dbf_ask(&mut self, table: PathBuf, shown: Dbf, work: DbfWork) {
        let at = self.dbf_now();
        let agreed = self.dbf.agreed.get(&table).cloned().unwrap_or_default();
        let memo = memo_file(&table);
        self.requests.push(Request::Dbf(Box::new(DbfAsk {
            table,
            shown,
            at,
            agreed,
            memo,
            work,
        })));
    }

    // ── what only reads ────────────────────────────────────────────────────────────────

    /// `m`, and `Enter` on a memo cell: the memo whole, in place of the table.
    fn dbf_memo(&mut self) {
        if !self.dbf_shown() {
            return;
        }
        let id = match self.mode.viewer_mut().and_then(|v| v.dbf_memo()) {
            Some(MemoShown::Shown) => return,
            Some(MemoShown::Nothing) => "dbf-memo-none",
            Some(MemoShown::NotMemo) => "dbf-not-memo",
            None => "dbf-no-record",
        };
        let m = self.i18n.t(id);
        self.set_error(m);
    }

    /// `Ctrl+F6`: what every numeric column adds up to, over the marked rows or the rows
    /// the view shows.
    fn dbf_sums(&mut self) {
        if !self.dbf_shown() {
            return;
        }
        let (sums, rows) = match self.mode.viewer_mut().and_then(|v| v.dbf_sums()) {
            Some(Ok(found)) => found,
            _ => {
                let m = self.i18n.t("table-too-many");
                self.set_error(m);
                return;
            }
        };
        if sums.is_empty() {
            let m = self.i18n.t("dbf-sums-none");
            self.set_error(m);
            return;
        }
        // The names to the left, the sums down their last digit, and in brackets how many
        // of the rows had a number there to add.
        let wide =
            |of: fn(&fcmd_view::dbf::ColumnSum) -> usize| sums.iter().map(of).max().unwrap_or(0);
        let (names, totals) = (wide(|s| s.name.chars().count()), wide(|s| s.sum.len()));
        let lines = sums
            .iter()
            .map(|s| format!("{:<names$}  {:>totals$}  ({})", s.name, s.sum, s.count))
            .collect();
        let title = self
            .i18n
            .t_args("dbf-sums-title", &[("rows", (rows as i64).into())]);
        self.dbf_park();
        self.show_message(title, lines);
    }

    /// `Ctrl+T`, and `F8` or `Shift+F8` in a dBase table: the next code page for a table
    /// whose header names none. The choice is the session's for that table, and from then
    /// on what is typed into it is written in that page.
    fn dbf_codepage(&mut self) {
        if !self.dbf_shown() {
            return;
        }
        let Some(viewer) = self.mode.viewer_mut() else {
            return;
        };
        if let Some(named) = viewer
            .dbf()
            .filter(|dbf| dbf.names_its_page())
            .map(|dbf| dbf.page_name())
        {
            let m = self
                .i18n
                .t_args("dbf-codepage-named", &[("page", named.into())]);
            self.set_error(m);
            return;
        }
        let next = Page::after(viewer.dbf_page());
        viewer.dbf_set_page(next);
        let local = viewer.path.is_local() && !viewer.path.in_container();
        let path = viewer.path.as_path().to_path_buf();
        if local {
            match next {
                Some(page) => self.dbf.pages.insert(path, page),
                None => self.dbf.pages.remove(&path),
            };
        }
        let m = match next {
            Some(page) => self
                .i18n
                .t_args("dbf-codepage", &[("page", page.name().into())]),
            None => self.i18n.t("dbf-codepage-header"),
        };
        self.set_status(m);
    }

    /// The page a table was said to be in earlier in the session, for the viewer that is
    /// about to open it.
    pub(super) fn dbf_page_for(&self, table: &Path) -> Option<Page> {
        self.dbf.pages.get(table).copied()
    }

    /// `Ctrl+F9`: ask where the marked rows, or the rows the view shows, are to be written.
    fn dbf_export(&mut self) {
        let Some((table, _)) = self.dbf_table() else {
            return;
        };
        let rows = self
            .mode
            .viewer_mut()
            .and_then(|v| v.table_chosen())
            .map_or(0, |records| records.len());
        if rows == 0 {
            let m = self.i18n.t("dbf-no-record");
            self.set_error(m);
            return;
        }
        let title = self
            .i18n
            .t_args("dbf-prompt-export", &[("rows", (rows as i64).into())]);
        // A name that is not the table's own, with the extension that says which of the
        // two formats is wanted.
        let stem = table
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.mode_prompt_over_viewer(PromptKind::DbfExport, title, format!("{stem}-export.csv"));
    }

    /// `Enter` in the question that asks for an export's name: beside the table unless the
    /// name says where, a table when the name says table, and comma-separated text
    /// otherwise.
    pub(super) fn dbf_export_named(&mut self, name: String) {
        self.restore_viewer();
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let Some((table, dbf)) = self.dbf_table() else {
            return;
        };
        let records = self
            .mode
            .viewer_mut()
            .and_then(|v| v.table_chosen())
            .unwrap_or_default();
        let target = match table.parent() {
            Some(folder) => folder.join(name),
            None => PathBuf::from(name),
        };
        // Said at once where it can be, and found again by the worker where it cannot: a
        // file that is there is never written over.
        if target.exists() {
            let m = self
                .i18n
                .t_args("dbf-export-exists", &[("name", name_of(&target).into())]);
            self.set_error(m);
            return;
        }
        let csv = !fcmd_view::table::dbase_name(name);
        self.dbf_ask(
            table,
            dbf,
            DbfWork::Export {
                records,
                target,
                csv,
            },
        );
    }

    /// Do what was asked, now that the table has its backup.
    fn dbf_go(&mut self, action: DbfAction) {
        let Some((table, dbf)) = self.dbf_table() else {
            return;
        };
        let here = self.mode.viewer_mut().and_then(|v| v.dbf_here());
        match action {
            DbfAction::Append => self.dbf_ask(table, dbf, DbfWork::Append),
            DbfAction::Pack => self.dbf_ask(table, dbf, DbfWork::Count),
            DbfAction::Marked { deleted } => {
                // One question for all of them, which says how many they are.
                let records = self.dbf_marked();
                if records.is_empty() {
                    return;
                }
                let body = self.i18n.t_args(
                    if deleted {
                        "dbf-marked-delete-body"
                    } else {
                        "dbf-marked-recall-body"
                    },
                    &[
                        ("name", name_of(&table).into()),
                        ("count", (records.len() as i64).into()),
                    ],
                );
                self.dbf_park();
                self.mode = Mode::Confirm(Confirm {
                    title: self.i18n.t("dbf-title"),
                    lines: lines_of(&body),
                    action: ConfirmAction::Dbf(DbfStep::Marked {
                        table,
                        records,
                        deleted,
                    }),
                    button: 0,
                });
            }
            DbfAction::Toggle => {
                if let Some(here) = here {
                    let work = DbfWork::Deleted {
                        record: here.record,
                        deleted: !here.deleted,
                    };
                    self.dbf_ask(table, dbf, work);
                }
            }
            DbfAction::Edit => {
                let Some(here) = here else {
                    return;
                };
                // On the column of the deletion mark there is no field under the view, and
                // the first one is the one meant: the question names it either way.
                let field = here.field.unwrap_or(0);
                let Some(info) = dbf.field(field) else {
                    return;
                };
                let text = here.cells.get(field).cloned().unwrap_or_default();
                let title = self.i18n.t_args(
                    "dbf-prompt-cell",
                    &[
                        ("field", info.name.into()),
                        (
                            "type",
                            type_label(info.kind, info.len, info.decimals).into(),
                        ),
                        ("record", ((here.record + 1) as i64).into()),
                    ],
                );
                self.mode_prompt_over_viewer(
                    PromptKind::DbfCell {
                        record: here.record,
                        field,
                    },
                    title,
                    text,
                );
            }
        }
    }

    /// `Enter` in the question that asks for a cell's new value.
    pub(super) fn dbf_cell_typed(&mut self, record: usize, field: usize, text: String) {
        self.restore_viewer();
        let Some((table, dbf)) = self.dbf_table() else {
            return;
        };
        // Refused here, before any file is opened: what was typed is wrong in itself.
        if let Err(why) = dbf_write::encode(&dbf, field, &text) {
            let m = self.dbf_refused(&why);
            self.set_error(m);
            return;
        }
        self.dbf_ask(
            table,
            dbf,
            DbfWork::Cell {
                record,
                field,
                text,
            },
        );
    }

    /// The worker has answered.
    pub(super) fn dbf_done(&mut self, table: PathBuf, result: Result<DbfDone, DbfFailed>) {
        let reopen = |app: &mut App, record: Option<usize>| {
            if app.dbf_viewer(&table).is_none() {
                return;
            }
            match FileSource::open(&table) {
                Ok(source) => {
                    if let Some(viewer) = app.dbf_viewer(&table) {
                        viewer.dbf_reload(Box::new(source), record);
                    }
                }
                Err(e) => app.set_error(format!("{}: {e}", table.display())),
            }
        };
        let done = match result {
            Ok(done) => done,
            Err(failed) => {
                let m = match failed {
                    DbfFailed::Backup(e) => self
                        .i18n
                        .t_args("dbf-backup-failed", &[("error", e.to_string().into())]),
                    // An index that was not there when the question was asked: nothing was
                    // written, and the question is asked again with it named. What had been
                    // asked for is asked for again by whoever asked.
                    DbfFailed::Write(WriteError::Indexes(files))
                    | DbfFailed::Pack(WriteError::Indexes(files)) => {
                        reopen(self, None);
                        if matches!(self.mode, Mode::Viewer(_)) && self.dbf_viewer(&table).is_some()
                        {
                            self.dbf_ask_indexes(table, files, None);
                        }
                        return;
                    }
                    DbfFailed::Write(WriteError::Exists(file)) => self
                        .i18n
                        .t_args("dbf-export-exists", &[("name", name_of(&file).into())]),
                    DbfFailed::Write(e) => self.dbf_failure(&table, e),
                    DbfFailed::Pack(e) => {
                        reopen(self, None);
                        self.dbf_failure(&table, e)
                    }
                };
                self.set_error(m);
                return;
            }
        };
        // Records are counted from one, as dBase counts them.
        let number = |n: usize| ((n + 1) as i64).into();
        match done {
            DbfDone::BackedUp { files, then } => {
                self.dbf.backed_up.insert(table.clone());
                let copy = files.first().map(|f| name_of(f)).unwrap_or_default();
                let m = self.i18n.t_args("dbf-backed-up", &[("name", copy.into())]);
                self.set_status(m);
                // Only if the table is still what is in front: somebody who left the viewer
                // while the copy was made did not ask for a question to open over the panels.
                if matches!(self.mode, Mode::Viewer(_)) && self.dbf_viewer(&table).is_some() {
                    self.dbf_proceed(then);
                }
            }
            DbfDone::DeletedMany { changed, deleted } => {
                let id = if deleted {
                    "dbf-marked-deleted"
                } else {
                    "dbf-marked-recalled"
                };
                let m = self.i18n.t_args(id, &[("count", (changed as i64).into())]);
                self.set_status(m);
            }
            DbfDone::Exported { target, done } => {
                let rows = done.rows as i64;
                let name = name_of(&target);
                let m = match &done.memo {
                    Some(memo) => self.i18n.t_args(
                        "dbf-exported-memo",
                        &[
                            ("rows", rows.into()),
                            ("name", name.into()),
                            ("memo", name_of(memo).into()),
                        ],
                    ),
                    None => self.i18n.t_args(
                        "dbf-exported",
                        &[("rows", rows.into()), ("name", name.into())],
                    ),
                };
                self.set_status(m);
            }
            DbfDone::Cell { record, field } => {
                let name = self
                    .dbf_viewer(&table)
                    .and_then(|v| v.dbf().and_then(|d| d.field(field)))
                    .map(|f| f.name)
                    .unwrap_or_default();
                let m = self.i18n.t_args(
                    "dbf-written",
                    &[("record", number(record)), ("field", name.into())],
                );
                self.set_status(m);
            }
            DbfDone::Deleted { record, deleted } => {
                let id = if deleted {
                    "dbf-deleted"
                } else {
                    "dbf-recalled"
                };
                let m = self.i18n.t_args(id, &[("record", number(record))]);
                self.set_status(m);
            }
            DbfDone::Appended { record } => {
                reopen(self, Some(record));
                let m = self
                    .i18n
                    .t_args("dbf-appended", &[("record", number(record))]);
                self.set_status(m);
            }
            DbfDone::Counted { deleted: 0 } => {
                let m = self.i18n.t("dbf-pack-nothing");
                self.set_status(m);
            }
            DbfDone::Counted { deleted } => {
                if !matches!(self.mode, Mode::Viewer(_)) || self.dbf_viewer(&table).is_none() {
                    return;
                }
                let body = self.i18n.t_args(
                    "dbf-pack-body",
                    &[
                        ("name", name_of(&table).into()),
                        ("count", (deleted as i64).into()),
                    ],
                );
                self.dbf_park();
                self.mode = Mode::Confirm(Confirm {
                    title: self.i18n.t("dbf-title"),
                    lines: lines_of(&body),
                    action: ConfirmAction::Dbf(DbfStep::Pack { table }),
                    button: 0,
                });
            }
            DbfDone::Packed(packed) => {
                reopen(self, None);
                let m = self.i18n.t_args(
                    "dbf-packed",
                    &[
                        ("removed", (packed.removed as i64).into()),
                        ("left", (packed.left as i64).into()),
                    ],
                );
                self.set_status(m);
            }
        }
    }

    /// Why a value does not go in its field, in the interface's language.
    fn dbf_refused(&self, why: &Refused) -> String {
        match why {
            Refused::ReadOnly { kind } => self
                .i18n
                .t_args("dbf-read-only-type", &[("type", kind.to_string().into())]),
            Refused::TooLong { bytes, room } => self.i18n.t_args(
                "dbf-too-long",
                &[
                    ("bytes", (*bytes as i64).into()),
                    ("room", (*room as i64).into()),
                ],
            ),
            Refused::NoByte { ch, exact } => self.i18n.t_args(
                if *exact {
                    "dbf-no-byte"
                } else {
                    "dbf-no-code-page"
                },
                &[("char", ch.to_string().into())],
            ),
            Refused::Number => self.i18n.t("dbf-not-number"),
            Refused::NumberFit { width, decimals } => self.i18n.t_args(
                "dbf-number-fit",
                &[
                    ("width", (*width as i64).into()),
                    ("decimals", (*decimals as i64).into()),
                ],
            ),
            Refused::Date => self.i18n.t("dbf-not-date"),
            Refused::Logical => self.i18n.t("dbf-not-logical"),
        }
    }

    /// Why a table was not written.
    fn dbf_failure(&self, table: &Path, e: WriteError) -> String {
        let name = name_of(table);
        match e {
            WriteError::Open(e) => self.i18n.t_args(
                "dbf-cannot-open",
                &[("name", name.into()), ("error", e.to_string().into())],
            ),
            WriteError::Io(e) => format!("{name}: {e}"),
            WriteError::Cell(why) => self.dbf_refused(&why),
            WriteError::Numbered => self.i18n.t("dbf-numbered"),
            WriteError::Leftover(file) => self
                .i18n
                .t_args("dbf-leftover", &[("name", name_of(&file).into())]),
            // Each has its own answer where it is found; here only for completeness.
            WriteError::Indexes(_)
            | WriteError::Exists(_)
            | WriteError::NotATable
            | WriteError::Changed
            | WriteError::Length
            | WriteError::NoRecord => self.i18n.t("dbf-changed"),
        }
    }
}
