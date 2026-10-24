//! Configuration: discovery, TOML parsing, defaults, and a dump of what is in effect.
//!
//! Unknown keys are warnings, never errors: a config written for a later version still loads.
//! See `docs/spec/07-config-theming.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The single configuration file. There is no configuration *directory*: SupremeCommander
/// ships as one executable and the only directory it ever looks for is the plugins folder
/// (see `docs/spec/07-config-theming.md` §1).
pub const CONFIG_FILE_NAME: &str = "scmd.toml";
/// Histories, panel paths and other remembered state: one file, next to the config.
pub const STATE_FILE_NAME: &str = "scmd-state.toml";
/// Recorded keyboard macros (SPEC 08 §5), beside the configuration.
pub const MACROS_FILE_NAME: &str = "scmd-macros.toml";
/// The one directory: optional, and the only thing that may live outside the binary.
pub const PLUGINS_DIR_NAME: &str = "plugins";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
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
}

/// Where this run reads and writes. Three paths, no trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// The directory the executable sits in — where portable mode and plugins are looked for.
    pub exe_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    /// Recorded macros. Beside the configuration, because they are configuration one made by
    /// pressing keys rather than by typing TOML.
    pub macros_file: PathBuf,
    pub plugins_dir: PathBuf,
    /// True when the configuration sits next to the executable.
    pub portable: bool,
}

