//! The Windows registry in a panel (SPEC 06 §17): `Alt+F1` → *Registry*.
//!
//! The roots are folders, the keys are folders, the values are rows: `Enter` goes into a
//! key and `..` comes out of it, `F3` shows a value whole, `F4` edits it, `Shift+F4` makes
//! one, `F7` makes a key, `F8` deletes and `F6` renames what is under the cursor, and `F5`
//! writes keys to a `.reg` file. `Enter` on a `.reg` file in an ordinary panel offers to
//! write it into the registry.
//!
//! **A list panel, re-listed at every step, and not a provider.** A provider would hand
//! the registry to the job engine — and then `F5`, `F6` and `F8` would copy, move and
//! delete keys the way they do files, with none of what this panel is for: the question
//! that names the key, and the backup. As a list the panel keeps the sort keys, the quick
//! search, the selection and the columns it already has; the key it shows is its title, and
//! each row carries its own name, so nothing about a panel is remembered anywhere but in
//! the panel — a tab, the other side, a swap of the two all keep working without this file
//! knowing about them.
//!
//! **Nothing is changed without a backup.** Before a value is written, made, renamed or
//! deleted the key it is in is exported to a `.reg` file in `registry-backups/` beside the
//! settings; before a key is deleted or renamed, the key and everything under it. The file
//! is named after the key and the moment, and the status line says where it went. A backup
//! that cannot be written is a change that is not made.
//!
//! The registry is read on the spot to list a key — it is memory the system already has,
//! as the process list is — and everything that writes, a file or the registry, is a
//! [`Request::Registry`] done on a worker and answered with an [`Event::Registry`].
//! [`perform`] is that work, and it is given the registry to do it on: the system's when
//! the program runs, one kept in memory when the tests do.

use std::path::{Path, PathBuf};

use fcmd_platform::regfile::{self, DataError, KeyBlock, Line, Value};
use fcmd_platform::registry::{self, RegError, Registry, SystemRegistry, ROOTS};

use super::*;
use crate::form::Field;

/// The row of the change-drive dialog this panel is, and what its panels answer to.
pub(super) const ORIGIN: &str = ":registry";

/// The types a new value is offered in, in regedit's order.
const NEW_TYPES: [u32; 6] = [
    regfile::REG_SZ,
    regfile::REG_EXPAND_SZ,
    regfile::REG_MULTI_SZ,
    regfile::REG_DWORD,
    regfile::REG_QWORD,
    regfile::REG_BINARY,
];

/// How many empty rows the dialog for a list of strings ends with, for strings to add.
const SPARE_ROWS: usize = 4;

/// What the session keeps about the registry.
pub(super) struct RegSession {
    /// The registry the panel lists from: the system's, or a test's.
    pub(super) registry: Box<dyn Registry>,
    /// Whether there is a registry here at all — the row in the drive menu, and `Enter` on
    /// a `.reg` file, exist only where there is.
    pub(super) available: bool,
    /// The key the panel was last on, which is where it opens the next time.
    last: String,
    /// What the dialog in front is about.
    pending: Option<RegPending>,
    /// The moment a test wants every backup to carry, in place of the clock.
    #[cfg(test)]
    pub(super) clock: Option<String>,
}

impl Default for RegSession {
    fn default() -> Self {
        RegSession {
            registry: Box::new(SystemRegistry),
            available: SystemRegistry::AVAILABLE,
            last: String::new(),
            pending: None,
            #[cfg(test)]
            clock: None,
        }
    }
}

/// What a dialog was opened for, kept while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RegPending {
    Edit {
        key: String,
        name: String,
        kind: u32,
    },
    NewValue {
        key: String,
    },
    NewKey {
        key: String,
    },
    RenameValue {
        key: String,
        name: String,
    },
    RenameKey {
        key: String,
    },
    Export {
        keys: Vec<String>,
    },
}

/// The row under the cursor of a registry panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RegRow {
    Up,
    Key(String),
    Value(String),
}

/// One piece of work on the registry, for the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegWork {
    /// Write one value. `new` is a value that was just made, which the editor opens on
    /// once it exists.
    SetValue {
        key: String,
        value: Value,
        new: bool,
    },
    DeleteValue {
        key: String,
        name: String,
    },
    RenameValue {
        key: String,
        old: String,
        new: String,
    },
    /// Make the key `name` under `key`.
    CreateKey {
        key: String,
        name: String,
    },
    /// Remove the key and everything under it.
    DeleteKey {
        key: String,
    },
    RenameKey {
        key: String,
        name: String,
    },
    /// Write these keys, and everything under them, to a `.reg` file.
    Export {
        keys: Vec<String>,
        file: PathBuf,
    },
    /// Read a `.reg` file and say what it would write, for the question to ask.
    Inspect {
        file: PathBuf,
    },
    /// Write a `.reg` file into the registry.
    Import {
        file: PathBuf,
    },
}

/// The work, the panel it was asked from, where the backup goes and the moment it is asked
/// at (`YYYYMMDD-hhmmss`, local time), which is in the backup's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegAsk {
    pub side: usize,
    pub backups: PathBuf,
    pub stamp: String,
    pub work: RegWork,
}

/// What the work came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegOk {
    /// The registry was changed, and this is the backup made first.
    Changed {
        backup: PathBuf,
    },
    Exported {
        keys: usize,
    },
    /// What a `.reg` file holds: the keys it names, the values it writes, and the keys and
    /// values it takes away.
    Inspected {
        keys: usize,
        values: usize,
        deletes: usize,
    },
    Imported {
        backup: PathBuf,
    },
}

