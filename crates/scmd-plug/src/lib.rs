//! Declarative plugins: a directory with a `plugin.toml` manifest and the files it declares.
//!
//! This is the first and simplest plugin tier of `docs/spec/08-plugins.md`: nothing here loads
//! code, so there is no sandbox to get wrong. A plugin is data — themes, keymaps, file
//! associations, decompiler rules and mask groups — and this crate only *carries* that data.
//! The declared blocks are handed to the host crates as raw TOML tables so that `scmd-ui`,
//! `scmd-cmd` and `scmd-config` keep parsing them with the parsers they already have; this
//! crate deliberately depends on none of them and understands no theme key.
//!
//! Discipline, the same as `scmd-config`: a broken plugin is reported and skipped, never fatal
//! and never silently ignored, and one bad plugin never stops the others.

pub mod host;

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The manifest, inside the plugin's own directory. The folder name is the plugin id.
pub const MANIFEST_FILE_NAME: &str = "plugin.toml";

#[derive(Debug, thiserror::Error)]
pub enum PlugError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{path}: [plugin] has no id")]
    MissingId { path: String },
    #[error("{path}: [plugin] id `{id}` is not a plugin id (lower-case letters, digits and `-`)")]
    BadId { path: String, id: String },
    #[error("{path}: [plugin] id `{id}` disagrees with the folder name `{folder}`")]
    IdMismatch {
        path: String,
        id: String,
        folder: String,
    },
    #[error("{path}: [[command]] `{id}` is not a name a plugin may define (it must start with `user.` or `plugin.`)")]
    BadCommandId { path: String, id: String },
    #[error("{path}: [[command]] `{id}` asks for `{needs}`, which is not something we pass")]
    UnknownNeed {
        path: String,
        id: String,
        needs: String,
    },
    #[error("{path}: [[command]] `{id}` has no label")]
    CommandLabel { path: String, id: String },
    #[error("{path}: duplicate plugin id `{id}`, already loaded from `{first}`")]
    DuplicateId {
        path: String,
        id: String,
        first: String,
    },
    #[error("{path}: [[{section}]] file = `{file}` is not a path inside the plugin's directory")]
    Escape {
        path: String,
        section: &'static str,
        file: String,
    },
    #[error("{path}: [[{section}]] file = `{file}` does not exist")]
    MissingFile {
        path: String,
        section: &'static str,
        file: String,
    },
    #[error("{path}: [plugin] scmd = `{req}` is not a requirement (`>=x.y.z`, `^x.y.z`, `x.y.z`)")]
    BadRequirement { path: String, req: String },
}

/// What a declared block is for. The serde spelling is the section name in the manifest, so
/// `[[mask-group]]` and [`ItemKind::MaskGroup`] cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    Theme,
    Keymap,
    Assoc,
    Decompile,
    MaskGroup,
    /// A `[[command]]`. It carries no TOML block to resolve — `ItemKind::all()` leaves it out
    /// for that reason — and is here so a refused `run =` path names its own section.
    Command,
}

impl ItemKind {
    /// The section name as it is written in a manifest, for error messages.
    pub fn section(self) -> &'static str {
        match self {
            ItemKind::Theme => "theme",
            ItemKind::Keymap => "keymap",
            ItemKind::Assoc => "assoc",
            ItemKind::Decompile => "decompile",
            ItemKind::MaskGroup => "mask-group",
            ItemKind::Command => "command",
        }
    }

    /// Every kind, in manifest order: what a host iterates over.
    pub fn all() -> &'static [ItemKind] {
        &[
            ItemKind::Theme,
            ItemKind::Keymap,
            ItemKind::Assoc,
            ItemKind::Decompile,
            ItemKind::MaskGroup,
        ]
    }
}

/// One declared block, resolved: the TOML the host crate will parse, and where it came from.
///
/// The value is raw on purpose. A theme is a theme because `scmd-ui` says so, not because this
/// crate looked inside it, which is what keeps the declarative tier free of the whole UI.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    /// `plugin:<id>`, the origin string the host reports to the user.
    pub origin: String,
    pub value: toml::Table,
}

/// `[plugin]`: who this is, and which scmd versions it says it works with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    /// A requirement over the scmd version: `>=0.0.3`, `^0.1.0`, `0.1.0`, or empty for any.
    pub scmd: String,
    /// Keys we do not know — `capabilities`, `platforms` and the rest of the WASM tier's
    /// vocabulary among them. Kept so they can be reported rather than vanish.
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, toml::Value>,
}

/// A `plugin.toml`, as parsed. Item blocks stay as tables: see [`Item`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub plugin: PluginMeta,
    /// `[[theme]]` blocks: inline, or `file = "themes/sunset.toml"`.
    #[serde(rename = "theme")]
    pub themes: Vec<toml::Table>,
    /// `[[keymap]]` blocks.
    #[serde(rename = "keymap")]
    pub keymaps: Vec<toml::Table>,
    /// `[[assoc]]` blocks: per-mask `view`/`edit`/… commands (`docs/spec/06-tools.md` §5).
    #[serde(rename = "assoc")]
    pub assocs: Vec<toml::Table>,
    /// `[[decompile]]` blocks: external tools the viewer may call.
    #[serde(rename = "decompile")]
    pub decompile: Vec<toml::Table>,
    /// `[[mask-group]]` blocks: named mask lists, usable wherever a mask is asked for.
    #[serde(rename = "mask-group")]
    pub mask_groups: Vec<toml::Table>,
    /// `[[command]]` blocks: a command the plugin provides (SPEC 08 §1). This is the first
    /// thing a plugin can do beyond carrying data, and it is deliberately a *process* rather
    /// than code in our address space: one artifact for every platform, no sandbox to get
    /// wrong, and nothing to match compiler versions with.
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
    /// Anything else in the file, reported as a warning the way `scmd-config` does it.
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, toml::Value>,
}