impl Paths {
    /// `--config <file>` wins. Otherwise, if `scmd.toml` sits next to the executable we are
    /// portable (a USB stick, a release zip); failing that, the per-OS config location — as a
    /// single file, `<config>/scmd.toml`, not a directory of our own.
    ///
    /// The plugins directory is `--plugins <dir>`, then `$SCMD_PLUGINS`, then `plugins/` next to
    /// the executable, then `scmd-plugins/` beside the config file.
    pub fn discover(config: Option<&Path>, plugins: Option<&Path>) -> Paths {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));

        let portable_config = exe_dir.join(CONFIG_FILE_NAME);
        let (config_file, portable) = match config {
            Some(explicit) => (explicit.to_path_buf(), false),
            None if portable_config.exists() => (portable_config, true),
            None => (
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(CONFIG_FILE_NAME),
                false,
            ),
        };

        let config_parent = config_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let state_file = config_parent.join(STATE_FILE_NAME);
        let macros_file = config_parent.join(MACROS_FILE_NAME);

        let plugins_dir = plugins
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("SCMD_PLUGINS").map(PathBuf::from))
            .unwrap_or_else(|| {
                let beside_exe = exe_dir.join(PLUGINS_DIR_NAME);
                if beside_exe.is_dir() {
                    beside_exe
                } else {
                    config_parent.join("scmd-plugins")
                }
            });

        Paths {
            exe_dir,
            config_file,
            state_file,
            macros_file,
            plugins_dir,
            portable,
        }
    }

    /// Human-readable summary for `scmd --paths`: the first question anyone asks.
    pub fn describe(&self) -> String {
        format!(
            "config   {}{}\nstate    {}\nmacros   {}{}\nplugins  {}{}\n",
            self.config_file.display(),
            if self.config_file.exists() {
                ""
            } else {
                "   (not present; defaults are compiled in)"
            },
            self.state_file.display(),
            self.macros_file.display(),
            if self.macros_file.exists() {
                ""
            } else {
                "   (not present; nothing recorded yet)"
            },
            self.plugins_dir.display(),
            if self.plugins_dir.is_dir() {
                ""
            } else {
                "   (not present; no plugins loaded)"
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub confirm_exit: bool,
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
}
// There is no `update_check` here, and that is the point: a setting nobody reads is a setting
// that lies, and this one was a promise wearing a setting's clothes. The promise is kept in
// the code instead — the program opens no connection the user did not ask for, and the only
// outbound code in the tree is the SFTP client behind `Alt+F1`. If a release check is ever
// wanted it will be a command one runs, not a flag that phones home at startup.

impl Default for General {
    fn default() -> Self {
        General {
            confirm_exit: false,
            confirm_delete: true,
            confirm_overwrite: true,
        }
    }
}

/// How typing in a panel behaves: FAR filters with `Alt`+letters, mc with bare letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuickSearch {
    Alt,
    Plain,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelCfg {
    pub view: String,
    pub sort: String,
    pub reverse: bool,
    pub dirs_first: bool,
    pub selected_first: bool,
    pub natural_sort: bool,
    /// On by default, and deliberately: a file manager that quietly leaves files out is
    /// lying about the directory. `.git` and `.env` are files like any other.
    pub show_hidden: bool,
    pub allow_reverse_sort: bool,
    /// The `Name | Size | Date | Time` row under the panel frame (FAR has it on by default).
    pub column_titles: bool,
    /// The item under the cursor, spelled out in the panel's own bottom row (FAR's
    /// `Show status line`), under a rule that also carries the selected totals.
    pub status_line: bool,
    /// A bar in the right frame showing where the view sits in a long listing.
    pub scrollbar: bool,
    /// Free space on the device, in the bottom frame.
    pub free_space: bool,
    pub quick_search: QuickSearch,
}

impl Default for PanelCfg {
    fn default() -> Self {
        PanelCfg {
            view: "full".into(),
            sort: "name".into(),
            reverse: false,
            dirs_first: true,
            selected_first: false,
            natural_sort: true,
            show_hidden: true,
            allow_reverse_sort: true,
            column_titles: true,
            status_line: true,
            scrollbar: true,
            free_space: true,
            quick_search: QuickSearch::Alt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    pub theme: String,
    pub keymap: String,
    pub language: String,
    pub key_bar: bool,
    pub status_bar: bool,
    /// The command line under the panels. Without it, typing goes to quick search.
    pub command_line: bool,
    /// How many command lines to keep between sessions.
    pub command_history: usize,
}

impl Default for UiCfg {
    fn default() -> Self {
        UiCfg {
            theme: "commander".into(),
            keymap: "far".into(),
            language: "auto".into(),
            key_bar: true,
            status_bar: true,
            command_line: true,
            command_history: 512,
        }
    }
}

/// `[net]`: the netbox's own settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetCfg {
    /// Where host keys are recorded. Empty means `~/.ssh/known_hosts`.
    pub known_hosts: String,
    /// Keepalive for a connection, in seconds. 0 turns it off.
    pub keepalive_secs: u64,
    /// The largest remote file the viewer and the editor will fetch into a temporary file.
    pub fetch_limit_mb: u64,
}

impl Default for NetCfg {
    fn default() -> Self {
        NetCfg {
            known_hosts: String::new(),
            keepalive_secs: 30,
            fetch_limit_mb: 512,
        }
    }
}

/// `[[s3]]` blocks: the object storage services a path may name.
///
/// There is deliberately no place here for an access key. A secret key is a password, and a
/// password in `scmd.toml` is a password in a backup, in a dotfiles repository and in a screen
/// share. The two places the rest of the world already keeps these are read instead: the
/// `AWS_*` environment variables and `~/.aws/credentials`, which every AWS tool shares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct S3Cfg {
    /// What a path calls this service: `s3://work@bucket/` names `work`. The service with an
    /// empty name is the default, reached by `s3://bucket/`.
    pub name: String,
    /// `https://minio.lan:9000`, `http://127.0.0.1:9000`, or empty for AWS itself.
    pub endpoint: String,
    /// The region the signature is scoped to. Empty means `AWS_REGION`, then `us-east-1`.
    pub region: String,
    /// Put the bucket in the path rather than in front of the host. Unset means the sensible
    /// default for the endpoint: AWS is addressed by host, everything else by path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_style: Option<bool>,
    /// `env`, `aws`, `aws:<profile>`, or `anonymous` for a bucket with a public read policy.
    pub credentials: String,
    /// How many rows a listing with no delimiter — every key under a prefix, however deep —
    /// will gather before it stops and says that it stopped. Zero means no limit, which on a
    /// bucket with millions of objects in it means a listing that takes minutes and a panel
    /// that holds all of them.
    pub listing_cap: usize,
}

impl Default for S3Cfg {
    fn default() -> S3Cfg {
        S3Cfg {
            name: String::new(),
            endpoint: String::new(),
            region: String::new(),
            path_style: None,
            credentials: "env".into(),
            listing_cap: 100_000,
        }
    }
}

/// `[arc]`: the guards every archive read goes through (SPEC 04 §4).
///
/// They are configuration because a zip bomb and a legitimate archive of a disk image look
/// alike from the outside, and only the person at the keyboard knows which one this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArcCfg {
    /// How many entries one archive may list.
    pub max_entries: usize,
    /// The total unpacked size an archive may add up to, in megabytes.
    pub max_total_mb: u64,
    /// Unpacked over packed. Text compresses 10:1 and a disk image far better, so the default
    /// is generous; a thousandfold archive is a bomb or a curiosity.
    pub max_ratio: u64,
    /// How many containers deep a path may go.
    pub max_depth: usize,
}

impl Default for ArcCfg {
    fn default() -> Self {
        let limits = scmd_arc::Limits::default();
        ArcCfg {
            max_entries: limits.entries,
            max_total_mb: limits.total_bytes >> 20,
            max_ratio: limits.ratio,
            max_depth: limits.depth,
        }
    }
}

impl ArcCfg {
    pub fn limits(&self) -> scmd_arc::Limits {
        scmd_arc::Limits {
            entries: self.max_entries,
            total_bytes: self.max_total_mb.saturating_mul(1 << 20),
            ratio: self.max_ratio,
            depth: self.max_depth,
        }
    }
}

/// A `[[site]]` block: one entry of the netbox's site list.
///
/// Secrets are never here. A site says *how* to authenticate (`agent`, `key:<path>`, `ask`)
/// and the password or passphrase is asked for when the connection needs it
/// (`docs/spec/04-vfs.md` §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteCfg {
    /// Short name, shown in the site list and usable from the command line.
    #[serde(default)]
    pub id: String,
    /// `sftp` today.
    #[serde(default = "default_scheme")]
    pub scheme: String,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
}

