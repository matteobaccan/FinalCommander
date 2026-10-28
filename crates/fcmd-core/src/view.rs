/// How a panel renders its rows — FAR's own list, in FAR's own order, which is the order the
/// `Ctrl+1..8` keys are in (see `docs/spec/02-panels.md` §3).
///
/// All ten of FAR's, on FAR's own numbers: the three that needed data a listing does not
/// carry — descriptions, links, and the alternative full view — are here too, each fetched
/// by a worker while it is the one on screen.
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
    /// Name and description, from the directory's own `descript.ion` (SPEC 02 §9).
    Descriptions,
    /// Name, size, write time and description: FAR's long descriptions.
    LongDescriptions,
    /// Name, owner, group, size. The owner is not part of a listing — it costs a lookup per
    /// file, and on Windows a call into the security API — so it is fetched only while this
    /// mode is the one on screen (SPEC 02 §3).
    Owners,
    /// Name, how many names the file has, size, time. The count costs a lookup per row — on
    /// Windows a file has to be opened for it — so it is fetched like the owners are.
    Links,
    /// The full view with the sizes written out in full, digits grouped: FAR's alternative
    /// full view, for when the difference between 1.2M and 1.9M is the thing one is after.
    AltFull,
}

impl ViewMode {
    /// Every mode, in the order the keys are in.
    pub const ALL: [ViewMode; 10] = [
        ViewMode::Brief,
        ViewMode::Medium,
        ViewMode::Full,
        ViewMode::Wide,
        ViewMode::Detail,
        ViewMode::Descriptions,
        ViewMode::LongDescriptions,
        ViewMode::Owners,
        ViewMode::Links,
        ViewMode::AltFull,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Brief => "brief",
            ViewMode::Medium => "medium",
            ViewMode::Full => "full",
            ViewMode::Wide => "wide",
            ViewMode::Detail => "detail",
            ViewMode::Descriptions => "descriptions",
            ViewMode::LongDescriptions => "long-descriptions",
            ViewMode::Owners => "owners",
            ViewMode::Links => "links",
            ViewMode::AltFull => "alt-full",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "brief" => Some(ViewMode::Brief),
            "medium" => Some(ViewMode::Medium),
            "full" => Some(ViewMode::Full),
            "wide" => Some(ViewMode::Wide),
            "detail" | "detailed" => Some(ViewMode::Detail),
            "descriptions" | "descr" => Some(ViewMode::Descriptions),
            "long-descriptions" | "long_descriptions" | "longdescr" => {
                Some(ViewMode::LongDescriptions)
            }
            "owners" | "owner" => Some(ViewMode::Owners),
            "links" | "link" => Some(ViewMode::Links),
            "alt-full" | "alt_full" | "altfull" => Some(ViewMode::AltFull),
            _ => None,
        }
    }

    /// How many columns of rows this mode draws. More than one means the rows run down a
    /// column and then across, as they do in FAR.
    pub fn columns(self) -> usize {
        match self {
            ViewMode::Brief => 3,
            ViewMode::Medium | ViewMode::Wide => 2,
            ViewMode::Full
            | ViewMode::Detail
            | ViewMode::Owners
            | ViewMode::Descriptions
            | ViewMode::LongDescriptions
            | ViewMode::Links
            | ViewMode::AltFull => 1,
        }
    }

    /// Does a row show the size beside the name?
    pub fn shows_size(self) -> bool {
        !matches!(
            self,
            ViewMode::Brief | ViewMode::Medium | ViewMode::Descriptions
        )
    }

    /// Does this view show what the directory's `descript.ion` says about a row? The file is
    /// read only while a view that shows it is on screen, as the owners are.
    pub fn shows_description(self) -> bool {
        matches!(self, ViewMode::Descriptions | ViewMode::LongDescriptions)
    }

    /// Does a row say how many names the file has? Counted only while this view is the one
    /// on screen, for the same reason the owners are.
    pub fn shows_links(self) -> bool {
        self == ViewMode::Links
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
        assert_eq!(ViewMode::parse("stripes"), None);
    }

    #[test]
    fn the_columns_are_fars() {
        assert_eq!(ViewMode::Brief.columns(), 3);
        assert_eq!(ViewMode::Medium.columns(), 2);
        assert_eq!(ViewMode::Wide.columns(), 2);
        assert_eq!(ViewMode::Full.columns(), 1);
        assert_eq!(ViewMode::Owners.columns(), 1);
        assert_eq!(ViewMode::Descriptions.columns(), 1);
        // The two that read the directory's own `descript.ion`, and nothing else does.
        assert!(
            ViewMode::Descriptions.shows_description()
                && ViewMode::LongDescriptions.shows_description()
        );
        assert!(!ViewMode::Full.shows_description());
        assert!(ViewMode::Wide.shows_size() && !ViewMode::Brief.shows_size());
    }
}