/// A command a plugin provides: what the palette shows, and what the host runs.
///
/// The host passes the context on standard input as JSON and reads *actions* from standard
/// output, one per line. An action names a command of the vocabulary, so SPEC 08's first
/// sentence holds literally — everything a plugin can do, a key or a macro can do too.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Command {
    /// The id, which must be in the `user.` or `plugin.` space: the built-in vocabulary is
    /// not something a plugin may redefine.
    pub id: String,
    /// What the palette and the plugin menu show.
    pub label: String,
    /// Where it may run: `panel`, `viewer`, `editor` or `global`. Default `panel`.
    #[serde(default)]
    pub scope: Option<String>,
    /// The program to run, relative to the plugin's own directory.
    pub run: String,
    /// What it is given. Nothing is passed that is not asked for, so a plugin that wants the
    /// selection says so and one that does not never sees it.
    #[serde(default)]
    pub needs: Vec<String>,
    /// How long it may take, in seconds. A plugin that hangs is killed and said so.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl Command {
    /// Is `id` something a plugin may define? The built-in vocabulary is not.
    pub fn id_is_allowed(id: &str) -> bool {
        let id = id.trim();
        (id.starts_with("user.") || id.starts_with("plugin."))
            && id.len() > 5
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
    }

    /// Does it ask for something we know how to give?
    pub fn needs_are_known(&self) -> Option<&str> {
        const KNOWN: [&str; 4] = ["panel", "other", "selection", "cursor"];
        self.needs
            .iter()
            .find(|n| !KNOWN.contains(&n.as_str()))
            .map(String::as_str)
    }
}

impl Manifest {
    /// Parse a manifest. `name` is only used in messages, so a caller with the text in hand
    /// (a test, a registry) does not need a file.
    pub fn parse(name: &str, src: &str) -> Result<(Manifest, Vec<String>), PlugError> {
        let manifest: Manifest = toml::from_str(src).map_err(|source| PlugError::Toml {
            path: name.to_string(),
            source,
        })?;
        let warnings = manifest
            .unknown
            .keys()
            .map(|k| format!("{name}: unknown section or key `{k}` (ignored)"))
            .chain(
                manifest
                    .plugin
                    .unknown
                    .keys()
                    .map(|k| format!("{name}: unknown key `plugin.{k}` (ignored)")),
            )
            .collect();
        Ok((manifest, warnings))
    }

    /// Read `<dir>/plugin.toml`. A missing manifest is an [`PlugError::Io`]: whether that
    /// directory was meant to be a plugin at all is [`discover`]'s business, not ours.
    pub fn load(dir: &Path) -> Result<(Manifest, Vec<String>), PlugError> {
        let file = dir.join(MANIFEST_FILE_NAME);
        let name = file.display().to_string();
        let src = std::fs::read_to_string(&file).map_err(|source| PlugError::Io {
            path: name.clone(),
            source,
        })?;
        Manifest::parse(&name, &src)
    }

    /// The blocks of one kind, as declared. Resolution of `file = ` happens in [`Plugin`],
    /// which is the only thing that knows the directory they are relative to.
    pub fn declared(&self, kind: ItemKind) -> &[toml::Table] {
        match kind {
            ItemKind::Theme => &self.themes,
            ItemKind::Keymap => &self.keymaps,
            ItemKind::Assoc => &self.assocs,
            ItemKind::Decompile => &self.decompile,
            ItemKind::MaskGroup => &self.mask_groups,
            // Not a declared TOML block: `[[command]]` is parsed into a type of its own.
            ItemKind::Command => &[],
        }
    }
}

/// A loaded plugin: its manifest, its directory, and every block resolved to a table.
///
/// Everything that can fail has already failed by the time one of these exists, which is why
/// [`Plugin::items`] cannot return an error: the host gets data or nothing.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub dir: PathBuf,
    pub manifest: Manifest,
    items: Vec<Item>,
    warnings: Vec<String>,
}

