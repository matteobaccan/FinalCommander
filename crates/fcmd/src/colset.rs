//! The user's own column sets, compiled once from the `[[column_set]]` blocks (SPEC 02 §3a).
//!
//! The shape is the one `[[highlight]]` and `[[sort_group]]` already use: a block in
//! `fcmd.toml` is turned into the core's own type at startup, and whatever could not be read
//! is a line on the startup screen rather than a surprise in a panel. A block that is thrown
//! out leaves the built-in view mode in place, which is the answer that still draws something.

use fcmd_config::{ColumnCfg, Config};
use fcmd_core::columns::{Align, Column, ColumnFlags, ColumnKind, ColumnSet, Width};

/// Every set that reads, in the order the file wrote them.
pub fn sets(cfg: &Config, warnings: &mut Vec<String>) -> Vec<ColumnSet> {
    let mut out: Vec<ColumnSet> = Vec::new();
    for (n, block) in cfg.column_sets.iter().enumerate() {
        let name = if block.name.is_empty() {
            format!("column set {} (id {})", n + 1, block.id)
        } else {
            block.name.clone()
        };
        if block.id > 9 {
            warnings.push(format!(
                "{name}: `id` is the digit of the view key, so it must be 0 to 9"
            ));
            continue;
        }
        // Two blocks on one key cannot both be drawn, and picking silently would make the
        // panel depend on the order of a file nobody re-reads.
        if out.iter().any(|set| set.id == block.id) {
            warnings.push(format!(
                "{name}: there is already a column set on `Ctrl+{}`, so this one is ignored",
                block.id
            ));
            continue;
        }
        // FAR's own pair of strings, for a mode somebody already has over there (§3b). The
        // list and the strings are two spellings of one thing, so a block that writes both is
        // a block whose author has not decided.
        if !block.far_types.trim().is_empty() {
            if !block.columns.is_empty() {
                warnings.push(format!(
                    "{name}: `columns` and `far_types` are two ways of saying the same thing; \
                     the strings are the ones used here"
                ));
            }
            let (mut set, problems) =
                ColumnSet::from_far(block.id, &block.far_types, &block.far_widths);
            set.flags = flags(block);
            warnings.extend(problems.into_iter().map(|p| format!("{name}: {p}")));
            if set.columns.is_empty() {
                warnings.push(format!(
                    "{name}: no column that could be read, so `Ctrl+{}` is the built-in view",
                    block.id
                ));
                continue;
            }
            out.push(set);
            continue;
        }
        let mut columns: Vec<Column> = Vec::new();
        for (i, col) in block.columns.iter().enumerate() {
            match column(col) {
                Ok(column) => columns.push(column),
                Err(problem) => warnings.push(format!("{name}: column {}: {problem}", i + 1)),
            }
        }
        if columns.is_empty() {
            warnings.push(format!(
                "{name}: no column that could be read, so `Ctrl+{}` is the built-in view",
                block.id
            ));
            continue;
        }
        out.push(ColumnSet {
            id: block.id,
            columns,
            flags: flags(block),
        });
    }
    out
}

/// The per-mode switches of the block, each left as it was before they existed when the
/// block does not say (SPEC 02 §3b).
fn flags(block: &fcmd_config::ColumnSetCfg) -> ColumnFlags {
    let d = ColumnFlags::default();
    ColumnFlags {
        fullscreen: block.fullscreen.unwrap_or(d.fullscreen),
        align_extensions: block.align_extensions.unwrap_or(d.align_extensions),
        align_folder_extensions: block
            .align_folder_extensions
            .unwrap_or(d.align_folder_extensions),
        folders_upper: block.folders_upper.unwrap_or(d.folders_upper),
        files_lower: block.files_lower.unwrap_or(d.files_lower),
        upper_to_lower: block.upper_to_lower.unwrap_or(d.upper_to_lower),
    }
}

/// The set a view's digit carries, if the user wrote one for it.
pub fn for_slot(sets: &[ColumnSet], slot: u8) -> Option<&ColumnSet> {
    sets.iter().find(|set| set.id == slot)
}

