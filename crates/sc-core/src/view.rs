/// How a panel renders its rows. M1 implements `Brief` and `Full`; the rest exist so that the
/// keymap and the config can already name them (see `docs/spec/02-panels.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    /// Names only, several columns.
    Brief,
    /// Name, size, modification time.
    Full,
    /// Name, size, time, attributes, owner.
    Detail,
    /// Names only, as wide as the panel.
    Wide,
}

impl ViewMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Brief => "brief",
            ViewMode::Full => "full",
            ViewMode::Detail => "detail",
            ViewMode::Wide => "wide",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "brief" => Some(ViewMode::Brief),
            "full" => Some(ViewMode::Full),
            "detail" | "detailed" => Some(ViewMode::Detail),
            "wide" => Some(ViewMode::Wide),
            _ => None,
        }
    }

    /// Implemented in this build? The UI says so instead of silently doing nothing.
    pub fn is_implemented(self) -> bool {
        matches!(self, ViewMode::Brief | ViewMode::Full)
    }
}