impl Plugin {
    /// Load the plugin in `dir`: parse the manifest, check the id against the folder name,
    /// check the version requirement is readable, and read every file the manifest points at.
    ///
    /// Eager, on purpose. A plugin that is half-loadable is worse than one that is reported
    /// and skipped, and reading a handful of small TOML files at startup costs nothing.
    pub fn load(dir: &Path) -> Result<Plugin, PlugError> {
        let path = dir.join(MANIFEST_FILE_NAME).display().to_string();
        let (manifest, mut warnings) = Manifest::load(dir)?;

        let id = manifest.plugin.id.trim();
        if id.is_empty() {
            return Err(PlugError::MissingId { path });
        }
        if !is_plugin_id(id) {
            return Err(PlugError::BadId {
                path,
                id: id.to_string(),
            });
        }
        // The folder name is the id, so removing a plugin stays "delete that directory" and
        // two plugins cannot fight over one name. Case is forgiven with a warning, because a
        // case-insensitive filesystem cannot tell `Sunset` from `sunset` and the same plugin
        // tree must behave the same on all three platforms.
        let folder = dir
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        if !folder.eq_ignore_ascii_case(id) {
            return Err(PlugError::IdMismatch {
                path,
                id: id.to_string(),
                folder,
            });
        }
        if folder != id {
            warnings.push(format!(
                "{path}: folder `{folder}` differs in case from the plugin id `{id}`"
            ));
        }

        // An unreadable requirement is a problem now rather than a silent "unsupported" later.
        if Requirement::parse(&manifest.plugin.scmd).is_none() {
            return Err(PlugError::BadRequirement {
                path,
                req: manifest.plugin.scmd.clone(),
            });
        }

        let origin = format!("plugin:{id}");
        let mut items = Vec::new();
        for kind in ItemKind::all() {
            for declared in manifest.declared(*kind) {
                items.push(Item {
                    kind: *kind,
                    origin: origin.clone(),
                    value: resolve(dir, &path, *kind, declared, &mut warnings)?,
                });
            }
        }

        // The declared commands, checked before anything can run one: a name outside the
        // plugin space, a request for something we do not pass, or a program that is not in
        // the plugin's own directory is a refusal to load, not a surprise at the first press.
        for command in &manifest.commands {
            if !Command::id_is_allowed(&command.id) {
                return Err(PlugError::BadCommandId {
                    path,
                    id: command.id.clone(),
                });
            }
            if command.label.trim().is_empty() {
                return Err(PlugError::CommandLabel {
                    path,
                    id: command.id.clone(),
                });
            }
            if let Some(needs) = command.needs_are_known() {
                return Err(PlugError::UnknownNeed {
                    path,
                    id: command.id.clone(),
                    needs: needs.to_string(),
                });
            }
            // The same rule as a `file =` entry: inside the plugin's directory, or nothing.
            safe_join(dir, &path, ItemKind::Command, &command.run)?;
        }

        Ok(Plugin {
            dir: dir.to_path_buf(),
            manifest,
            items,
            warnings,
        })
    }

    /// The commands this plugin provides, with the program resolved to a real path.
    pub fn commands(&self) -> Vec<(&Command, PathBuf)> {
        self.manifest
            .commands
            .iter()
            .map(|c| (c, self.dir.join(c.run.trim())))
            .collect()
    }

    pub fn id(&self) -> &str {
        self.manifest.plugin.id.trim()
    }

    /// What the host shows as the source of a theme or a binding: `plugin:<id>`, which is
    /// exactly what `ThemeSource::Plugin` and the keymap's origin field expect.
    pub fn origin(&self) -> String {
        format!("plugin:{}", self.id())
    }

    /// Everything this plugin declares, in manifest order, kind by kind.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The tables of one kind, ready to hand to a host parser that takes `&[toml::Table]`.
    pub fn tables(&self, kind: ItemKind) -> Vec<toml::Table> {
        self.items
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| i.value.clone())
            .collect()
    }

    /// Non-fatal remarks about this plugin: unknown keys, a case-different folder name.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Does the plugin declare support for this scmd version?
    ///
    /// Three forms are enough for three cases, and a hand-rolled compare keeps a semver
    /// dependency out of a crate whose whole job is to read five kinds of table.
    pub fn supports(&self, scmd_version: &str) -> bool {
        // `load` rejects an unreadable requirement, so `Any` here only covers a `Plugin`
        // somebody built by hand.
        let req = Requirement::parse(&self.manifest.plugin.scmd).unwrap_or(Requirement::Any);
        match Version::parse(scmd_version) {
            Some(version) => req.matches(version),
            // We cannot read our own version: that is not the plugin's fault.
            None => true,
        }
    }
}

/// What one walk of the plugins directory found.
#[derive(Debug, Default)]
pub struct Discovery {
    /// The plugins that loaded, in directory-name order.
    pub plugins: Vec<Plugin>,
    /// One message per plugin that did not load, naming the file and the reason.
    pub problems: Vec<String>,
    /// Non-fatal remarks from the plugins that did load.
    pub warnings: Vec<String>,
}

impl Discovery {
    pub fn ids(&self) -> Vec<&str> {
        self.plugins.iter().map(Plugin::id).collect()
    }

    pub fn get(&self, id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.id() == id.trim())
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// What `scmd --list-plugins` prints: what loaded, then what did not.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        for p in &self.plugins {
            let meta = &p.manifest.plugin;
            let name = if meta.name.is_empty() {
                p.id()
            } else {
                meta.name.as_str()
            };
            // What it brings, counted separately: a plugin that provides one command and no
            // data read as "(0 items)", which is the opposite of what it does.
            let mut brings: Vec<String> = Vec::new();
            if !p.items().is_empty() {
                brings.push(format!("{} items", p.items().len()));
            }
            let commands = p.manifest.commands.len();
            if commands > 0 {
                brings.push(format!(
                    "{commands} command{}",
                    if commands == 1 { "" } else { "s" }
                ));
            }
            if brings.is_empty() {
                brings.push("nothing declared".to_string());
            }
            out.push_str(&format!(
                "{:<16} {:<10} {}   ({})\n",
                p.id(),
                meta.version,
                name,
                brings.join(", ")
            ));
        }
        for problem in &self.problems {
            out.push_str(&format!("!  {problem}\n"));
        }
        out
    }

    /// Add a plugin, unless its id is taken. The first one wins: whichever directory the
    /// filesystem lists first keeps the name, and the other is reported.
    fn push(&mut self, plugin: Plugin) {
        if let Some(first) = self
            .plugins
            .iter()
            .find(|p| p.id().eq_ignore_ascii_case(plugin.id()))
        {
            self.problems.push(
                PlugError::DuplicateId {
                    path: plugin.dir.join(MANIFEST_FILE_NAME).display().to_string(),
                    id: plugin.id().to_string(),
                    first: first.dir.display().to_string(),
                }
                .to_string(),
            );
            return;
        }
        self.warnings.extend(plugin.warnings.iter().cloned());
        self.plugins.push(plugin);
    }
}

