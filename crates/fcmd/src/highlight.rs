//! The `[[highlight]]` groups: the configuration's blocks compiled into the rules the panel
//! paints with, and the dialog that edits them (SPEC 02 §5, SPEC 10 §3.11).
//!
//! The compiling lives here rather than in `main.rs` because two places need it: the startup,
//! which turns the file into rules once, and the dialog, which turns an edited block into a
//! rule again every time a group is changed. One function, so that what the dialog accepts
//! and what the file accepts cannot drift apart.

use fcmd_config::Config;

/// The `[[highlight]]` groups of the configuration, compiled into the rules the panel asks.
///
/// A group that does not parse is a warning and is left out rather than a reason to refuse to
/// start: the file still lists, and the colour it would have had is the one it had before.
/// The message names the group, which is why a group is worth naming.
pub fn groups(cfg: &Config, warnings: &mut Vec<String>) -> fcmd_ui::Highlight {
    let groups = fcmd_core::filter::default_groups();
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
    fcmd_ui::Highlight::with_rules(rules)
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
