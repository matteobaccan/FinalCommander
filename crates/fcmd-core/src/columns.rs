//! The user's own column sets: what a panel draws when it is in one of the custom views
//! (SPEC 02 §3).
//!
//! A built-in view is a list of columns somebody else decided on; a column set is the same
//! list with the deciding left to the user. Ten of them, `custom:0` to `custom:9`, on
//! `Ctrl+Shift+1` to `Ctrl+Shift+0`, each written in the configuration file:
//!
//! ```toml
//! [[column_set]]
//! id = 0
//! columns = [
//!   { type = "name",  width = "*" },
//!   { type = "size",  width = 10, align = "right", fmt = "bytes" },
//!   { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
//! ]
//! ```
//!
//! The same shape can be written in FAR's own spelling — the pair of strings every FAR user
//! already has in their configuration — which [`ColumnSet::from_far`] reads (§3b).
//!
//! Nothing here draws: the widths are arithmetic on a width, which is what makes them
//! testable without a terminal, and `fcmd-ui` puts the columns on the screen.

/// What a column holds.
///
/// Every one of these is either in an [`crate::Entry`] already or fetched by a worker for
/// the view that shows it — the owner, the group, how many names a file has, and what the
/// directory's `descript.ion` says about it. A set that asks for one of those four asks the
/// same worker the built-in view would, and a row the answer has not reached yet draws
/// blank, exactly as it does there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnKind {
    Name,
    /// The extension in a column of its own, as FAR's `X` is.
    Ext,
    Size,
    /// What the file takes on the device, which only a device that has said how big its
    /// clusters are can answer.
    Allocated,
    /// When the file was last written, made, and last read.
    Mtime,
    Btime,
    Atime,
    Attrs,
    Owner,
    Group,
    /// How many names the file has.
    Links,
    /// What the directory's own `descript.ion` says about it.
    Descr,
}

impl ColumnKind {
    /// Every kind, in the order the documentation lists them.
    pub const ALL: [ColumnKind; 12] = [
        ColumnKind::Name,
        ColumnKind::Ext,
        ColumnKind::Size,
        ColumnKind::Allocated,
        ColumnKind::Mtime,
        ColumnKind::Btime,
        ColumnKind::Atime,
        ColumnKind::Attrs,
        ColumnKind::Owner,
        ColumnKind::Group,
        ColumnKind::Links,
        ColumnKind::Descr,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ColumnKind::Name => "name",
            ColumnKind::Ext => "ext",
            ColumnKind::Size => "size",
            ColumnKind::Allocated => "allocated",
            ColumnKind::Mtime => "mtime",
            ColumnKind::Btime => "btime",
            ColumnKind::Atime => "atime",
            ColumnKind::Attrs => "attrs",
            ColumnKind::Owner => "owner",
            ColumnKind::Group => "group",
            ColumnKind::Links => "links",
            ColumnKind::Descr => "descr",
        }
    }

    /// The names the configuration file may use, FAR's own words among them.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "name" => Some(ColumnKind::Name),
            "ext" | "extension" => Some(ColumnKind::Ext),
            "size" => Some(ColumnKind::Size),
            "allocated" | "packed_size" | "packed" => Some(ColumnKind::Allocated),
            "mtime" | "modified" | "write" | "time" | "date" => Some(ColumnKind::Mtime),
            "btime" | "created" | "creation" => Some(ColumnKind::Btime),
            "atime" | "accessed" | "access" => Some(ColumnKind::Atime),
            "attrs" | "attr" | "attributes" | "perm" => Some(ColumnKind::Attrs),
            "owner" | "user" => Some(ColumnKind::Owner),
            "group" => Some(ColumnKind::Group),
            "links" => Some(ColumnKind::Links),
            "descr" | "description" | "diz" => Some(ColumnKind::Descr),
            _ => None,
        }
    }

    /// What the column takes when nothing says — FAR's automatic width. Zero means the
    /// column has no width of its own and takes what is left over: a name and a description
    /// are both as long as they happen to be.
    pub fn natural_width(self) -> u16 {
        match self {
            ColumnKind::Name | ColumnKind::Descr => 0,
            ColumnKind::Ext => 4,
            ColumnKind::Size | ColumnKind::Allocated => 9,
            ColumnKind::Mtime | ColumnKind::Btime | ColumnKind::Atime => 16,
            ColumnKind::Attrs => 5,
            ColumnKind::Owner | ColumnKind::Group => 12,
            ColumnKind::Links => 4,
        }
    }

    /// Does the column read right to left? A figure does; a word does not.
    pub fn right_aligned(self) -> bool {
        matches!(
            self,
            ColumnKind::Size | ColumnKind::Allocated | ColumnKind::Links | ColumnKind::Ext
        )
    }

    /// Does this column cost a lookup per file, which is to say does a set that asks for it
    /// put a worker to work?
    pub fn is_fetched(self) -> bool {
        matches!(
            self,
            ColumnKind::Owner | ColumnKind::Group | ColumnKind::Links | ColumnKind::Descr
        )
    }
}