/// Walk one level of the plugins directory.
///
/// One level only: a plugin is a directory with a manifest, not a tree to search, so nothing
/// deeper is read and the walk cannot wander off through a nested symlink. A directory without
/// a `plugin.toml` is skipped quietly — it is not pretending to be a plugin — and so is a plain
/// file, which is the later WASM/Lua tier's business (`docs/spec/08-plugins.md` §3).
pub fn discover(dir: &Path) -> Discovery {
    let mut found = Discovery::default();
    // No plugins directory is the normal case, not a problem: `scmd --paths` already says so.
    if !dir.is_dir() {
        return found;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) => {
            found.problems.push(
                PlugError::Io {
                    path: dir.display().to_string(),
                    source,
                }
                .to_string(),
            );
            return found;
        }
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries {
        match entry {
            // `is_dir` follows links: a plugin folder may legitimately be a symlink, and what
            // it may *contain* is checked per file when a `file = ` entry is resolved.
            Ok(entry) if entry.path().is_dir() => candidates.push(entry.path()),
            Ok(_) => {}
            Err(source) => found.problems.push(
                PlugError::Io {
                    path: dir.display().to_string(),
                    source,
                }
                .to_string(),
            ),
        }
    }
    // Directory order is not defined; ours is, so two runs report the same thing.
    candidates.sort();

    for candidate in candidates {
        if !candidate.join(MANIFEST_FILE_NAME).is_file() {
            continue;
        }
        match Plugin::load(&candidate) {
            Ok(plugin) => found.push(plugin),
            Err(e) => found.problems.push(e.to_string()),
        }
    }
    found
}

/// `[a-z0-9-]+`: short enough to type on a command line, safe as a directory name on every
/// filesystem, and with no room for a path separator or a case surprise.
pub fn is_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Resolve one declared block: inline tables are taken as they are, a `file = ` entry is read
/// from the plugin's own directory.
fn resolve(
    dir: &Path,
    manifest: &str,
    kind: ItemKind,
    declared: &toml::Table,
    warnings: &mut Vec<String>,
) -> Result<toml::Table, PlugError> {
    let section = kind.section();
    match declared.get("file") {
        Some(toml::Value::String(file)) => {
            if declared.len() > 1 {
                warnings.push(format!(
                    "{manifest}: [[{section}]] has `file` and inline keys; the file wins"
                ));
            }
            let path = safe_join(dir, manifest, kind, file)?;
            let name = path.display().to_string();
            let src = std::fs::read_to_string(&path).map_err(|source| PlugError::Io {
                path: name.clone(),
                source,
            })?;
            toml::from_str(&src).map_err(|source| PlugError::Toml { path: name, source })
        }
        Some(_) => {
            warnings.push(format!(
                "{manifest}: [[{section}]] `file` is not a string; the block is taken as inline"
            ));
            Ok(declared.clone())
        }
        None => Ok(declared.clone()),
    }
}

/// Join a `file = ` entry to the plugin's directory, or refuse.
///
/// A manifest is data somebody downloaded, and the plugins folder is the only directory this
/// program reads, so `../../etc/passwd` and `/etc/passwd` must never become a read. Two checks,
/// because strings alone are not enough: the path must be relative with no `..` in it, *and*
/// the canonical target must still be inside the canonical plugin directory — which is what
/// catches a symlink that points out of the tree. The directory is canonicalised too, or a
/// plugins folder reached through a link (`/tmp` on macOS) would fail the comparison.
fn safe_join(dir: &Path, manifest: &str, kind: ItemKind, file: &str) -> Result<PathBuf, PlugError> {
    let escape = || PlugError::Escape {
        path: manifest.to_string(),
        section: kind.section(),
        file: file.to_string(),
    };

    let relative = Path::new(file.trim());
    let mut named_something = false;
    for component in relative.components() {
        match component {
            Component::Normal(_) => named_something = true,
            Component::CurDir => {}
            // RootDir, Prefix (`C:\`, `\\?\`) and ParentDir all leave the directory.
            _ => return Err(escape()),
        }
    }
    if !named_something {
        return Err(escape());
    }

    let base = dir.canonicalize().map_err(|source| PlugError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let target = base.join(relative);
    let real = match target.canonicalize() {
        Ok(real) => real,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(PlugError::MissingFile {
                path: manifest.to_string(),
                section: kind.section(),
                file: file.to_string(),
            })
        }
        Err(source) => {
            return Err(PlugError::Io {
                path: target.display().to_string(),
                source,
            })
        }
    };
    if !real.starts_with(&base) {
        return Err(escape());
    }
    Ok(real)
}