fn column(cfg: &ColumnCfg) -> Result<Column, String> {
    let kind = ColumnKind::parse(&cfg.kind)
        .ok_or_else(|| format!("`{}` is not a column anything can draw", cfg.kind))?;
    let width = match &cfg.width {
        None => Width::Auto,
        Some(toml::Value::Integer(n)) => match u16::try_from(*n) {
            Ok(0) => Width::Auto,
            Ok(n) => Width::Fixed(n),
            Err(_) => return Err(format!("`{n}` is not a width")),
        },
        Some(toml::Value::String(text)) => {
            Width::parse(text).ok_or_else(|| format!("`{text}` is not a width"))?
        }
        Some(other) => return Err(format!("`{other}` is not a width")),
    };
    let align =
        Align::parse(&cfg.align).ok_or_else(|| format!("`{}` is not left or right", cfg.align))?;
    Ok(Column {
        kind,
        width,
        align,
        fmt: cfg.fmt.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> (Vec<ColumnSet>, Vec<String>) {
        let (cfg, problems) = Config::parse("test.toml", src).expect("it parses");
        assert!(problems.is_empty(), "{problems:?}");
        let mut warnings = Vec::new();
        let sets = sets(&cfg, &mut warnings);
        (sets, warnings)
    }

    #[test]
    fn a_block_becomes_the_columns_of_the_view_its_digit_names() {
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 3
            name = "mine"
            columns = [
              { type = "name", width = "*" },
              { type = "size", width = 10, align = "right", fmt = "bytes" },
              { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
            ]
            "#,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(sets.len(), 1);
        let set = &sets[0];
        assert_eq!(set.id, 3);
        assert_eq!(
            set.columns.iter().map(|c| c.kind).collect::<Vec<_>>(),
            vec![ColumnKind::Name, ColumnKind::Size, ColumnKind::Mtime]
        );
        assert_eq!(set.columns[0].width, Width::Fill);
        assert_eq!(set.columns[1].width, Width::Fixed(10));
        assert!(set.columns[1].right_aligned());
        assert_eq!(set.columns[2].fmt.as_deref(), Some("%Y-%m-%d %H:%M"));
        // And it is found by the view's own digit, which is how the panel asks for it.
        assert!(for_slot(&sets, 3).is_some());
        assert!(for_slot(&sets, 4).is_none());
    }

    #[test]
    fn what_cannot_be_read_is_said_and_the_built_in_view_stays() {
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 2
            columns = [
              { type = "name" },
              { type = "stripes" },
              { type = "size", width = "wide" },
              { type = "ext", align = "middle" },
            ]

            [[column_set]]
            id = 11
            columns = [{ type = "name" }]

            [[column_set]]
            id = 4
            columns = [{ type = "nothing" }]
            "#,
        );
        // The first set keeps the one column that read; the three bad ones are named.
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].columns.len(), 1);
        assert_eq!(warnings.len(), 6, "{warnings:?}");
        assert!(
            warnings.iter().any(|w| w.contains("stripes")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("`wide`")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("middle")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("0 to 9")),
            "{warnings:?}"
        );
        // The set whose every column was nonsense leaves its key alone and says so.
        assert!(for_slot(&sets, 4).is_none());
        assert!(
            warnings.iter().any(|w| w.contains("built-in view")),
            "{warnings:?}"
        );
    }

    /// A FAR user's own panel mode, in the pair of strings FAR's dialog writes (§3b).
    #[test]
    fn fars_own_column_strings_are_a_set_as_well() {
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 6
            name = "from far"
            far_types = "N,SC,DM"
            far_widths = "0,9,16"
            "#,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(sets.len(), 1);
        assert_eq!(
            sets[0].columns.iter().map(|c| c.kind).collect::<Vec<_>>(),
            vec![ColumnKind::Name, ColumnKind::Size, ColumnKind::Mtime]
        );
        // `C` is FAR's "group the digits", which our size column calls `bytes`.
        assert_eq!(sets[0].columns[1].fmt.as_deref(), Some("bytes"));

        // And a column we have no answer for is named rather than quietly dropped.
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 7
            far_types = "N,G"
            far_widths = "0,8"
            "#,
        );
        assert_eq!(sets[0].columns.len(), 1);
        assert!(warnings.iter().any(|w| w.contains("`G`")), "{warnings:?}");
    }

    #[test]
    fn the_per_mode_switches_default_to_what_the_panel_always_drew() {
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 3
            columns = [{ type = "name" }]
            "#,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(sets[0].flags, ColumnFlags::default());
        assert!(
            sets[0].flags.align_extensions && !sets[0].flags.align_folder_extensions,
            "a file's extension lines up, a directory's does not"
        );

        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 4
            columns = [{ type = "name" }]
            fullscreen = true
            align_extensions = false
            folders_upper = true
            files_lower = true
            upper_to_lower = true
            "#,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        let flags = sets[0].flags;
        assert!(flags.fullscreen && flags.folders_upper && flags.files_lower);
        assert!(flags.upper_to_lower && !flags.align_extensions);

        // They ride on FAR's own pair of strings too: the switches are a different part of
        // that dialog, so one spelling of the columns must not cost the other.
        let (sets, _) = parse(
            r#"
            [[column_set]]
            id = 5
            far_types = "N,S"
            far_widths = "0,9"
            fullscreen = true
            "#,
        );
        assert!(sets[0].flags.fullscreen);
    }

    #[test]
    fn two_sets_on_one_key_is_a_mistake_worth_naming() {
        let (sets, warnings) = parse(
            r#"
            [[column_set]]
            id = 5
            name = "first"
            columns = [{ type = "name" }]

            [[column_set]]
            id = 5
            name = "second"
            columns = [{ type = "name" }, { type = "size" }]
            "#,
        );
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].columns.len(), 1, "the first one is the one kept");
        assert!(
            warnings.iter().any(|w| w.contains("second")),
            "{warnings:?}"
        );
    }
}