/// How wide a column is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Width {
    /// FAR's `0`: the column's own natural width, or the space left over for a column that
    /// has none.
    #[default]
    Auto,
    Fixed(u16),
    /// A share of the panel's width.
    Percent(u16),
    /// `*`: what is left after every other column has had its share.
    Fill,
}

impl Width {
    /// `20`, `20%`, `*`, or `0` for automatic — the three spellings the configuration file
    /// and FAR's own width string have between them.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "*" {
            return Some(Width::Fill);
        }
        if let Some(num) = s.strip_suffix('%') {
            let p: u16 = num.trim().parse().ok()?;
            return (p > 0 && p <= 100).then_some(Width::Percent(p));
        }
        // FAR appends `+1` or `+2` to a date or time column to ask for a longer one. We take
        // the width and the caller says the rest is not applied.
        let digits = s.split('+').next().unwrap_or(s).trim();
        let n: u16 = digits.parse().ok()?;
        Some(if n == 0 { Width::Auto } else { Width::Fixed(n) })
    }
}

/// Which way the text sits in its column. `Default` is the kind's own answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Default,
    Left,
    Right,
}

impl Align {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "default" => Some(Align::Default),
            "left" => Some(Align::Left),
            "right" => Some(Align::Right),
            _ => None,
        }
    }
}

/// One column of a set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub kind: ColumnKind,
    pub width: Width,
    pub align: Align,
    /// `human` or `bytes` for a size; a `strftime` string for a date. `None` is each
    /// column's usual spelling.
    pub fmt: Option<String>,
}

impl Column {
    pub fn new(kind: ColumnKind) -> Self {
        Column {
            kind,
            width: Width::Auto,
            align: Align::Default,
            fmt: None,
        }
    }

    pub fn with_width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    /// Right-aligned unless the set says otherwise.
    pub fn right_aligned(&self) -> bool {
        match self.align {
            Align::Default => self.kind.right_aligned(),
            Align::Left => false,
            Align::Right => true,
        }
    }

    /// Does this column take what is left over rather than a width of its own?
    fn fills(&self) -> bool {
        self.width == Width::Fill || (self.width == Width::Auto && self.kind.natural_width() == 0)
    }
}

/// A view of the user's own: a list of columns and the number the keys know it by.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColumnSet {
    /// `0..=9`: `custom:3` is the set with id 3, on `Ctrl+Shift+4`.
    pub id: u8,
    pub columns: Vec<Column>,
}

/// Under this a name is not worth reading, so a column that would leave less than this is
/// dropped instead — which is the rule the built-in views follow too.
const NAME_MIN: u16 = 16;