/// A blank site, as the session editor starts from: the scheme is the only field with a
/// sensible value, and `serde` fills the same one in for a `[[site]]` that leaves it out.
impl Default for SiteCfg {
    fn default() -> SiteCfg {
        SiteCfg {
            id: String::new(),
            scheme: default_scheme(),
            host: String::new(),
            port: None,
            user: None,
            auth: None,
            start_dir: None,
            group: None,
            keepalive_secs: None,
            compress: None,
        }
    }
}

fn default_scheme() -> String {
    "sftp".to_string()
}

/// A `[[decompile]]` rule: an external tool that turns a compiled file into readable text.
///
/// SupremeCommander disassembles x86 code and prints WebAssembly itself; real decompilation is
/// a research project, so it calls the specialists instead of reimplementing them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompileRule {
    /// Shown in the viewer's status line.
    pub name: String,
    /// `*.class`, `*.pyc`, or an exact file name.
    #[serde(rename = "match")]
    pub mask: String,
    /// The command. `{file}` is replaced with the path; it is executed directly, never
    /// through a shell.
    pub cmd: String,
}

impl DecompileRule {
    /// Does this rule apply to `name`? `*.ext` compares the extension, anything else the
    /// whole file name, both case-insensitively.
    pub fn matches(&self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        let mask = self.mask.trim().to_ascii_lowercase();
        match mask.strip_prefix("*.") {
            Some(ext) => name.rsplit('.').next() == Some(ext),
            None => name == mask,
        }
    }

    /// The program the rule runs, for an availability check.
    pub fn program(&self) -> &str {
        self.cmd.split_whitespace().next().unwrap_or("")
    }
}

