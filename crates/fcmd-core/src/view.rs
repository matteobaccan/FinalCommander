/// How a panel renders its rows — FAR's own list, in FAR's own order, which is the order the
/// `Ctrl+1..8` keys are in (see `docs/spec/02-panels.md` §3).
///
/// The three that need data we do not collect yet — descriptions, links, and the alternative
/// full view with its thousands separators — are lines in `docs/BACKLOG.md` rather than
/// variants that would say "not implemented".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    /// Names only, three columns.
    Brief,
    /// Names only, two columns.
    Medium,
    /// Name, size, modification time.
    Full,
    /// Name and size, two columns.
    Wide,
    /// Name, size, time, attributes.
    Detail,
    /// Name, owner, group, size. The owner is not part of a listing — it costs a lookup per
    /// file, and on Windows a call into the security API — so it is fetched only while this
    /// mode is the one on screen (SPEC 02 §3).
    Owners,
}

impl ViewMode {
    /// Every mode, in the order the keys are in.
    pub const ALL: [ViewMode; 6] = [
        ViewMode::Brief,
        ViewMode::Medium,
        ViewMode::Full,
        ViewMode::Wide,
        ViewMode::Detail,
        ViewMode::Owners,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Brief => "brief",
            ViewMode::Medium => "medium",
            ViewMode::Full => "full",
            ViewMode::Wide => "wide",
            ViewMode::Detail => "detail",
            ViewMode::Owners => "owners",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "brief" => Some(ViewMode::Brief),
            "medium" => Some(ViewMode::Medium),
            "full" => Some(ViewMode::Full),
            "wide" => Some(ViewMode::Wide),
            "detail" | "detailed" => Some(ViewMode::Detail),
            "owners" | "owner" => Some(ViewMode::Owners),
            _ => None,
        }
    }

    /// How many columns of rows this mode draws. More than one means the rows run down a
    /// column and then across, as they do in FAR.
    pub fn columns(self) -> usize {
        match self {
            ViewMode::Brief => 3,
            ViewMode::Medium | ViewMode::Wide => 2,
            ViewMode::Full | ViewMode::Detail | ViewMode::Owners => 1,
        }
    }

    /// Does a row show the size beside the name?
    pub fn shows_size(self) -> bool {
        !matches!(self, ViewMode::Brief | ViewMode::Medium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_round_trips_through_its_name() {
        for mode in ViewMode::ALL {
            assert_eq!(ViewMode::parse(mode.as_str()), Some(mode));
        }
        // The spelling FAR's own dialog uses, and one that is not a mode at all.
        assert_eq!(ViewMode::parse("Detailed"), Some(ViewMode::Detail));
        assert_eq!(ViewMode::parse("descriptions"), None);
    }

    #[test]
    fn the_columns_are_fars() {
        assert_eq!(ViewMode::Brief.columns(), 3);
        assert_eq!(ViewMode::Medium.columns(), 2);
        assert_eq!(ViewMode::Wide.columns(), 2);
        assert_eq!(ViewMode::Full.columns(), 1);
        assert_eq!(ViewMode::Owners.columns(), 1);
        assert!(ViewMode::Wide.shows_size() && !ViewMode::Brief.shows_size());
    }
}