/// `major.minor.patch`. Missing parts are zero, so `0.1` is `0.1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// Numbers and dots, nothing else: a pre-release or build suffix is not something three
    /// hand-written cases should pretend to understand.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let mut parts = text.split('.');
        let mut numbers = [0u64; 3];
        for (i, part) in parts.by_ref().take(3).enumerate() {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            numbers[i] = part.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        Some(Version::new(numbers[0], numbers[1], numbers[2]))
    }
}

/// What a manifest's `scmd = ` field asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Empty or `*`: the plugin makes no claim, so it is taken at the user's word.
    Any,
    /// `>=x.y.z`.
    AtLeast(Version),
    /// `^x.y.z`: up to, but not including, the next version semver calls incompatible — which
    /// below 1.0 is the next minor, and below 0.1 the next patch.
    Compatible(Version),
    /// `x.y.z` or `=x.y.z`.
    Exact(Version),
}

impl Requirement {
    pub fn parse(req: &str) -> Option<Requirement> {
        let req = req.trim();
        if req.is_empty() || req == "*" {
            return Some(Requirement::Any);
        }
        if let Some(rest) = req.strip_prefix(">=") {
            return Version::parse(rest).map(Requirement::AtLeast);
        }
        if let Some(rest) = req.strip_prefix('^') {
            return Version::parse(rest).map(Requirement::Compatible);
        }
        if let Some(rest) = req.strip_prefix('=') {
            return Version::parse(rest).map(Requirement::Exact);
        }
        Version::parse(req).map(Requirement::Exact)
    }

