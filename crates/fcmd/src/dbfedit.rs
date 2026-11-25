//! Writing a dBase table from the viewer's table mode: the dialogs in front of
//! `fcmd_view::dbf_write` (SPEC 05 §1.11).
//!
//! The rules are the author's, of 25 November 2026, and they are about tables that Clipper
//! and Harbour programs still open every day:
//!
//! * before the first change to a table in a session a dialog says the table is about to be
//!   altered, and makes a dated copy of it and of its memo file — or the change is not made;
//! * a table with index files beside it is not written at all, and the message says why;
//! * `Pack` says how many records it removes before it removes them.
//!
//! Nothing here touches the disk but one look at the folder for index files: every write,
//! the backup and the count for `Pack` are a [`Request::Dbf`] done on a worker, which opens
//! the table for that write alone and answers with an [`Event::Dbf`].
//!
//! Left out of the compact build with the reader (`--no-default-features`), where the four
//! commands say that dBase tables are not in it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use fcmd_view::dbf::Dbf;
use fcmd_view::dbf_write::{self, Packed, Refused, Stamp, WriteError};

use super::*;

/// What the session remembers about the tables it has written.
#[derive(Debug, Default)]
pub(super) struct DbfSession {
    /// The tables whose backup was made: the dialog is asked once for each, and the changes
    /// after the first go straight through.
    backed_up: HashSet<PathBuf>,
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
}

/// What a confirmation about a table does on `Enter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbfStep {
    /// Make the backup, and then do what was asked.
    Backup { table: PathBuf, then: DbfAction },
    /// Take the deleted records out.
    Pack { table: PathBuf },
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
    Append,
    /// Count the deleted records, which is what `Pack` asks about before it runs.
    Count,
    Pack,
}

/// The work, the table it is for, the header the viewer has of it — a write is refused when
/// the file no longer has those fields — and the moment it is asked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbfAsk {
    pub table: PathBuf,
    pub shown: Dbf,
    pub at: Stamp,
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
    Appended {
        record: usize,
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
    match &ask.work {
        DbfWork::Backup { then } => {
            // The refusal comes before the copy: a backup of a table that is not going to
            // be written is a file nobody asked for.
            let indexes = dbf_write::index_files(table);
            if !indexes.is_empty() {
                return Err(write(WriteError::Indexes(indexes)));
            }
            dbf_write::backup(table, at)
                .map(|files| DbfDone::BackedUp { files, then: *then })
                .map_err(DbfFailed::Backup)
        }
        DbfWork::Cell {
            record,
            field,
            text,
        } => dbf_write::set_cell(table, shown, *record, *field, text, at)
            .map(|()| DbfDone::Cell {
                record: *record,
                field: *field,
            })
            .map_err(write),
        DbfWork::Deleted { record, deleted } => {
            dbf_write::set_deleted(table, shown, *record, *deleted, at)
                .map(|()| DbfDone::Deleted {
                    record: *record,
                    deleted: *deleted,
                })
                .map_err(write)
        }
        DbfWork::Append => dbf_write::append(table, shown, at)
            .map(|record| DbfDone::Appended { record })
            .map_err(write),
        DbfWork::Count => dbf_write::count_deleted(table, shown)
            .map(|deleted| DbfDone::Counted { deleted })
            .map_err(write),
        DbfWork::Pack => dbf_write::pack(table, shown, at)
            .map(DbfDone::Packed)
            .map_err(DbfFailed::Pack),
    }
}

/// The memo file beside a dBase table, if the table has one and it is there.
///
/// dBase writes `.dbt`, FoxPro writes `.fpt`, and either may be spelled in capitals beside a
/// table spelled in capitals — a DOS habit that outlived DOS. Nothing else is guessed at: a
/// file that is not a `.dbf` has no memo file, and a name that is there is used as it is.
pub(super) fn memo_file(table: &std::path::Path) -> Option<std::path::PathBuf> {
    let name = table.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = name.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case("dbf") {
        return None;
    }
    let upper = ext.chars().all(|c| c.is_ascii_uppercase());
    for memo in ["dbt", "fpt"] {
        let memo = if upper {
            memo.to_uppercase()
        } else {
            memo.into()
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

    /// `Ctrl+F4`, `Ctrl+N`, `Ctrl+F8`, `Shift+Del` in table mode.
    pub(super) fn dbf_command(&mut self, cmd: Command) {
        let action = match cmd {
            Command::ViewerDbfEdit => DbfAction::Edit,
            Command::ViewerDbfAppend => DbfAction::Append,
            Command::ViewerDbfDelete => DbfAction::Toggle,
            _ => DbfAction::Pack,
        };
        let Some((table, dbf)) = self.dbf_table() else {
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
        // One look at the folder, which is all this does on its own: an index beside the
        // table is a refusal, and it is better made before a backup than after one.
        let indexes = dbf_write::index_files(&table);
        if !indexes.is_empty() {
            self.dbf_refuse_indexes(&table, &indexes);
            return;
        }
        if self.dbf.backed_up.contains(&table) {
            self.dbf_go(action);
            return;
        }
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
    }

    /// The answer to one of the two questions was yes.
    pub(super) fn dbf_confirmed(&mut self, step: DbfStep) {
        self.restore_viewer();
        let (table, work) = match step {
            DbfStep::Backup { table, then } => (table, DbfWork::Backup { then }),
            DbfStep::Pack { table } => (table, DbfWork::Pack),
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
        self.requests.push(Request::Dbf(Box::new(DbfAsk {
            table,
            shown,
            at,
            work,
        })));
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
                    DbfFailed::Write(WriteError::Indexes(files))
                    | DbfFailed::Pack(WriteError::Indexes(files)) => {
                        reopen(self, None);
                        self.dbf_refuse_indexes(&table, &files);
                        return;
                    }
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
                    self.dbf_go(then);
                }
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

    /// The refusal that has the most to explain, in a box of its own: which files, and why
    /// they are a reason.
    fn dbf_refuse_indexes(&mut self, table: &Path, indexes: &[PathBuf]) {
        let mut files: Vec<String> = indexes.iter().take(4).map(|f| name_of(f)).collect();
        if indexes.len() > files.len() {
            files.push("…".into());
        }
        let body = self.i18n.t_args(
            "dbf-indexes",
            &[
                ("name", name_of(table).into()),
                ("files", files.join(", ").into()),
            ],
        );
        self.dbf_park();
        self.show_message(self.i18n.t("dbf-title"), lines_of(&body));
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
            // Shown where they are found, with the files named; here only for completeness.
            WriteError::Indexes(_)
            | WriteError::NotATable
            | WriteError::Changed
            | WriteError::Length
            | WriteError::NoRecord => self.i18n.t("dbf-changed"),
        }
    }
}