/// Rules compiled in, used when the config file does not override them. Each is tried only if
/// its program is actually installed.
pub fn default_decompile_rules() -> Vec<DecompileRule> {
    let rule = |name: &str, mask: &str, cmd: &str| DecompileRule {
        name: name.to_string(),
        mask: mask.to_string(),
        cmd: cmd.to_string(),
    };
    vec![
        rule("javap", "*.class", "javap -c -p -constants {file}"),
        rule("ilspycmd", "*.dll", "ilspycmd {file}"),
        rule("python -m dis", "*.pyc", "python3 -m dis {file}"),
        rule("wasm2wat", "*.wasm", "wasm2wat {file}"),
        rule("objdump", "*.o", "objdump -d -C {file}"),
        rule("objdump", "*.so", "objdump -d -C {file}"),
        rule("objdump", "*.a", "objdump -d -C {file}"),
        rule(
            "retdec",
            "*.exe",
            "retdec-decompiler --cleanup -o /dev/stdout {file}",
        ),
    ]
}

/// Is `program` on the PATH (or an existing path of its own)?
pub fn command_exists(program: &str) -> bool {
    if program.is_empty() {
        return false;
    }
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let direct = dir.join(program);
        if direct.is_file() {
            return true;
        }
        // Windows: the extension is implied.
        ["exe", "bat", "cmd", "com"]
            .iter()
            .any(|ext| dir.join(format!("{program}.{ext}")).is_file())
    })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub panel: PanelCfg,
    pub ui: UiCfg,
    /// `[[theme]]` blocks: themes live in the config file, not in a themes directory.
    #[serde(rename = "theme")]
    pub themes: Vec<toml::Table>,
    /// `[[keymap]]` blocks: same reasoning.
    #[serde(rename = "keymap")]
    pub keymaps: Vec<toml::Table>,
    /// `[[decompile]]` blocks: external tools the viewer may call.
    #[serde(rename = "decompile")]
    pub decompile: Vec<DecompileRule>,
    pub net: NetCfg,
    pub arc: ArcCfg,
    /// `[[site]]` blocks: the netbox's site list.
    #[serde(rename = "site")]
    pub sites: Vec<SiteCfg>,
    /// `[[s3]]` blocks: object storage services.
    #[serde(rename = "s3")]
    pub s3: Vec<S3Cfg>,
    /// Anything we did not recognise, kept so `--dump-config` can show it and warn.
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, toml::Value>,
}

impl Config {
    /// Load the config file if it exists. A missing file is not an error; a broken one is.
    pub fn load(paths: &Paths) -> Result<(Config, Vec<String>), ConfigError> {
        let file = paths.config_file.clone();
        if !file.exists() {
            return Ok((Config::default(), Vec::new()));
        }
        let src = std::fs::read_to_string(&file).map_err(|source| ConfigError::Io {
            path: file.display().to_string(),
            source,
        })?;
        Self::parse(&file.display().to_string(), &src)
    }

    pub fn parse(name: &str, src: &str) -> Result<(Config, Vec<String>), ConfigError> {
        let cfg: Config = toml::from_str(src).map_err(|source| ConfigError::Toml {
            path: name.to_string(),
            source,
        })?;
        let warnings = cfg
            .unknown
            .keys()
            .map(|k| format!("{name}: unknown section or key `{k}` (ignored)"))
            .collect();
        Ok((cfg, warnings))
    }