impl ColumnSet {
    /// What the set draws in a panel this wide: the columns that fit, each with its width,
    /// as indices into [`Self::columns`].
    ///
    /// One blank column between each pair, as the built-in views have. A column the panel
    /// has no room for is dropped, from the right, and what is left over goes to the
    /// columns that asked to fill — or, when none did, to the first column, because a set
    /// of fixed widths in a wide panel should not leave a gap down the middle of the frame.
    pub fn layout(&self, inner_w: u16) -> Vec<(usize, u16)> {
        let mut out: Vec<(usize, u16)> = Vec::new();
        if self.columns.is_empty() || inner_w < 4 {
            return out;
        }
        let mut fills: Vec<usize> = Vec::new();
        let mut used = 0u16;
        for (i, col) in self.columns.iter().enumerate() {
            let gap = u16::from(!out.is_empty());
            let want = if col.fills() {
                1
            } else {
                match col.width {
                    Width::Fixed(n) => n,
                    Width::Percent(p) => (u32::from(inner_w) * u32::from(p) / 100) as u16,
                    _ => col.kind.natural_width(),
                }
                .max(1)
            };
            // No room for this one: the set stops here rather than drawing a column nobody
            // can read anything in.
            if used + gap + want > inner_w {
                break;
            }
            used += gap + want;
            if col.fills() {
                fills.push(out.len());
            }
            out.push((i, want));
        }
        let left = inner_w.saturating_sub(used);
        if !fills.is_empty() {
            let each = left / fills.len() as u16;
            let extra = left % fills.len() as u16;
            for (n, slot) in fills.iter().enumerate() {
                out[*slot].1 += each + if n == 0 { extra } else { 0 };
            }
        } else if left > 0 {
            if let Some(first) = out.first_mut() {
                first.1 += left;
            }
        }
        // A set whose name column came out unreadable is better off without the column that
        // crowded it: drop from the right until the name has room, as the detailed view does
        // with its extra dates.
        while out.len() > 1 {
            let name = out
                .iter()
                .find(|(i, _)| self.columns[*i].kind == ColumnKind::Name);
            match name {
                Some((_, w)) if *w < NAME_MIN => {}
                _ => break,
            }
            out.pop();
            let used: u16 = out
                .iter()
                .map(|(_, w)| w + 1)
                .sum::<u16>()
                .saturating_sub(1);
            let left = inner_w.saturating_sub(used);
            let fill = out
                .iter()
                .position(|(i, _)| self.columns[*i].fills())
                .or_else(|| {
                    out.iter()
                        .position(|(i, _)| self.columns[*i].kind == ColumnKind::Name)
                });
            if let Some(slot) = fill {
                out[slot].1 += left;
            }
        }
        out
    }

    /// Does the set ask for something a listing does not carry, which is to say does
    /// choosing it put a worker to work?
    pub fn asks(&self, kind: ColumnKind) -> bool {
        self.columns.iter().any(|c| c.kind == kind)
    }