/// Why it was not done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegFailed {
    /// The backup could not be made, so nothing was changed.
    Backup(String),
    /// The registry refused, for this key.
    Registry { key: String, error: RegError },
    /// The name is taken.
    Exists(String),
    /// A file could not be read or written.
    File(String),
    /// The file is not a `.reg` file, or not one that can be read.
    NotARegFile(String),
}

/// The answer to a [`RegAsk`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegDone {
    pub ask: RegAsk,
    pub result: Result<RegOk, RegFailed>,
}

// ── the work, off the UI thread ─────────────────────────────────────────────────────────

/// A key's path as a file name: every character that is not a letter, a digit, a dot or a
/// dash becomes `_`, and a name longer than a file system likes is cut.
fn file_stem(label: &str) -> String {
    let mut out: String = label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(100)
        .collect();
    if out.is_empty() {
        out.push_str("registry");
    }
    out
}

/// Export these keys — `true` beside one for everything under it as well — to a new file
/// in the backup folder, named after `label` and the moment. A key that is not there yet
/// has nothing to keep and is left out; any other refusal to read is a refusal to back up.
fn backup(
    reg: &dyn Registry,
    ask: &RegAsk,
    label: &str,
    keys: &[(String, bool)],
) -> Result<PathBuf, RegFailed> {
    let mut blocks: Vec<KeyBlock> = Vec::new();
    for (key, deep) in keys {
        match registry::export(reg, key, *deep) {
            Ok(found) => blocks.extend(found),
            Err(RegError::NotFound) => {}
            Err(error) => {
                return Err(RegFailed::Backup(format!("{key}: {}", plain_error(&error))));
            }
        }
    }
    let failed = |why: std::io::Error| RegFailed::Backup(why.to_string());
    std::fs::create_dir_all(&ask.backups).map_err(failed)?;
    let stem = format!("{}-{}", file_stem(label), ask.stamp);
    // Two changes to one key within a second are two backups, not one written twice.
    let mut file = ask.backups.join(format!("{stem}.reg"));
    let mut again = 2;
    while file.exists() {
        file = ask.backups.join(format!("{stem}-{again}.reg"));
        again += 1;
    }
    std::fs::write(&file, regfile::encode(&regfile::write(&blocks))).map_err(failed)?;
    Ok(file)
}

/// A refusal in a few words, for a message that already names the key.
fn plain_error(error: &RegError) -> String {
    match error {
        RegError::Unsupported => "no registry on this system".into(),
        RegError::Denied => "access denied".into(),
        RegError::NotFound => "not found".into(),
        RegError::Failed(why) => why.clone(),
    }
}

fn read_reg_file(file: &Path) -> Result<Vec<KeyBlock>, RegFailed> {
    let bytes = std::fs::read(file).map_err(|why| RegFailed::File(why.to_string()))?;
    regfile::parse(&regfile::decode(&bytes)).map_err(|why| RegFailed::NotARegFile(why.to_string()))
}

