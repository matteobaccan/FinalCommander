//! The `[[highlight]]` groups: the configuration's blocks compiled into the rules the panel
//! paints with, and the dialog that edits them (SPEC 02 §5, SPEC 10 §3.11).
//!
//! The compiling lives here rather than in `main.rs` because two places need it: the startup,
//! which turns the file into rules once, and the dialog, which turns an edited block into a
//! rule again every time a group is changed. One function, so that what the dialog accepts
//! and what the file accepts cannot drift apart.

use fcmd_config::Config;

/// The `[[sort_group]]` blocks of the configuration, compiled into the bands the sort cuts a
/// listing into (SPEC 02 §4).
///
/// Here beside the colouring groups because it is the same job on the same file: a list of
/// masks, in the order they were written, each named so that a block which does not parse can
/// be named back. Such a block is a warning and is left out — a listing that refuses to sort
/// over a typo in one mask would be a worse answer than one band missing.
pub fn sort_groups(cfg: &Config, warnings: &mut Vec<String>) -> fcmd_core::sort::SortGroups {
    use fcmd_core::sort::{SortGroup, SortGroups};

    let predefined = cfg.mask_groups();
    let mut bands = Vec::new();
    for (n, g) in cfg.sort_groups.iter().enumerate() {
        let name = if g.name.is_empty() {
            format!("sort group {}", n + 1)
        } else {
            g.name.clone()
        };
        if g.mask.trim().is_empty() {
            warnings.push(format!("{name}: no mask, so it would hold nothing"));
            continue;
        }
        let masks = match fcmd_core::filter::MaskSet::parse_with(&g.mask, &predefined, false) {
            Ok(masks) => masks,
            Err(e) => {
                warnings.push(format!("{name}: {e}"));
                continue;
            }
        };
        let back = match g.place.trim().to_ascii_lowercase().as_str() {
            "" | "front" | "first" | "before" => false,
            "back" | "last" | "after" => true,
            other => {
                warnings.push(format!("{name}: `{other}` is not front or back"));
                false
            }
        };
        bands.push(SortGroup::new(name, masks, back));
    }
    SortGroups::new(bands)
}

/// The `[[highlight]]` groups of the configuration, compiled into the rules the panel asks.
///
/// A group that does not parse is a warning and is left out rather than a reason to refuse to
/// start: the file still lists, and the colour it would have had is the one it had before.
/// The message names the group, which is why a group is worth naming.
pub fn groups(cfg: &Config, warnings: &mut Vec<String>) -> fcmd_ui::Highlight {
    let groups = cfg.mask_groups();
    let mut rules = Vec::new();
    for (n, h) in cfg.highlight.iter().enumerate() {
        let name = if h.name.is_empty() {
            format!("#{}", n + 1)
        } else {
            h.name.clone()
        };
        match rule(h, &groups, &name) {
            Ok(rule) => rules.push(rule),
            Err(problem) => warnings.push(format!("highlight group `{name}`: {problem}")),
        }
    }
    fcmd_ui::Highlight::with_rules_in(rules, &groups)
}

/// One `[[highlight]]` block as a rule: every condition it names, parsed the way the filter
/// dialog parses the same words, and the two inks.
pub fn rule(
    h: &fcmd_config::HighlightCfg,
    groups: &fcmd_core::filter::Groups,
    name: &str,
) -> Result<fcmd_ui::highlight::Rule, String> {
    use fcmd_core::filter::{
        AttrConditions, DateCondition, DateRange, Filter, KindFilter, MaskSet, SizeRange, TimeField,
    };
    let mut filter = Filter::named(name);
    if !h.mask.is_empty() {
        // Case-insensitive, as the built-in groups are: `.ZIP` is an archive anywhere.
        filter.masks =
            Some(MaskSet::parse_with(&h.mask, groups, false).map_err(|e| e.to_string())?);
    }
    filter.kind =
        KindFilter::parse(&h.kind).ok_or_else(|| format!("`{}` is not a kind", h.kind))?;
    if !h.size.is_empty() {
        filter.size = Some(SizeRange::parse(&h.size).map_err(|e| e.to_string())?);
    }
    if !h.age.is_empty() {
        let field = TimeField::parse(&h.date_field)
            .ok_or_else(|| format!("`{}` is not a timestamp", h.date_field))?;
        let range = DateRange::parse_age(&h.age).map_err(|e| e.to_string())?;
        filter.date = Some(DateCondition::new(field, range));
    }
    filter.attrs = AttrConditions::parse(&h.attrs).map_err(|e| e.to_string())?;
    if filter.is_empty() {
        return Err("no condition, so it would paint every row".to_string());
    }
    Ok(fcmd_ui::highlight::Rule {
        name: name.to_string(),
        filter,
        fg: (!h.fg.is_empty()).then(|| fcmd_ui::highlight::Ink::parse(&h.fg)),
        bg: (!h.bg.is_empty()).then(|| fcmd_ui::highlight::Ink::parse(&h.bg)),
    })
}