    /// Read FAR's own pair of strings — `Column types` and `Column widths` from its panel
    /// mode dialog — into a set (SPEC 02 §3b).
    ///
    /// What we have no column for is reported rather than dropped in silence: a mode that
    /// came in missing its stream-size column should say so, not quietly draw something
    /// else.
    pub fn from_far(id: u8, types: &str, widths: &str) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut columns = Vec::new();
        let mut widths = widths
            .split(',')
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .map(|w| (w.to_string(), Width::parse(w)));
        for token in types.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            let upper = token.to_ascii_uppercase();
            // The two-letter types first, or `DM` reads as a write date with an `M` flag and
            // `LN` as an owner.
            let (kind, flags) = match () {
                _ if upper.starts_with("DM") => {
                    (Some(ColumnKind::Mtime), token.get(2..).unwrap_or(""))
                }
                _ if upper.starts_with("DC") => {
                    (Some(ColumnKind::Btime), token.get(2..).unwrap_or(""))
                }
                _ if upper.starts_with("DA") => {
                    (Some(ColumnKind::Atime), token.get(2..).unwrap_or(""))
                }
                _ if upper.starts_with("DE") => (None, token.get(2..).unwrap_or("")),
                _ if upper.starts_with("LN") => (Some(ColumnKind::Links), &token[2..]),
                _ => {
                    let rest = token.get(1..).unwrap_or("");
                    let kind = match upper.as_bytes().first() {
                        Some(b'N') => Some(ColumnKind::Name),
                        Some(b'X') => Some(ColumnKind::Ext),
                        Some(b'S') => Some(ColumnKind::Size),
                        Some(b'P') => Some(ColumnKind::Allocated),
                        Some(b'D') => Some(ColumnKind::Mtime),
                        Some(b'T') => Some(ColumnKind::Mtime),
                        Some(b'A') => Some(ColumnKind::Attrs),
                        Some(b'Z') => Some(ColumnKind::Descr),
                        Some(b'O') => Some(ColumnKind::Owner),
                        _ => None,
                    };
                    (kind, rest)
                }
            };
            let width = widths.next();
            let Some(kind) = kind else {
                warnings.push(format!(
                    "column `{token}`: there is no such column here, and it is left out"
                ));
                continue;
            };
            let mut column = Column::new(kind);
            // The date and the time of FAR's old pair are each half of what our one date
            // column draws, and the format is what says which half.
            if upper == "D" {
                column.fmt = Some("%Y-%m-%d".into());
            } else if upper == "T" {
                column.fmt = Some("%H:%M".into());
            }
            // The flags we have an answer for; the rest are reported.
            for flag in flags.chars().filter(|c| !c.is_whitespace()) {
                match (kind, flag.to_ascii_uppercase()) {
                    // Group the digits rather than abbreviate: FAR's `C`, our `bytes`.
                    (ColumnKind::Size | ColumnKind::Allocated, 'C') => {
                        column.fmt = Some("bytes".into())
                    }
                    // What FAR's name column does with what does not fit, which is what ours
                    // does with it in any case.
                    (ColumnKind::Name, 'R' | 'F' | 'M' | 'D' | 'O' | 'N') => {}
                    (ColumnKind::Ext, 'R') => {}
                    (ColumnKind::Owner, 'L') => {}
                    _ => warnings.push(format!(
                        "column `{token}`: the `{flag}` flag is not applied here"
                    )),
                }
            }
            match width {
                Some((_, Some(w))) => column.width = w,
                Some((text, None)) => warnings.push(format!(
                    "column `{token}`: `{text}` is not a width, so the column takes its own"
                )),
                None => {}
            }
            columns.push(column);
        }
        let extra = widths.count();
        if extra > 0 {
            warnings.push(format!(
                "there are {extra} more widths than there are columns, and they are ignored"
            ));
        }
        (ColumnSet { id, columns }, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(columns: Vec<Column>) -> ColumnSet {
        ColumnSet { id: 0, columns }
    }

    #[test]
    fn every_kind_round_trips_through_its_name() {
        for kind in ColumnKind::ALL {
            assert_eq!(ColumnKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(ColumnKind::parse("Description"), Some(ColumnKind::Descr));
        assert_eq!(ColumnKind::parse("stripes"), None);
    }

    #[test]
    fn a_width_is_a_number_a_share_or_what_is_left() {
        assert_eq!(Width::parse("12"), Some(Width::Fixed(12)));
        assert_eq!(Width::parse("30%"), Some(Width::Percent(30)));
        assert_eq!(Width::parse("*"), Some(Width::Fill));
        assert_eq!(Width::parse("0"), Some(Width::Auto));
        // FAR's precision suffix on a time column: the width is read, the rest is the
        // format string's business.
        assert_eq!(Width::parse("8+2"), Some(Width::Fixed(8)));
        assert_eq!(Width::parse("wide"), None);
        assert_eq!(Width::parse("200%"), None);
    }

    #[test]
    fn the_name_takes_what_the_other_columns_leave() {
        let s = set(vec![
            Column::new(ColumnKind::Name),
            Column::new(ColumnKind::Size).with_width(Width::Fixed(9)),
            Column::new(ColumnKind::Mtime).with_width(Width::Fixed(16)),
        ]);
        let out = s.layout(80);
        assert_eq!(out.len(), 3);
        // Three columns and the two blanks between them come to the panel's width.
        let total: u16 = out.iter().map(|(_, w)| w + 1).sum::<u16>() - 1;
        assert_eq!(total, 80);
        assert_eq!(out[1].1, 9);
        assert_eq!(out[2].1, 16);
        assert_eq!(out[0].1, 80 - 9 - 16 - 2);
    }

    #[test]
    fn two_columns_that_fill_share_what_is_left() {
        let s = set(vec![
            Column::new(ColumnKind::Name),
            Column::new(ColumnKind::Descr),
            Column::new(ColumnKind::Size).with_width(Width::Fixed(9)),
        ]);
        let out = s.layout(60);
        let room = 60 - 9 - 2;
        assert_eq!(out[0].1 + out[1].1, room);
        // Within one column of each other: the odd column goes to the first of them.
        assert!(out[0].1.abs_diff(out[1].1) <= 1);
    }

    #[test]
    fn a_share_of_the_width_is_a_share_of_the_panel() {
        let s = set(vec![
            Column::new(ColumnKind::Name),
            Column::new(ColumnKind::Descr).with_width(Width::Percent(25)),
        ]);
        let out = s.layout(100);
        assert_eq!(out[1].1, 25);
        assert_eq!(out[0].1, 100 - 25 - 1);
    }

    #[test]
    fn a_panel_too_narrow_drops_columns_from_the_right() {
        let s = set(vec![
            Column::new(ColumnKind::Name),
            Column::new(ColumnKind::Size).with_width(Width::Fixed(9)),
            Column::new(ColumnKind::Mtime).with_width(Width::Fixed(16)),
            Column::new(ColumnKind::Attrs).with_width(Width::Fixed(5)),
        ]);
        // Room for the name and one figure, and no more.
        let out = s.layout(30);
        assert!(out.len() < 4, "{out:?}");
        assert_eq!(self::kind_of(&s, &out[0]), ColumnKind::Name);
        assert!(out[0].1 >= 16, "the name stays readable: {out:?}");
        // And the widths still add up to the panel.
        let total: u16 = out.iter().map(|(_, w)| w + 1).sum::<u16>() - 1;
        assert_eq!(total, 30);
    }

    fn kind_of(s: &ColumnSet, cell: &(usize, u16)) -> ColumnKind {
        s.columns[cell.0].kind
    }

    #[test]
    fn a_set_of_fixed_widths_still_fills_the_frame() {
        let s = set(vec![
            Column::new(ColumnKind::Name).with_width(Width::Fixed(20)),
            Column::new(ColumnKind::Size).with_width(Width::Fixed(9)),
        ]);
        let out = s.layout(60);
        let total: u16 = out.iter().map(|(_, w)| w + 1).sum::<u16>() - 1;
        assert_eq!(total, 60, "the slack goes to the first column: {out:?}");
        assert_eq!(out[0].1, 60 - 9 - 1);
    }

    #[test]
    fn fars_own_strings_read_as_a_set() {
        let (s, warnings) = ColumnSet::from_far(2, "N,SC,DM", "0,9,16");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            s.columns.iter().map(|c| c.kind).collect::<Vec<_>>(),
            vec![ColumnKind::Name, ColumnKind::Size, ColumnKind::Mtime]
        );
        assert_eq!(s.columns[0].width, Width::Auto);
        assert_eq!(s.columns[1].width, Width::Fixed(9));
        // `C` is FAR's "group the digits", which is our `bytes`.
        assert_eq!(s.columns[1].fmt.as_deref(), Some("bytes"));
        assert_eq!(s.id, 2);
    }

    #[test]
    fn fars_date_and_time_are_two_halves_of_one_column() {
        let (s, warnings) = ColumnSet::from_far(0, "N,D,T", "0,10,5");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s.columns[1].kind, ColumnKind::Mtime);
        assert_eq!(s.columns[1].fmt.as_deref(), Some("%Y-%m-%d"));
        assert_eq!(s.columns[2].kind, ColumnKind::Mtime);
        assert_eq!(s.columns[2].fmt.as_deref(), Some("%H:%M"));
    }

    #[test]
    fn what_we_have_no_column_for_is_said_rather_than_dropped() {
        // `G` is the stream size and `DE` the change time: neither is a column here.
        let (s, warnings) = ColumnSet::from_far(0, "N,G,DE,LN", "0,8,14,4");
        assert_eq!(
            s.columns.iter().map(|c| c.kind).collect::<Vec<_>>(),
            vec![ColumnKind::Name, ColumnKind::Links]
        );
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("`G`")), "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("`DE`")), "{warnings:?}");
        // The width that went with a column nobody drew went with it.
        assert_eq!(s.columns[1].width, Width::Fixed(4));
    }

    #[test]
    fn the_columns_that_cost_a_lookup_are_the_four_the_workers_fill() {
        let fetched: Vec<ColumnKind> = ColumnKind::ALL
            .into_iter()
            .filter(|k| k.is_fetched())
            .collect();
        assert_eq!(
            fetched,
            vec![
                ColumnKind::Owner,
                ColumnKind::Group,
                ColumnKind::Links,
                ColumnKind::Descr
            ]
        );
    }
}