    /// The first rule that applies to `name` and whose program is installed: the config
    /// file's rules first, then the built-in ones.
    /// A site by its id, case-insensitively.
    pub fn site(&self, id: &str) -> Option<&SiteCfg> {
        self.sites
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(id.trim()))
    }

    pub fn decompiler_for(&self, name: &str) -> Option<DecompileRule> {
        self.decompile
            .iter()
            .cloned()
            .chain(default_decompile_rules())
            .find(|rule| rule.matches(name) && command_exists(rule.program()))
    }

    /// The effective configuration as TOML (`scmd --dump-config`).
    pub fn dump(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_else(|e| format!("# cannot serialise: {e}\n"))
    }

    /// Apply a `--set key=value` override. Returns an error message when it does not apply.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "ui.theme" => self.ui.theme = value.to_string(),
            "ui.keymap" => self.ui.keymap = value.to_string(),
            "ui.language" => self.ui.language = value.to_string(),
            "ui.key_bar" => self.ui.key_bar = parse_bool(value)?,
            "ui.command_line" => self.ui.command_line = parse_bool(value)?,
            "ui.command_history" => {
                self.ui.command_history = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number"))?
            }
            "ui.status_bar" => self.ui.status_bar = parse_bool(value)?,
            "panel.view" => self.panel.view = value.to_string(),
            "panel.sort" => self.panel.sort = value.to_string(),
            "panel.reverse" => self.panel.reverse = parse_bool(value)?,
            "panel.show_hidden" => self.panel.show_hidden = parse_bool(value)?,
            "panel.dirs_first" => self.panel.dirs_first = parse_bool(value)?,
            "panel.selected_first" => self.panel.selected_first = parse_bool(value)?,
            "panel.natural_sort" => self.panel.natural_sort = parse_bool(value)?,
            "panel.quick_search" => {
                self.panel.quick_search = match value {
                    "alt" => QuickSearch::Alt,
                    "plain" => QuickSearch::Plain,
                    "off" => QuickSearch::Off,
                    other => {
                        return Err(format!(
                            "panel.quick_search: expected alt|plain|off, got `{other}`"
                        ))
                    }
                }
            }
            "net.known_hosts" => self.net.known_hosts = value.to_string(),
            "net.keepalive_secs" => {
                self.net.keepalive_secs = value
                    .parse()
                    .map_err(|_| format!("net.keepalive_secs: `{value}` is not a number"))?
            }
            "net.fetch_limit_mb" => {
                self.net.fetch_limit_mb = value
                    .parse()
                    .map_err(|_| format!("net.fetch_limit_mb: `{value}` is not a number"))?
            }
            "panel.allow_reverse_sort" => self.panel.allow_reverse_sort = parse_bool(value)?,
            "panel.column_titles" => self.panel.column_titles = parse_bool(value)?,
            "panel.status_line" => self.panel.status_line = parse_bool(value)?,
            "panel.scrollbar" => self.panel.scrollbar = parse_bool(value)?,
            "panel.free_space" => self.panel.free_space = parse_bool(value)?,
            "general.confirm_exit" => self.general.confirm_exit = parse_bool(value)?,
            "general.confirm_delete" => self.general.confirm_delete = parse_bool(value)?,
            "general.confirm_overwrite" => self.general.confirm_overwrite = parse_bool(value)?,
            other => return Err(format!("unknown setting `{other}`")),
        }
        Ok(())
    }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        other => Err(format!("expected a boolean, got `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_documented_ones() {
        let c = Config::default();
        assert_eq!(c.ui.theme, "commander");
        assert_eq!(c.ui.keymap, "far");
        assert_eq!(c.panel.view, "full");
        assert!(c.panel.dirs_first);
        // Nothing to assert about an update check: there is no such setting, on purpose.
        // What there is to assert is that asking about an overwrite is on by default, which
        // is the one of these three that costs data when it is wrong.
        assert!(c.general.confirm_overwrite);
        assert_eq!(c.panel.quick_search, QuickSearch::Alt);
    }

    #[test]
    fn partial_config_keeps_the_other_defaults() {
        let (c, warnings) = Config::parse(
            "t.toml",
            r#"
            [ui]
            theme = "matrix"
            [panel]
            show_hidden = true
            "#,
        )
        .unwrap();
        assert_eq!(c.ui.theme, "matrix");
        assert_eq!(c.ui.keymap, "far");
        assert!(c.panel.show_hidden);
        assert!(c.panel.dirs_first);
        assert!(warnings.is_empty());
    }

    #[test]
    fn unknown_sections_warn_but_load() {
        let (c, warnings) = Config::parse(
            "t.toml",
            r#"
            [ui]
            theme = "tron"
            [from_the_future]
            holograms = true
            "#,
        )
        .unwrap();
        assert_eq!(c.ui.theme, "tron");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("from_the_future"), "{:?}", warnings);
    }

    #[test]
    fn broken_toml_is_an_error_naming_the_file() {
        let err = Config::parse("broken.toml", "[ui\ntheme =").unwrap_err();
        assert!(err.to_string().starts_with("broken.toml:"), "{err}");
    }

    #[test]
    fn the_state_file_survives_a_round_trip_and_a_broken_file_is_not_fatal() {
        let dir = std::env::temp_dir().join(format!("scmd-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = Paths::discover(Some(&dir.join("scmd.toml")), None);
        let (state, problem) = State::load(&paths);
        assert!(problem.is_none(), "a missing state file is the first run");
        assert!(state.command_history.is_empty());

        let state = State {
            command_history: vec!["make".into(), "cargo test".into()],
            folder_history: vec!["/tmp".into()],
            left: "/home".into(),
            right: "/tmp".into(),
        };
        state.save(&paths).unwrap();
        let (back, problem) = State::load(&paths);
        assert!(problem.is_none());
        assert_eq!(back, state);

        std::fs::write(&paths.state_file, "this is not toml {{{").unwrap();
        let (back, problem) = State::load(&paths);
        assert_eq!(
            back,
            State::default(),
            "a broken file falls back to nothing"
        );
        assert!(problem.unwrap().contains("scmd-state.toml"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dump_round_trips() {
        let mut c = Config::default();
        c.ui.theme = "blade".into();
        c.panel.show_hidden = true;
        // What the session editor stores has to survive `Save setup` as well.
        c.sites.push(SiteCfg {
            id: "jump".into(),
            host: "jump.example.net".into(),
            port: Some(2222),
            auth: Some("key:~/.ssh/id_ed25519".into()),
            ..Default::default()
        });
        let (again, warnings) = Config::parse("dump.toml", &c.dump()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.ui.theme, "blade");
        assert!(again.panel.show_hidden);
        assert_eq!(again.sites.len(), 1);
        assert_eq!(again.sites[0].host, "jump.example.net");
        assert_eq!(again.sites[0].port, Some(2222));
        assert_eq!(again.sites[0].scheme, "sftp");
    }

    #[test]
    fn cli_overrides_are_validated() {
        let mut c = Config::default();
        c.set("ui.theme", "matrix").unwrap();
        c.set("panel.show_hidden", "yes").unwrap();
        assert_eq!(c.ui.theme, "matrix");
        assert!(c.panel.show_hidden);
        assert!(c.set("panel.show_hidden", "maybe").is_err());
        assert!(c.set("ui.colour_of_magic", "octarine").is_err());
    }

    #[test]
    fn an_explicit_config_file_wins_and_state_sits_beside_it() {
        let p = Paths::discover(Some(Path::new("/tmp/scconf/scmd.toml")), None);
        assert_eq!(p.config_file, PathBuf::from("/tmp/scconf/scmd.toml"));
        assert_eq!(p.state_file, PathBuf::from("/tmp/scconf/scmd-state.toml"));
        assert!(!p.portable);
    }

    #[test]
    fn an_explicit_plugins_dir_wins() {
        let p = Paths::discover(None, Some(Path::new("/opt/sc/plugins")));
        assert_eq!(p.plugins_dir, PathBuf::from("/opt/sc/plugins"));
    }

    #[test]
    fn discovery_produces_one_file_and_no_directory_of_our_own() {
        let p = Paths::discover(None, None);
        assert_eq!(
            p.config_file.file_name().unwrap(),
            std::ffi::OsStr::new("scmd.toml")
        );
        // `~/.config/scmd.toml`, not `~/.config/sc/config.toml`: nothing to create, nothing to
        // leave behind but one file.
        let parent = p.config_file.parent().unwrap();
        assert_ne!(parent.file_name(), Some(std::ffi::OsStr::new("scmd")));
        assert!(p.describe().contains("plugins"));
    }

    #[test]
    fn decompile_masks_match_extensions_and_names() {
        let rule = DecompileRule {
            name: "javap".into(),
            mask: "*.CLASS".into(),
            cmd: "javap -c {file}".into(),
        };
        assert!(rule.matches("Main.class"));
        assert!(rule.matches("main.CLASS"));
        assert!(!rule.matches("main.classx"));
        assert!(!rule.matches("classfile"));
        assert_eq!(rule.program(), "javap");

        let exact = DecompileRule {
            name: "x".into(),
            mask: "a.out".into(),
            cmd: "objdump -d {file}".into(),
        };
        assert!(exact.matches("a.out"));
        assert!(!exact.matches("b.out"));
    }

    #[test]
    fn a_configured_rule_wins_over_the_builtin_one() {
        let (cfg, warnings) = Config::parse(
            "scmd.toml",
            r#"
            [[decompile]]
            name = "my tool"
            match = "*.class"
            cmd = "sh -c true {file}"
            "#,
        )
        .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(cfg.decompile.len(), 1);
        // `sh` exists on any unix runner; on Windows this falls through to the built-ins.
        if command_exists("sh") {
            assert_eq!(cfg.decompiler_for("A.class").unwrap().name, "my tool");
        }
        assert!(cfg.decompiler_for("whatever.unknown").is_none());
    }

    #[test]
    fn command_existence_is_checked_before_a_tool_is_offered() {
        assert!(command_exists("cargo") || command_exists("rustc"));
        assert!(!command_exists("definitely-not-a-real-program-42"));
        assert!(!command_exists(""));
    }

    #[test]
    fn themes_and_keymaps_come_from_the_config_file() {
        let (cfg, warnings) = Config::parse(
            "scmd.toml",
            r##"
            [ui]
            theme = "mine"

            [[theme]]
            id = "mine"
            inherit = "matrix"
            [theme.colors]
            "selected.fg" = "#ff00ff"

            [[keymap]]
            name = "mine"
            inherit = "far"
            [[keymap.bind]]
            keys = ["F4"]
            cmd = "file.view"
            "##,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cfg.themes.len(), 1);
        assert_eq!(cfg.keymaps.len(), 1);
        assert_eq!(cfg.themes[0]["id"].as_str(), Some("mine"));
        assert_eq!(cfg.keymaps[0]["name"].as_str(), Some("mine"));
    }
}

/// What survives between sessions and is not configuration: the histories, and where the
/// panels were. A missing file is the normal first run; a broken one is reported and ignored,
/// because losing a history is never a reason to refuse to start.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Oldest first, as the command line walks it.
    #[serde(default)]
    pub command_history: Vec<String>,
    /// Where each panel had been, oldest first.
    #[serde(default)]
    pub folder_history: Vec<String>,
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub right: String,
}

impl State {
    /// Read the state file. The second value is a problem to report, not an error to die on.
    pub fn load(paths: &Paths) -> (State, Option<String>) {
        let file = &paths.state_file;
        if !file.exists() {
            return (State::default(), None);
        }
        match std::fs::read_to_string(file) {
            Ok(src) => match toml::from_str::<State>(&src) {
                Ok(state) => (state, None),
                Err(e) => (State::default(), Some(format!("{}: {e}", file.display()))),
            },
            Err(e) => (State::default(), Some(format!("{}: {e}", file.display()))),
        }
    }

    pub fn save(&self, paths: &Paths) -> std::io::Result<()> {
        if let Some(parent) = paths.state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)
            .unwrap_or_else(|e| format!("# cannot serialise the state: {e}\n"));
        std::fs::write(&paths.state_file, body)
    }
}