    pub fn matches(&self, version: Version) -> bool {
        match *self {
            Requirement::Any => true,
            Requirement::AtLeast(min) => version >= min,
            Requirement::Exact(want) => version == want,
            Requirement::Compatible(min) => {
                let upper = if min.major > 0 {
                    Version::new(min.major + 1, 0, 0)
                } else if min.minor > 0 {
                    Version::new(0, min.minor + 1, 0)
                } else {
                    Version::new(0, 0, min.patch + 1)
                };
                version >= min && version < upper
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plugin directory with the manifest text given, and nothing else.
    fn plugin(root: &Path, folder: &str, manifest: &str) -> PathBuf {
        let dir = root.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST_FILE_NAME), manifest).unwrap();
        dir
    }

    const SUNSET: &str = r##"
        [plugin]
        id = "sunset"
        name = "Sunset"
        version = "1.0.0"
        author = "A. Nother"
        description = "warm colours"
        scmd = ">=0.0.1"

        [[theme]]
        id = "sunset"
        inherit = "commander"
        [theme.colors]
        "selected.fg" = "#ff8800"

        [[keymap]]
        name = "sunset"
        inherit = "far"
        [[keymap.bind]]
        keys = ["F4"]
        cmd = "file.view"

        [[assoc]]
        mask = "*.md"
        view = "glow {file}"

        [[decompile]]
        name = "javap"
        match = "*.class"
        cmd = "javap -c {file}"

        [[mask-group]]
        name = "media"
        masks = "*.mp3,*.flac"
    "##;

    #[test]
    fn a_manifest_parses_from_a_string_with_every_item_kind() {
        let (m, warnings) = Manifest::parse("plugin.toml", SUNSET).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(m.plugin.id, "sunset");
        assert_eq!(m.plugin.name, "Sunset");
        assert_eq!(m.plugin.version, "1.0.0");
        assert_eq!(m.plugin.author, "A. Nother");
        assert_eq!(m.plugin.description, "warm colours");
        assert_eq!(m.themes.len(), 1);
        assert_eq!(m.keymaps.len(), 1);
        assert_eq!(m.assocs.len(), 1);
        assert_eq!(m.decompile.len(), 1);
        assert_eq!(m.mask_groups.len(), 1);
        assert_eq!(
            m.declared(ItemKind::MaskGroup)[0]["name"].as_str(),
            Some("media")
        );
    }

    #[test]
    fn an_unknown_section_or_key_is_a_warning_not_an_error() {
        let (m, warnings) = Manifest::parse(
            "plugin.toml",
            r#"
            [plugin]
            id = "later"
            capabilities = ["vfs:net"]
            [[column]]
            name = "git"
            "#,
        )
        .unwrap();
        assert_eq!(m.plugin.id, "later");
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(
            warnings.iter().any(|w| w.contains("`column`")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("`plugin.capabilities`")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_manifest_that_does_not_parse_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        plugin(root.path(), "broken", "[plugin\nid = ");
        let found = discover(root.path());
        assert!(found.is_empty());
        assert_eq!(found.problems.len(), 1);
        assert!(found.problems[0].contains("broken"), "{:?}", found.problems);
        assert!(
            found.problems[0].contains(MANIFEST_FILE_NAME),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_plugin_loads_from_a_directory_and_stamps_its_origin_on_every_item() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(root.path(), "sunset", SUNSET);
        let p = Plugin::load(&dir).unwrap();
        assert_eq!(p.id(), "sunset");
        assert_eq!(p.origin(), "plugin:sunset");
        assert_eq!(p.items().len(), 5);
        assert!(p.items().iter().all(|i| i.origin == "plugin:sunset"));
        assert!(p.warnings().is_empty(), "{:?}", p.warnings());
        let kinds: Vec<ItemKind> = p.items().iter().map(|i| i.kind).collect();
        assert_eq!(kinds, ItemKind::all().to_vec());
    }

    #[test]
    fn items_are_handed_over_as_raw_toml_for_the_host_to_parse() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(root.path(), "sunset", SUNSET);
        let p = Plugin::load(&dir).unwrap();
        // The theme table arrives exactly as written: this crate does not know what a colour
        // is, and `scmd-ui` is the one that will say whether `#ff8800` makes sense.
        let theme = &p.tables(ItemKind::Theme)[0];
        assert_eq!(theme["id"].as_str(), Some("sunset"));
        assert_eq!(theme["inherit"].as_str(), Some("commander"));
        assert_eq!(theme["colors"]["selected.fg"].as_str(), Some("#ff8800"));
        assert_eq!(p.tables(ItemKind::Assoc)[0]["mask"].as_str(), Some("*.md"));
        assert_eq!(
            p.tables(ItemKind::Decompile)[0]["match"].as_str(),
            Some("*.class")
        );
    }

    #[test]
    fn an_item_may_live_in_a_file_the_manifest_points_at() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "sunset",
            r#"
            [plugin]
            id = "sunset"
            [[theme]]
            file = "themes/sunset.toml"
            "#,
        );
        std::fs::create_dir_all(dir.join("themes")).unwrap();
        std::fs::write(
            dir.join("themes/sunset.toml"),
            "id = \"sunset\"\ninherit = \"far\"\n",
        )
        .unwrap();
        let p = Plugin::load(&dir).unwrap();
        assert_eq!(p.items().len(), 1);
        assert_eq!(p.items()[0].kind, ItemKind::Theme);
        assert_eq!(p.items()[0].value["inherit"].as_str(), Some("far"));
    }

    #[test]
    fn a_file_entry_beside_inline_keys_is_reported_as_a_warning() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "sunset",
            r#"
            [plugin]
            id = "sunset"
            [[theme]]
            file = "t.toml"
            id = "ignored"
            "#,
        );
        std::fs::write(dir.join("t.toml"), "id = \"from-file\"\n").unwrap();
        let p = Plugin::load(&dir).unwrap();
        assert_eq!(p.items()[0].value["id"].as_str(), Some("from-file"));
        assert_eq!(p.warnings().len(), 1, "{:?}", p.warnings());
        assert!(
            p.warnings()[0].contains("the file wins"),
            "{:?}",
            p.warnings()
        );
    }

    #[test]
    fn a_manifest_whose_id_disagrees_with_its_folder_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        plugin(
            root.path(),
            "dawn",
            "[plugin]\nid = \"sunset\"\nversion = \"1.0.0\"\n",
        );
        let found = discover(root.path());
        assert!(found.is_empty());
        assert_eq!(found.problems.len(), 1);
        assert!(
            found.problems[0].contains("disagrees with the folder name `dawn`"),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_folder_that_differs_only_in_case_loads_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(root.path(), "Sunset", "[plugin]\nid = \"sunset\"\n");
        let p = Plugin::load(&dir).unwrap();
        assert_eq!(p.id(), "sunset");
        assert_eq!(p.warnings().len(), 1, "{:?}", p.warnings());
        assert!(
            p.warnings()[0].contains("differs in case"),
            "{:?}",
            p.warnings()
        );
    }

    #[test]
    fn an_id_outside_the_allowed_alphabet_is_a_problem() {
        assert!(is_plugin_id("sunset"));
        assert!(is_plugin_id("git-status-2"));
        assert!(!is_plugin_id("Sunset"));
        assert!(!is_plugin_id("sun set"));
        assert!(!is_plugin_id("../etc"));
        assert!(!is_plugin_id(""));

        let root = tempfile::tempdir().unwrap();
        plugin(root.path(), "sun_set", "[plugin]\nid = \"sun_set\"\n");
        let found = discover(root.path());
        assert!(found.is_empty());
        assert!(
            found.problems[0].contains("is not a plugin id"),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_manifest_with_no_id_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        plugin(root.path(), "nameless", "[plugin]\nname = \"Nameless\"\n");
        let found = discover(root.path());
        assert!(found.is_empty());
        assert!(
            found.problems[0].contains("has no id"),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_duplicate_id_is_a_problem_and_the_first_plugin_wins() {
        let root = tempfile::tempdir().unwrap();
        // Two directories, one id. On a case-insensitive filesystem these are one directory;
        // where they are two, the second one to be walked is reported and skipped.
        plugin(
            root.path(),
            "Sunset",
            "[plugin]\nid = \"sunset\"\nname = \"First\"\n",
        );
        plugin(
            root.path(),
            "sunset",
            "[plugin]\nid = \"sunset\"\nname = \"Second\"\n",
        );
        let found = discover(root.path());
        if found.plugins.len() == 1 && found.problems.is_empty() {
            return; // a case-insensitive filesystem: there was only ever one directory
        }
        assert_eq!(found.len(), 1, "{:?}", found.ids());
        assert_eq!(found.problems.len(), 1, "{:?}", found.problems);
        assert!(
            found.problems[0].contains("duplicate plugin id `sunset`"),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_file_entry_pointing_outside_the_plugin_directory_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("secret.toml"), "id = \"stolen\"\n").unwrap();
        let dir = plugin(
            root.path(),
            "nosy",
            "[plugin]\nid = \"nosy\"\n[[theme]]\nfile = \"../secret.toml\"\n",
        );
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(
            e.contains("is not a path inside the plugin's directory"),
            "{e}"
        );
        assert!(e.contains("../secret.toml"), "{e}");

        // And the deep version, which is what a hostile manifest actually writes.
        let dir = plugin(
            root.path(),
            "nosier",
            "[plugin]\nid = \"nosier\"\n[[keymap]]\nfile = \"../../etc/passwd\"\n",
        );
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(
            e.contains("is not a path inside the plugin's directory"),
            "{e}"
        );
    }

    #[test]
    fn an_absolute_file_entry_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        // Forward slashes: a backslash in a bare TOML string is an escape, so `C:\W` would
        // fail to parse and the test would pass for the wrong reason. `C:/…` is just as
        // absolute to Windows.
        let absolute = if cfg!(windows) {
            "C:/Windows/win.ini"
        } else {
            "/etc/passwd"
        };
        let dir = plugin(
            root.path(),
            "nosy",
            &format!("[plugin]\nid = \"nosy\"\n[[assoc]]\nfile = \"{absolute}\"\n"),
        );
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(
            e.contains("is not a path inside the plugin's directory"),
            "{e}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_that_escapes_the_plugin_directory_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("secret.toml"), "id = \"stolen\"\n").unwrap();
        let dir = plugin(
            root.path(),
            "sneaky",
            "[plugin]\nid = \"sneaky\"\n[[theme]]\nfile = \"link.toml\"\n",
        );
        // The name is innocent and no component is `..`: only comparing canonical paths
        // catches this one.
        std::os::unix::fs::symlink(root.path().join("secret.toml"), dir.join("link.toml")).unwrap();
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(
            e.contains("is not a path inside the plugin's directory"),
            "{e}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_that_stays_inside_the_plugin_directory_is_read() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "tidy",
            "[plugin]\nid = \"tidy\"\n[[theme]]\nfile = \"current.toml\"\n",
        );
        std::fs::write(dir.join("real.toml"), "id = \"tidy\"\n").unwrap();
        std::os::unix::fs::symlink(dir.join("real.toml"), dir.join("current.toml")).unwrap();
        let p = Plugin::load(&dir).unwrap();
        assert_eq!(p.items()[0].value["id"].as_str(), Some("tidy"));
    }

    #[test]
    fn a_file_entry_that_names_a_missing_file_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "gappy",
            "[plugin]\nid = \"gappy\"\n[[theme]]\nfile = \"themes/gone.toml\"\n",
        );
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(e.contains("does not exist"), "{e}");
        assert!(e.contains("themes/gone.toml"), "{e}");
    }

    #[test]
    fn an_empty_file_entry_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "empty",
            "[plugin]\nid = \"empty\"\n[[theme]]\nfile = \"\"\n",
        );
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(
            e.contains("is not a path inside the plugin's directory"),
            "{e}"
        );
    }

    #[test]
    fn a_declared_file_that_does_not_parse_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "sunset",
            "[plugin]\nid = \"sunset\"\n[[theme]]\nfile = \"t.toml\"\n",
        );
        std::fs::write(dir.join("t.toml"), "id = \nnot toml").unwrap();
        let e = Plugin::load(&dir).unwrap_err().to_string();
        assert!(e.contains("t.toml"), "{e}");
    }

    #[test]
    fn an_unparseable_version_requirement_is_a_problem() {
        let root = tempfile::tempdir().unwrap();
        plugin(
            root.path(),
            "vague",
            "[plugin]\nid = \"vague\"\nscmd = \"~> 1.2\"\n",
        );
        let found = discover(root.path());
        assert!(found.is_empty());
        assert!(
            found.problems[0].contains("is not a requirement"),
            "{:?}",
            found.problems
        );
    }

    #[test]
    fn a_directory_without_a_manifest_is_skipped_quietly() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("notes/deeper")).unwrap();
        std::fs::write(root.path().join("notes/deeper/plugin.toml"), "[plugin]\n").unwrap();
        let found = discover(root.path());
        assert!(found.is_empty());
        // And the nested manifest is not found either: the walk is one level deep.
        assert!(found.problems.is_empty(), "{:?}", found.problems);
    }

    #[test]
    fn a_plain_file_in_the_plugins_folder_is_skipped_quietly() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("git-status.wasm"), b"\0asm").unwrap();
        std::fs::write(root.path().join("align.lua"), b"-- later").unwrap();
        let found = discover(root.path());
        assert!(found.is_empty());
        assert!(found.problems.is_empty(), "{:?}", found.problems);
    }

    #[test]
    fn a_missing_plugins_directory_is_not_a_problem() {
        let root = tempfile::tempdir().unwrap();
        let found = discover(&root.path().join("no-such-folder"));
        assert!(found.is_empty());
        assert!(found.problems.is_empty());
        assert!(found.warnings.is_empty());
        assert_eq!(found.describe(), "");
    }

    #[test]
    fn one_bad_plugin_never_stops_the_others() {
        let root = tempfile::tempdir().unwrap();
        plugin(root.path(), "aaa-broken", "[plugin\n");
        plugin(root.path(), "sunset", SUNSET);
        plugin(
            root.path(),
            "zzz-gone",
            "[plugin]\nid = \"zzz-gone\"\n[[theme]]\nfile = \"x.toml\"\n",
        );
        plugin(
            root.path(),
            "dusk",
            "[plugin]\nid = \"dusk\"\nversion = \"0.2.0\"\nname = \"Dusk\"\n",
        );
        let found = discover(root.path());
        assert_eq!(found.ids(), vec!["dusk", "sunset"]);
        assert_eq!(found.problems.len(), 2, "{:?}", found.problems);
        assert!(found.get("sunset").is_some());
        assert!(found.get("aaa-broken").is_none());
    }

    #[test]
    fn discovery_describes_what_it_found_and_what_it_refused() {
        let root = tempfile::tempdir().unwrap();
        plugin(root.path(), "sunset", SUNSET);
        plugin(root.path(), "broken", "[plugin\n");
        let found = discover(root.path());
        let text = found.describe();
        assert!(text.contains("sunset"), "{text}");
        assert!(text.contains("1.0.0"), "{text}");
        assert!(text.contains("Sunset"), "{text}");
        assert!(text.contains("5 items"), "{text}");
        assert!(text.contains("!  "), "{text}");
        assert!(text.contains("broken"), "{text}");
    }

    #[test]
    fn at_least_requirements_compare_numerically() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "sunset",
            "[plugin]\nid = \"sunset\"\nscmd = \">=0.0.3\"\n",
        );
        let p = Plugin::load(&dir).unwrap();
        assert!(!p.supports("0.0.2"));
        assert!(p.supports("0.0.3"));
        assert!(p.supports("0.0.10")); // not a string compare
        assert!(p.supports("1.4.0"));
        // A space after the operator is a typo worth forgiving, not a broken manifest.
        assert_eq!(
            Requirement::parse(">= 0.0.3"),
            Some(Requirement::AtLeast(Version::new(0, 0, 3)))
        );
    }

    #[test]
    fn caret_requirements_stop_at_the_next_incompatible_version() {
        let req = Requirement::parse("^1.2.3").unwrap();
        assert!(!req.matches(Version::new(1, 2, 2)));
        assert!(req.matches(Version::new(1, 2, 3)));
        assert!(req.matches(Version::new(1, 9, 9)));
        assert!(!req.matches(Version::new(2, 0, 0)));
        // Below 1.0 the minor is the breaking digit, and below 0.1 the patch is.
        let req = Requirement::parse("^0.2.0").unwrap();
        assert!(req.matches(Version::new(0, 2, 7)));
        assert!(!req.matches(Version::new(0, 3, 0)));
        let req = Requirement::parse("^0.0.3").unwrap();
        assert!(req.matches(Version::new(0, 0, 3)));
        assert!(!req.matches(Version::new(0, 0, 4)));
    }

    #[test]
    fn an_exact_requirement_matches_only_itself() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "picky",
            "[plugin]\nid = \"picky\"\nscmd = \"0.0.3\"\n",
        );
        let p = Plugin::load(&dir).unwrap();
        assert!(p.supports("0.0.3"));
        assert!(!p.supports("0.0.4"));
        assert!(!p.supports("0.0.2"));
        assert_eq!(
            Requirement::parse("=0.0.3"),
            Some(Requirement::Exact(Version::new(0, 0, 3)))
        );
    }

    #[test]
    fn a_requirement_that_is_absent_supports_every_version() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(root.path(), "easy", "[plugin]\nid = \"easy\"\n");
        let p = Plugin::load(&dir).unwrap();
        assert!(p.supports("0.0.1"));
        assert!(p.supports("99.0.0"));
        assert_eq!(Requirement::parse("*"), Some(Requirement::Any));
    }

    #[test]
    fn a_requirement_we_cannot_read_is_refused_rather_than_guessed() {
        for req in [
            ">1.0.0",
            "~1.0",
            "^",
            ">=",
            "1.0.0-beta",
            "latest",
            "0.1.2.3",
        ] {
            assert_eq!(Requirement::parse(req), None, "{req}");
        }
        assert_eq!(Version::parse("1"), Some(Version::new(1, 0, 0)));
        assert_eq!(Version::parse("0.1"), Some(Version::new(0, 1, 0)));
        assert_eq!(Version::parse(""), None);
        assert_eq!(Version::parse("1."), None);
    }

    #[test]
    fn an_unreadable_host_version_is_not_the_plugins_fault() {
        let root = tempfile::tempdir().unwrap();
        let dir = plugin(
            root.path(),
            "picky",
            "[plugin]\nid = \"picky\"\nscmd = \"0.0.3\"\n",
        );
        let p = Plugin::load(&dir).unwrap();
        assert!(p.supports("not a version"));
        assert!(p.supports("0.0.4-nightly"));
    }

    #[test]
    fn a_manifest_round_trips_through_toml() {
        let (m, _) = Manifest::parse("plugin.toml", SUNSET).unwrap();
        let text = toml::to_string(&m).unwrap();
        let (again, warnings) = Manifest::parse("round.toml", &text).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.plugin.id, m.plugin.id);
        assert_eq!(again.themes, m.themes);
        assert_eq!(again.mask_groups, m.mask_groups);
    }

    #[test]
    fn item_kinds_are_spelled_the_way_the_manifest_spells_them() {
        assert_eq!(ItemKind::MaskGroup.section(), "mask-group");
        assert_eq!(
            toml::Value::try_from(ItemKind::MaskGroup).unwrap().as_str(),
            Some("mask-group")
        );
        assert_eq!(ItemKind::all().len(), 5);
    }
}