/// Do one piece of work on this registry: the backup first, where the work changes
/// anything, and the change only once the backup is on disk.
pub fn perform(reg: &dyn Registry, ask: &RegAsk) -> Result<RegOk, RegFailed> {
    let refused = |key: &str| {
        let key = key.to_string();
        move |error: RegError| RegFailed::Registry { key, error }
    };
    match &ask.work {
        RegWork::SetValue { key, value, .. } => {
            let backup = backup(reg, ask, key, &[(key.clone(), false)])?;
            reg.set_value(key, value).map_err(refused(key))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::DeleteValue { key, name } => {
            let backup = backup(reg, ask, key, &[(key.clone(), false)])?;
            reg.delete_value(key, name).map_err(refused(key))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::RenameValue { key, old, new } => {
            let values = reg.values(key).map_err(refused(key))?;
            let taken = values.iter().any(|v| registry::same_name(&v.name, new));
            if taken && !registry::same_name(old, new) {
                return Err(RegFailed::Exists(new.clone()));
            }
            let backup = backup(reg, ask, key, &[(key.clone(), false)])?;
            registry::rename_value(reg, key, old, new).map_err(refused(key))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::CreateKey { key, name } => {
            let full = registry::join(key, name);
            if reg.subkeys(&full).is_ok() {
                return Err(RegFailed::Exists(name.clone()));
            }
            let backup = backup(reg, ask, key, &[(key.clone(), false)])?;
            reg.create_key(&full).map_err(refused(&full))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::DeleteKey { key } => {
            let backup = backup(reg, ask, key, &[(key.clone(), true)])?;
            reg.delete_tree(key).map_err(refused(key))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::RenameKey { key, name } => {
            let above = registry::parent(key).unwrap_or_default();
            let target = registry::join(above, name);
            if !registry::same_name(registry::leaf(key), name) && reg.subkeys(&target).is_ok() {
                return Err(RegFailed::Exists(name.clone()));
            }
            let backup = backup(reg, ask, key, &[(key.clone(), true)])?;
            reg.rename_key(key, name).map_err(refused(key))?;
            Ok(RegOk::Changed { backup })
        }
        RegWork::Export { keys, file } => {
            let mut blocks = Vec::new();
            for key in keys {
                blocks.extend(registry::export(reg, key, true).map_err(refused(key))?);
            }
            std::fs::write(file, regfile::encode(&regfile::write(&blocks)))
                .map_err(|why| RegFailed::File(why.to_string()))?;
            Ok(RegOk::Exported { keys: blocks.len() })
        }
        RegWork::Inspect { file } => {
            let blocks = read_reg_file(file)?;
            let mut values = 0;
            let mut deletes = 0;
            for block in &blocks {
                if block.delete {
                    deletes += 1;
                }
                for line in &block.lines {
                    match line {
                        Line::Set(_) => values += 1,
                        Line::Delete(_) => deletes += 1,
                    }
                }
            }
            Ok(RegOk::Inspected {
                keys: blocks.len(),
                values,
                deletes,
            })
        }
        RegWork::Import { file } => {
            let blocks = read_reg_file(file)?;
            // Every key the file touches, once: its values for a key it writes into, the
            // whole subtree for a key it removes.
            let mut touched: Vec<(String, bool)> = Vec::new();
            for block in &blocks {
                match touched
                    .iter_mut()
                    .find(|(key, _)| registry::same_name(key, &block.path))
                {
                    Some((_, deep)) => *deep |= block.delete,
                    None => touched.push((block.path.clone(), block.delete)),
                }
            }
            let label = format!(
                "import-{}",
                file.file_stem().unwrap_or_default().to_string_lossy()
            );
            let backup = backup(reg, ask, &label, &touched)?;
            registry::apply(reg, &blocks).map_err(|error| RegFailed::Registry {
                key: file.display().to_string(),
                error,
            })?;
            Ok(RegOk::Imported { backup })
        }
    }
}

// ── the rows ────────────────────────────────────────────────────────────────────────────

/// A row's name as the path the list keeps for it: `k` or `v` and the name's bytes in hex,
/// which is a file name on every system whatever the name has in it — a value may be
/// called `a/b\c`, or nothing at all.
fn row_path(kind: char, name: &str) -> VPath {
    let hex: String = name.bytes().map(|b| format!("{b:02x}")).collect();
    VPath::local(format!("{kind}{hex}"))
}

pub(super) fn row_of(path: &VPath) -> Option<RegRow> {
    let text = path.display_name();
    let mut chars = text.chars();
    let kind = chars.next()?;
    let hex = chars.as_str();
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok())
        .collect::<Option<_>>()?;
    let name = String::from_utf8(bytes).ok()?;
    match kind {
        'k' => Some(RegRow::Key(name)),
        'v' => Some(RegRow::Value(name)),
        _ => None,
    }
}

/// One line of a panel row: no line ends, no tabs, and not longer than a screen is wide.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(200)
        .collect()
}

impl App {
    /// The moment a change is asked at, as a backup's name has it: `20261127-101500`.
    fn reg_stamp(&self) -> String {
        #[cfg(test)]
        if let Some(fixed) = &self.reg.clock {
            return fixed.clone();
        }
        fcmd_ui::format_stamp(std::time::SystemTime::now())
            .chars()
            .filter_map(|c| match c {
                '-' | ':' => None,
                ' ' => Some('-'),
                c => Some(c),
            })
            .collect()
    }

    /// Where the backups go: `registry-backups/` beside the settings.
    pub(super) fn reg_backup_dir(&self) -> PathBuf {
        self.paths
            .config_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("registry-backups")
    }

    /// The name a value is shown under: its own, or `(Default)` for the one that has none.
    fn reg_value_name(&self, name: &str) -> String {
        if name.is_empty() {
            self.i18n.t("reg-default")
        } else {
            name.to_string()
        }
    }

    /// A refusal of the registry's as a sentence that names the key.
    fn reg_error(&self, key: &str, error: &RegError) -> String {
        let key = key.to_string();
        match error {
            RegError::Unsupported => self.i18n.t("reg-not-here"),
            RegError::Denied => self.i18n.t_args("reg-denied", &[("key", key.into())]),
            RegError::NotFound => self.i18n.t_args("reg-missing", &[("key", key.into())]),
            RegError::Failed(why) => self.i18n.t_args(
                "reg-failed",
                &[("key", key.into()), ("error", why.clone().into())],
            ),
        }
    }

    /// The key a registry panel is on — empty for the list of roots — and `None` for a
    /// panel that is not one. It is the panel's title: a key's name starts with `HKEY_`,
    /// and the title of the roots is the panel's own name.
    pub(super) fn reg_key(&self, side: usize) -> Option<String> {
        if self.panels[side].temp_origin() != Some(ORIGIN) {
            return None;
        }
        let title = &self.panels[side].temp.as_ref()?.title;
        Some(if title.starts_with("HKEY_") {
            title.clone()
        } else {
            String::new()
        })
    }

    /// The row under the cursor of the active panel, where that is a registry panel.
    pub(super) fn reg_row(&self) -> Option<RegRow> {
        let entry = self.panel().current()?;
        if entry.kind == fcmd_core::EntryKind::Parent {
            return Some(RegRow::Up);
        }
        row_of(&self.panel().path_of(entry))
    }

    /// `Alt+F1` → Registry: the panel on `side` becomes the registry, on the key it was
    /// last on.
    pub(super) fn open_registry_panel(&mut self, side: usize) {
        if !self.reg.available {
            let m = self.i18n.t("reg-not-here");
            self.set_error(m);
            return;
        }
        let mut key = self.reg.last.clone();
        // The key one was on may have gone since: the nearest one above it that is there.
        while !key.is_empty() && self.reg.registry.subkeys(&key).is_err() {
            key = registry::parent(&key).unwrap_or_default().to_string();
        }
        if self.reg_show(side, &key, None) {
            self.active = side;
        }
    }

    /// List `key` — or the roots, for an empty one — on the panel on `side`, with the
    /// cursor on `focus` if that row is there. `false`, with the reason said, when the key
    /// cannot be listed; the panel then stays as it was.
    pub(super) fn reg_show(&mut self, side: usize, key: &str, focus: Option<&RegRow>) -> bool {
        use fcmd_core::{Entry, EntryFlags, EntryId, EntryKind};

        let row = |name: String, kind: EntryKind, size: u64| Entry {
            name: name.into(),
            kind,
            size,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(0),
            link_target: None,
        };
        let mut items: Vec<(VPath, Entry)> = Vec::new();
        if key.is_empty() {
            for root in ROOTS {
                items.push((
                    row_path('k', root),
                    row(root.to_string(), EntryKind::Dir, 0),
                ));
            }
        } else {
            let keys = match self.reg.registry.subkeys(key) {
                Ok(keys) => keys,
                Err(error) => {
                    let m = self.reg_error(key, &error);
                    self.set_error(m);
                    return false;
                }
            };
            // A key whose subkeys can be listed and whose values cannot is still worth
            // being in: the keys are shown, and the line says what is missing.
            let values = match self.reg.registry.values(key) {
                Ok(values) => values,
                Err(error) => {
                    let m = self.reg_error(key, &error);
                    self.set_error(m);
                    Vec::new()
                }
            };
            items.push((VPath::local(".."), Entry::parent_entry()));
            for name in keys {
                items.push((row_path('k', &name), row(name, EntryKind::Dir, 0)));
            }
            // The columns of a value's row — name, type, data — as wide as their widest,
            // so the rows line up; the size column the panel has is the data's length.
            let width = |text: &str| unicode_width::UnicodeWidthStr::width(text);
            let shown: Vec<(String, String)> = values
                .iter()
                .map(|v| {
                    (
                        one_line(&self.reg_value_name(&v.name)),
                        regfile::type_name(v.kind),
                    )
                })
                .collect();
            let name_w = shown
                .iter()
                .map(|(n, _)| width(n))
                .max()
                .unwrap_or(0)
                .min(32);
            let type_w = shown.iter().map(|(_, t)| width(t)).max().unwrap_or(0);
            for (value, (name, kind)) in values.iter().zip(&shown) {
                let label = format!(
                    "{}  {}  {}",
                    pad_to(name, name_w),
                    pad_to(kind, type_w),
                    one_line(&value.summary())
                );
                items.push((
                    row_path('v', &value.name),
                    row(
                        label.trim_end().to_string(),
                        EntryKind::File,
                        value.data.len() as u64,
                    ),
                ));
            }
        }
        let title = if key.is_empty() {
            self.plain("drive-registry")
        } else {
            key.to_string()
        };
        let was = self.panels[side].cursor;
        let generation = self.panels[side].generation + 1;
        self.panels[side].generation = generation;
        self.panels[side].set_temp(title, items, generation);
        self.panels[side].set_temp_origin(ORIGIN);
        let at = focus.and_then(|want| {
            let panel = &self.panels[side];
            (0..panel.listing.len()).find(|row| {
                panel.listing.row(*row).is_some_and(|e| match want {
                    RegRow::Up => e.kind == EntryKind::Parent,
                    other => row_of(&panel.path_of(e)).as_ref() == Some(other),
                })
            })
        });
        // No row asked for, or the row has gone: the one the cursor was on, which after a
        // delete is the row that came up into its place.
        self.panels[side].cursor_to(at.unwrap_or(was));
        self.reg.last = key.to_string();
        true
    }

    /// `Ctrl+R`, and after every change: the same key read again, on every panel that
    /// shows the registry. A key that has gone is left for the nearest one above it.
    pub(super) fn reg_reread(&mut self, focus: Option<(usize, RegRow)>) {
        for side in 0..self.panels.len() {
            let Some(mut key) = self.reg_key(side) else {
                continue;
            };
            let mut gone = None;
            while !key.is_empty() && self.reg.registry.subkeys(&key).is_err() {
                gone = Some(RegRow::Key(registry::leaf(&key).to_string()));
                key = registry::parent(&key).unwrap_or_default().to_string();
            }
            let want = match &focus {
                Some((on, row)) if *on == side => Some(row.clone()),
                _ => gone,
            };
            let last = self.reg.last.clone();
            self.reg_show(side, &key, want.as_ref());
            // What the panel opens on next time is where somebody went, not what a
            // refresh of the other side happened to read last.
            if side != self.active {
                self.reg.last = last;
            }
        }
    }

    /// `Enter`: into the key under the cursor, out of this one on `..`, and the editor on
    /// a value — which is what `Enter` does in regedit.
    pub(super) fn reg_enter(&mut self) {
        let Some(key) = self.reg_key(self.active) else {
            return;
        };
        match self.reg_row() {
            Some(RegRow::Up) => self.reg_up(),
            Some(RegRow::Key(name)) => {
                let into = if key.is_empty() {
                    name
                } else {
                    registry::join(&key, &name)
                };
                let side = self.active;
                self.panels[side].cursor_to(0);
                self.reg_show(side, &into, Some(&RegRow::Up));
            }
            Some(RegRow::Value(name)) => self.reg_edit_value(&key, &name),
            None => {}
        }
    }

    /// `..`, `Backspace`, `Ctrl+PgUp`: the key above, with the cursor on the one come out of.
    pub(super) fn reg_up(&mut self) {
        let Some(key) = self.reg_key(self.active) else {
            return;
        };
        if key.is_empty() {
            return;
        }
        let above = registry::parent(&key).unwrap_or_default().to_string();
        let from = RegRow::Key(registry::leaf(&key).to_string());
        self.reg_show(self.active, &above, Some(&from));
    }

    /// `Ctrl+\`: the five roots.
    pub(super) fn reg_roots(&mut self) {
        let from = self
            .reg_key(self.active)
            .and_then(|key| key.split('\\').next().map(str::to_string))
            .map(RegRow::Key);
        self.reg_show(self.active, "", from.as_ref());
    }

    /// One value of a key as the registry has it now, or the reason there is none, as a
    /// sentence.
    fn reg_find(&self, key: &str, name: &str) -> Result<Value, String> {
        let values = self
            .reg
            .registry
            .values(key)
            .map_err(|error| self.reg_error(key, &error))?;
        values
            .into_iter()
            .find(|v| registry::same_name(&v.name, name))
            .ok_or_else(|| {
                let what = format!("{key}\\{}", self.reg_value_name(name));
                self.reg_error(&what, &RegError::NotFound)
            })
    }

    /// The same, with the reason said on the status line.
    fn reg_value(&mut self, key: &str, name: &str) -> Option<Value> {
        match self.reg_find(key, name) {
            Ok(value) => Some(value),
            Err(why) => {
                self.set_error(why);
                None
            }
        }
    }

    /// What `F3` and the quick view show of a row: a value in full — its key, its type and
    /// length, then the text, the strings one a line, the number both ways or the dump —,
    /// and of a key how much it holds.
    pub(super) fn reg_card(&self, side: usize, row: &RegRow) -> Option<(String, Vec<String>)> {
        let key = self.reg_key(side)?;
        match row {
            RegRow::Up => None,
            RegRow::Value(name) => {
                let value = self.reg_find(&key, name).ok()?;
                let mut lines = vec![
                    key,
                    format!("{}  {} B", regfile::type_name(value.kind), value.data.len()),
                    String::new(),
                ];
                lines.extend(value.full());
                Some((self.reg_value_name(name), lines))
            }
            RegRow::Key(name) => {
                let full = if key.is_empty() {
                    name.clone()
                } else {
                    registry::join(&key, name)
                };
                let keys = self.reg.registry.subkeys(&full);
                let values = self.reg.registry.values(&full);
                let count = |n: Option<usize>| n.map_or("?".to_string(), |n| n.to_string());
                let lines = vec![
                    full,
                    format!(
                        "{}  {}",
                        self.i18n.t("reg-card-keys"),
                        count(keys.ok().map(|k| k.len()))
                    ),
                    format!(
                        "{}  {}",
                        self.i18n.t("reg-card-values"),
                        count(values.ok().map(|v| v.len()))
                    ),
                ];
                Some((name.clone(), lines))
            }
        }
    }

    /// `F3`: the row under the cursor, whole.
    pub(super) fn reg_view(&mut self) {
        let Some(row) = self.reg_row() else {
            return;
        };
        // A value that has gone since the key was listed says so, rather than nothing.
        if let (Some(key), RegRow::Value(name)) = (self.reg_key(self.active), &row) {
            if self.reg_value(&key, name).is_none() {
                return;
            }
        }
        if let Some((title, lines)) = self.reg_card(self.active, &row) {
            self.mode = Mode::Message { title, lines };
        }
    }

    fn reg_form(&self, title: &str, fields: Vec<Field>, cursor: usize) -> Form {
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Registry,
            title: self.i18n.t(title),
            fields,
            cursor,
            site: None,
            button: None,
            buttons: vec![self.i18n.t("btn-ok"), self.i18n.t("btn-cancel")],
        }
    }

    fn reg_text(key: &'static str, label: String, value: String) -> Field {
        Field::Text {
            key,
            label,
            caret: value.chars().count(),
            value,
            secret: false,
            all: false,
        }
    }

    /// `F4` on a value: the dialog its type wants — the text of a string, a number in
    /// decimal (hex is typed after `0x`), a row a string for a list with empty rows to add
    /// to, pairs of hex digits for bytes.
    pub(super) fn reg_edit_value(&mut self, key: &str, name: &str) {
        let Some(value) = self.reg_value(key, name) else {
            return;
        };
        let mut fields = vec![
            Field::Note(format!("{key}\\{}", self.reg_value_name(&value.name))),
            Field::Note(regfile::type_name(value.kind)),
        ];
        if value.kind == regfile::REG_MULTI_SZ {
            let mut strings = value.strings();
            strings.extend(std::iter::repeat_n(String::new(), SPARE_ROWS));
            for (i, s) in strings.into_iter().enumerate() {
                fields.push(Self::reg_text("reg.data", format!("{}", i + 1), s));
            }
        } else {
            fields.push(Self::reg_text(
                "reg.data",
                self.i18n.t("reg-field-data"),
                value.edit_text(),
            ));
        }
        self.reg.pending = Some(RegPending::Edit {
            key: key.to_string(),
            name: value.name.clone(),
            kind: value.kind,
        });
        self.mode = Mode::Form(Box::new(self.reg_form("reg-edit-title", fields, 2)));
    }

    /// `F4`: the value under the cursor. A key has nothing to edit but what is in it.
    pub(super) fn reg_edit(&mut self) {
        let key = self.reg_key(self.active);
        match (key, self.reg_row()) {
            (Some(key), Some(RegRow::Value(name))) => self.reg_edit_value(&key, &name),
            _ => {
                let m = self.i18n.t("msg-nothing-to-do");
                self.set_error(m);
            }
        }
    }

    /// The key the panel is on, where that is a key something can be made in or taken out
    /// of — the list of roots is not, and says so.
    fn reg_key_to_change(&mut self) -> Option<String> {
        let key = self.reg_key(self.active)?;
        if key.is_empty() {
            let m = self.i18n.t("reg-root");
            self.set_error(m);
            return None;
        }
        Some(key)
    }

    /// `Shift+F4`: a new value — its name and its type; it is made empty and the editor
    /// opens on it.
    pub(super) fn reg_new_value(&mut self) {
        let Some(key) = self.reg_key_to_change() else {
            return;
        };
        let options: Vec<(String, String)> = NEW_TYPES
            .iter()
            .map(|kind| (kind.to_string(), regfile::type_name(*kind)))
            .collect();
        let fields = vec![
            Field::Note(key.clone()),
            Self::reg_text("reg.name", self.i18n.t("reg-field-name"), String::new()),
            Field::Choice {
                key: "reg.type",
                label: self.i18n.t("reg-field-type"),
                options,
                chosen: 0,
            },
        ];
        self.reg.pending = Some(RegPending::NewValue { key });
        self.mode = Mode::Form(Box::new(self.reg_form("reg-new-value-title", fields, 1)));
    }

    /// `F7`: a new key under the one the panel is on.
    pub(super) fn reg_new_key(&mut self) {
        let Some(key) = self.reg_key_to_change() else {
            return;
        };
        let fields = vec![
            Field::Note(key.clone()),
            Self::reg_text("reg.name", self.i18n.t("reg-field-name"), String::new()),
        ];
        self.reg.pending = Some(RegPending::NewKey { key });
        self.mode = Mode::Form(Box::new(self.reg_form("reg-new-key-title", fields, 1)));
    }

    /// `F6`, `Shift+F6`: another name for the value or the key under the cursor.
    pub(super) fn reg_rename(&mut self) {
        let Some(key) = self.reg_key_to_change() else {
            return;
        };
        let (pending, what, name) = match self.reg_row() {
            Some(RegRow::Value(name)) => (
                RegPending::RenameValue {
                    key: key.clone(),
                    name: name.clone(),
                },
                format!("{key}\\{}", self.reg_value_name(&name)),
                name,
            ),
            Some(RegRow::Key(name)) => {
                let full = registry::join(&key, &name);
                (RegPending::RenameKey { key: full.clone() }, full, name)
            }
            _ => {
                let m = self.i18n.t("msg-nothing-to-do");
                self.set_error(m);
                return;
            }
        };
        let fields = vec![
            Field::Note(what),
            Self::reg_text("reg.name", self.i18n.t("reg-field-name"), name),
        ];
        self.reg.pending = Some(pending);
        self.mode = Mode::Form(Box::new(self.reg_form("reg-rename-title", fields, 1)));
    }

    /// `F5`: the selected keys — or the one under the cursor, or the one the panel is on
    /// when the cursor is on a value — written to a `.reg` file, which the dialog names.
    pub(super) fn reg_export(&mut self) {
        let Some(key) = self.reg_key(self.active) else {
            return;
        };
        let under = |name: &str| {
            if key.is_empty() {
                name.to_string()
            } else {
                registry::join(&key, name)
            }
        };
        let panel = self.panel();
        let mut keys: Vec<String> = (0..panel.listing.len())
            .filter_map(|row| panel.listing.row(row))
            .filter(|e| panel.selection.contains(e.id))
            .filter_map(|e| match row_of(&panel.path_of(e)) {
                Some(RegRow::Key(name)) => Some(under(&name)),
                _ => None,
            })
            .collect();
        if keys.is_empty() {
            match self.reg_row() {
                Some(RegRow::Key(name)) => keys.push(under(&name)),
                _ if !key.is_empty() => keys.push(key.clone()),
                _ => {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
            }
        }
        // Into the folder the other panel shows, where that is a folder on this machine;
        // otherwise a bare name, which lands where this panel was before it was the
        // registry.
        let name = format!("{}.reg", file_stem(registry::leaf(&keys[0])));
        let other = self.other();
        let target = if other.temp.is_none() && other.cwd.is_local() && !other.cwd.in_container() {
            other.cwd.join(&name).to_string()
        } else {
            name
        };
        let what = if keys.len() == 1 {
            keys[0].clone()
        } else {
            format!("{} … ({})", keys[0], keys.len())
        };
        let fields = vec![
            Field::Note(what),
            Self::reg_text("reg.file", self.i18n.t("reg-field-file"), target),
        ];
        self.reg.pending = Some(RegPending::Export { keys });
        self.mode = Mode::Form(Box::new(self.reg_form("reg-export-title", fields, 1)));
    }

    /// `F8`: delete the value or the key under the cursor, after a question that names it
    /// — and, for a key, says how many keys go with it — with *Cancel* lit.
    pub(super) fn reg_delete(&mut self) {
        let Some(key) = self.reg_key_to_change() else {
            return;
        };
        let backups = self.reg_backup_dir();
        let note = self.i18n.t_args(
            "reg-backup-note",
            &[("dir", backups.display().to_string().into())],
        );
        let (lines, work) = match self.reg_row() {
            Some(RegRow::Value(name)) => (
                vec![self.i18n.t_args(
                    "reg-ask-delete-value",
                    &[
                        ("name", self.reg_value_name(&name).into()),
                        ("key", key.clone().into()),
                    ],
                )],
                RegWork::DeleteValue { key, name },
            ),
            Some(RegRow::Key(name)) => {
                let full = registry::join(&key, &name);
                let held = match registry::count_subkeys(self.reg.registry.as_ref(), &full) {
                    Ok(held) => held,
                    Err(error) => {
                        let m = self.reg_error(&full, &error);
                        self.set_error(m);
                        return;
                    }
                };
                let mut lines = vec![self
                    .i18n
                    .t_args("reg-ask-delete-key", &[("key", full.clone().into())])];
                if held > 0 {
                    lines.push(
                        self.i18n
                            .t_args("reg-holds", &[("count", (held as i64).into())]),
                    );
                }
                (lines, RegWork::DeleteKey { key: full })
            }
            _ => {
                let m = self.i18n.t("msg-nothing-to-do");
                self.set_error(m);
                return;
            }
        };
        let mut lines = lines;
        lines.push(note);
        let ask = self.reg_ask(work);
        self.mode = Mode::Confirm(Confirm {
            title: self.plain("drive-registry"),
            lines,
            action: ConfirmAction::Registry(Box::new(ask)),
            button: 1,
        });
    }

    fn reg_ask(&self, work: RegWork) -> RegAsk {
        RegAsk {
            side: self.active,
            backups: self.reg_backup_dir(),
            stamp: self.reg_stamp(),
            work,
        }
    }

    fn reg_start(&mut self, work: RegWork) {
        let ask = self.reg_ask(work);
        self.requests.push(Request::Registry(ask));
    }

    /// What a dialog's rows of this key hold, as typed: a string value may begin or end
    /// with a space, and those are part of it.
    fn reg_typed(form: &Form, want: &str) -> Vec<String> {
        form.fields
            .iter()
            .filter_map(|field| match field {
                Field::Text { key, value, .. } if *key == want => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// OK in one of this panel's dialogs.
    pub(super) fn apply_registry_form(&mut self, form: &Form) {
        let Some(pending) = self.reg.pending.take() else {
            return;
        };
        let first = |want: &str| {
            Self::reg_typed(form, want)
                .into_iter()
                .next()
                .unwrap_or_default()
        };
        match pending {
            RegPending::Edit { key, name, kind } => {
                let typed = Self::reg_typed(form, "reg.data");
                let data = if kind == regfile::REG_MULTI_SZ {
                    // An empty row is a row nobody filled in: a list cannot hold an empty
                    // string anyway, since that is what ends it.
                    let strings: Vec<&String> = typed.iter().filter(|s| !s.is_empty()).collect();
                    Ok(Value::multi_sz(&name, &strings).data)
                } else {
                    regfile::parse_data(kind, typed.first().map_or("", String::as_str))
                };
                match data {
                    Ok(data) => self.reg_start(RegWork::SetValue {
                        key,
                        value: Value { name, kind, data },
                        new: false,
                    }),
                    Err(why) => {
                        let m = match why {
                            DataError::NotANumber => self.i18n.t("reg-bad-number"),
                            DataError::TooLarge => self.i18n.t_args(
                                "reg-too-large",
                                &[("type", regfile::type_name(kind).into())],
                            ),
                            DataError::NotHex => self.i18n.t("reg-bad-hex"),
                        };
                        self.set_error(m);
                        // The dialog stays, with what was typed, for the mistake to be
                        // put right in.
                        self.reg.pending = Some(RegPending::Edit { key, name, kind });
                        let mut form = form.clone();
                        form.button = None;
                        self.mode = Mode::Form(Box::new(form));
                    }
                }
            }
            RegPending::NewValue { key } => {
                let name = first("reg.name").trim().to_string();
                let kind = form
                    .choice_value("reg.type")
                    .parse::<u32>()
                    .unwrap_or(regfile::REG_SZ);
                let taken =
                    self.reg.registry.values(&key).is_ok_and(|values| {
                        values.iter().any(|v| registry::same_name(&v.name, &name))
                    });
                if taken {
                    let shown = self.reg_value_name(&name);
                    let m = self.i18n.t_args("reg-exists", &[("name", shown.into())]);
                    self.set_error(m);
                    return;
                }
                self.reg_start(RegWork::SetValue {
                    key,
                    value: Value::empty(&name, kind),
                    new: true,
                });
            }
            RegPending::NewKey { key } => {
                let name = first("reg.name").trim().trim_matches('\\').to_string();
                if name.is_empty() {
                    return;
                }
                self.reg_start(RegWork::CreateKey { key, name });
            }
            RegPending::RenameValue { key, name } => {
                let new = first("reg.name").trim().to_string();
                if new == name {
                    return;
                }
                self.reg_start(RegWork::RenameValue {
                    key,
                    old: name,
                    new,
                });
            }
            RegPending::RenameKey { key } => {
                let name = first("reg.name").trim().to_string();
                // A key's name cannot hold the character that separates keys.
                if name.is_empty() || name.contains('\\') || name == registry::leaf(&key) {
                    return;
                }
                self.reg_start(RegWork::RenameKey { key, name });
            }
            RegPending::Export { keys } => {
                let raw = first("reg.file").trim().to_string();
                if raw.is_empty() {
                    return;
                }
                let target = self.resolve_path(&raw);
                if !target.is_local() || target.in_container() {
                    let m = self.i18n.t("temp-not-a-file");
                    self.set_error(m);
                    return;
                }
                let mut file = target.as_path().to_path_buf();
                if file.extension().is_none() {
                    file.set_extension("reg");
                }
                self.reg_start(RegWork::Export { keys, file });
            }
        }
    }

    /// `Enter` on a `.reg` file in an ordinary panel, where there is a registry: the file
    /// is read on a worker, and what it would write is asked about before it is written.
    /// `true` when the key was taken.
    pub(super) fn reg_offer_import(&mut self) -> bool {
        if !self.reg.available || self.panel().temp.is_some() {
            return false;
        }
        let Some(path) = self.panel().current_path() else {
            return false;
        };
        let is_reg = path
            .as_path()
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("reg"));
        if !is_reg || !path.is_local() || path.in_container() {
            return false;
        }
        self.reg_start(RegWork::Inspect {
            file: path.as_path().to_path_buf(),
        });
        true
    }

    /// The worker's answer.
    pub(super) fn reg_done(&mut self, done: RegDone) {
        let RegDone { ask, result } = done;
        let file_of = |file: &Path| file.display().to_string();
        match result {
            Ok(RegOk::Changed { backup }) => {
                let (what, focus, edit) = match &ask.work {
                    RegWork::SetValue { key, value, new } => (
                        format!("{key}\\{}", self.reg_value_name(&value.name)),
                        Some(RegRow::Value(value.name.clone())),
                        new.then(|| (key.clone(), value.name.clone())),
                    ),
                    RegWork::RenameValue { key, new, .. } => (
                        format!("{key}\\{}", self.reg_value_name(new)),
                        Some(RegRow::Value(new.clone())),
                        None,
                    ),
                    RegWork::DeleteValue { key, name } => {
                        (format!("{key}\\{}", self.reg_value_name(name)), None, None)
                    }
                    RegWork::CreateKey { key, name } => (
                        registry::join(key, name),
                        // A name with a `\` in it made several keys: the first is the row.
                        name.split('\\').next().map(|n| RegRow::Key(n.to_string())),
                        None,
                    ),
                    RegWork::RenameKey { key, name } => (
                        registry::join(registry::parent(key).unwrap_or_default(), name),
                        Some(RegRow::Key(name.clone())),
                        None,
                    ),
                    RegWork::DeleteKey { key } => (key.clone(), None, None),
                    RegWork::Export { .. } | RegWork::Inspect { .. } | RegWork::Import { .. } => {
                        (String::new(), None, None)
                    }
                };
                let m = self.i18n.t_args(
                    "reg-done",
                    &[("what", what.into()), ("backup", file_of(&backup).into())],
                );
                self.reg_reread(focus.map(|row| (ask.side, row)));
                self.set_status(m);
                // A value just made is empty: the editor opens on it, as it would after
                // `F4`, if the panel is still where it was made.
                if let Some((key, name)) = edit {
                    if self.reg_key(self.active).as_deref() == Some(key.as_str())
                        && matches!(self.mode, Mode::Normal)
                    {
                        self.reg_edit_value(&key, &name);
                    }
                }
            }
            Ok(RegOk::Exported { keys }) => {
                if let RegWork::Export { file, .. } = &ask.work {
                    let m = self.i18n.t_args(
                        "reg-exported",
                        &[
                            ("count", (keys as i64).into()),
                            ("file", file_of(file).into()),
                        ],
                    );
                    self.set_status(m);
                }
            }
            Ok(RegOk::Inspected {
                keys,
                values,
                deletes,
            }) => {
                let RegWork::Inspect { file } = &ask.work else {
                    return;
                };
                let lines = vec![
                    self.i18n
                        .t_args("reg-ask-import", &[("file", file_of(file).into())]),
                    self.i18n.t_args(
                        "reg-import-what",
                        &[
                            ("keys", (keys as i64).into()),
                            ("values", (values as i64).into()),
                            ("deletes", (deletes as i64).into()),
                        ],
                    ),
                    self.i18n
                        .t_args("reg-backup-note", &[("dir", file_of(&ask.backups).into())]),
                ];
                let import = RegAsk {
                    work: RegWork::Import { file: file.clone() },
                    ..ask.clone()
                };
                self.mode = Mode::Confirm(Confirm {
                    title: self.plain("drive-registry"),
                    lines,
                    action: ConfirmAction::Registry(Box::new(import)),
                    button: 1,
                });
            }
            Ok(RegOk::Imported { backup }) => {
                if let RegWork::Import { file } = &ask.work {
                    let m = self.i18n.t_args(
                        "reg-imported",
                        &[
                            ("file", file_of(file).into()),
                            ("backup", file_of(&backup).into()),
                        ],
                    );
                    self.reg_reread(None);
                    self.set_status(m);
                }
            }
            Err(failed) => {
                let file = match &ask.work {
                    RegWork::Export { file, .. }
                    | RegWork::Inspect { file }
                    | RegWork::Import { file } => file_of(file),
                    _ => String::new(),
                };
                let m = match failed {
                    RegFailed::Backup(error) => self.i18n.t_args(
                        "reg-backup-failed",
                        &[
                            ("dir", file_of(&ask.backups).into()),
                            ("error", error.into()),
                        ],
                    ),
                    RegFailed::Registry { key, error } => self.reg_error(&key, &error),
                    RegFailed::Exists(name) => {
                        let shown = self.reg_value_name(&name);
                        self.i18n.t_args("reg-exists", &[("name", shown.into())])
                    }
                    RegFailed::File(error) => self.i18n.t_args(
                        "reg-file-failed",
                        &[("file", file.into()), ("error", error.into())],
                    ),
                    RegFailed::NotARegFile(error) => self.i18n.t_args(
                        "reg-not-a-reg-file",
                        &[("file", file.into()), ("error", error.into())],
                    ),
                };
                // Half an import may have been written before the refusal: what the
                // panels show is read again either way.
                self.reg_reread(None);
                self.set_error(m);
            }
        }
    }
}
