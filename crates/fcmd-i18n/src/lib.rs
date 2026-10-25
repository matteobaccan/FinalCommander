//! Localisation: fluent catalogues compiled in, English as the always-present fallback.
//!
//! Shipped languages: English, Italian, German, French, Spanish, Russian, Chinese
//! (Simplified). See `docs/spec/07-config-theming.md` §5.

use std::borrow::Cow;

use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// `(tag, endonym)` for every language compiled into the binary.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("it", "Italiano"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("es", "Español"),
    ("ru", "Русский"),
    ("zh", "简体中文"),
];

const CATALOGUES: &[(&str, &str)] = &[
    ("en", include_str!("../i18n/en/fc.ftl")),
    ("it", include_str!("../i18n/it/fc.ftl")),
    ("de", include_str!("../i18n/de/fc.ftl")),
    ("fr", include_str!("../i18n/fr/fc.ftl")),
    ("es", include_str!("../i18n/es/fc.ftl")),
    ("ru", include_str!("../i18n/ru/fc.ftl")),
    ("zh", include_str!("../i18n/zh/fc.ftl")),
];

type Bundle = FluentBundle<FluentResource>;

pub struct I18n {
    tag: String,
    primary: Bundle,
    fallback: Option<Bundle>,
}

impl I18n {
    /// `tag` may be `auto`, a language tag (`it`), or a locale (`it-IT`, `zh-Hans-CN`).
    /// Unknown tags fall back to English instead of failing.
    pub fn new(tag: &str) -> Self {
        let wanted = resolve_tag(tag);
        let primary = bundle_for(&wanted).unwrap_or_else(|| bundle_for("en").expect("en builds"));
        let fallback = if wanted == "en" {
            None
        } else {
            bundle_for("en")
        };
        I18n {
            tag: wanted,
            primary,
            fallback,
        }
    }

    /// Follow the OS locale (`LC_ALL`/`LC_MESSAGES`/`LANG`, or `WINDOWS` user default).
    pub fn from_env() -> Self {
        Self::new("auto")
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    pub fn language_name(&self) -> &'static str {
        LANGUAGES
            .iter()
            .find(|(t, _)| *t == self.tag)
            .map(|(_, name)| *name)
            .unwrap_or("English")
    }

    /// Look up a message. A missing id returns the id itself, so a typo is visible in the UI
    /// instead of producing an empty label.
    pub fn t(&self, id: &str) -> String {
        self.format(id, None)
    }

    /// Look up a message with arguments: `t_args("msg-theme", &[("name", "matrix".into())])`.
    pub fn t_args(&self, id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let mut fargs = FluentArgs::new();
        for (k, v) in args {
            fargs.set(*k, v.clone());
        }
        self.format(id, Some(&fargs))
    }

    fn format(&self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        for bundle in [Some(&self.primary), self.fallback.as_ref()]
            .into_iter()
            .flatten()
        {
            if let Some(msg) = bundle.get_message(id) {
                if let Some(pattern) = msg.value() {
                    let mut errors = vec![];
                    let out = bundle.format_pattern(pattern, args, &mut errors);
                    if errors.is_empty() {
                        return out.into_owned();
                    }
                    // A broken pattern is a bug in our catalogue, not a reason to show nothing.
                    return match out {
                        Cow::Borrowed(s) => s.to_string(),
                        Cow::Owned(s) => s,
                    };
                }
            }
        }
        id.to_string()
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::new("en")
    }
}

fn bundle_for(tag: &str) -> Option<Bundle> {
    let src = CATALOGUES.iter().find(|(t, _)| *t == tag)?.1;
    let langid: LanguageIdentifier = tag.parse().ok()?;
    let resource = FluentResource::try_new(src.to_string()).ok()?;
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    // Bidi isolation marks would show up as stray characters in a terminal cell grid.
    bundle.set_use_isolating(false);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

fn resolve_tag(tag: &str) -> String {
    let raw = if tag.trim().eq_ignore_ascii_case("auto") || tag.trim().is_empty() {
        env_locale()
    } else {
        tag.to_string()
    };
    let lower = raw.trim().to_ascii_lowercase();
    // `it_IT.UTF-8`, `zh-Hans-CN`, `pt_BR` → the first subtag.
    let base = lower
        .split(['.', '@'])
        .next()
        .unwrap_or("en")
        .replace('_', "-");
    let primary = base.split('-').next().unwrap_or("en");
    if LANGUAGES.iter().any(|(t, _)| *t == primary) {
        primary.to_string()
    } else {
        "en".to_string()
    }
}

fn env_locale() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() && v != "C" && v != "POSIX" {
                return v;
            }
        }
    }
    "en".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalogue_parses_and_has_every_key() {
        let en = I18n::new("en");
        let ids: Vec<&str> = CATALOGUES[0]
            .1
            .lines()
            .filter(|l| !l.starts_with('#') && l.contains('='))
            .map(|l| l.split('=').next().unwrap().trim())
            .collect();
        assert!(ids.len() > 30, "English catalogue looks too small");

        for (tag, _) in LANGUAGES {
            let i18n = I18n::new(tag);
            assert_eq!(i18n.tag(), *tag);
            for id in &ids {
                let translated = i18n.t(id);
                assert_ne!(translated, *id, "{tag}: `{id}` is missing");
                assert!(!translated.is_empty(), "{tag}: `{id}` is empty");
            }
        }
        drop(en);
    }

    /// Every message in the catalogue is shown by something.
    ///
    /// The other way round — every id the code asks for exists — is the test above, and the
    /// two together are what keeps a translation honest: a message nobody shows is seven
    /// translations of nothing, and it is how a catalogue grows a tail of text that no longer
    /// matches the program. When this fails, either use the id or delete it from all seven.
    ///
    /// It reads the source tree, so an id built at run time rather than written as a literal
    /// would look unused. There are none today; if one appears, give it a literal somewhere
    /// (a `const`, a match arm) rather than loosening this.
    #[test]
    fn every_message_in_the_catalogue_is_shown_by_something() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("the workspace is two directories up")
            .to_path_buf();
        let mut source = String::new();
        let mut stack = vec![root.join("crates")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        source.push_str(&text);
                    }
                }
            }
        }
        assert!(
            source.len() > 100_000,
            "the source tree was not found; this test would pass for the wrong reason"
        );

        let catalogue = include_str!("../i18n/en/fc.ftl");
        let mut unused: Vec<&str> = Vec::new();
        for line in catalogue.lines() {
            let Some((id, _)) = line.split_once(" = ") else {
                continue;
            };
            if id.starts_with('#') || id.starts_with(' ') || id.is_empty() {
                continue;
            }
            if !source.contains(&format!("\"{id}\"")) {
                unused.push(id);
            }
        }
        assert!(
            unused.is_empty(),
            "messages nobody shows — use them or delete them from all seven catalogues:\n  {}",
            unused.join("\n  ")
        );
    }

    #[test]
    fn arguments_are_interpolated_without_bidi_marks() {
        let it = I18n::new("it");
        let s = it.t_args("msg-theme", &[("name", "matrix".into())]);
        assert_eq!(s, "tema: matrix");
        assert!(
            !s.contains('\u{2068}'),
            "bidi isolation marks leaked into the UI"
        );
    }

    #[test]
    fn locales_and_nonsense_resolve_sensibly() {
        assert_eq!(I18n::new("it_IT.UTF-8").tag(), "it");
        assert_eq!(I18n::new("zh-Hans-CN").tag(), "zh");
        assert_eq!(I18n::new("tlh").tag(), "en"); // Klingon is not shipped yet
        assert_eq!(I18n::new("").tag(), I18n::from_env().tag());
    }

    #[test]
    fn missing_ids_surface_as_themselves() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.t("no-such-message"), "no-such-message");
    }

    #[test]
    fn language_names_are_endonyms() {
        assert_eq!(I18n::new("ru").language_name(), "Русский");
        assert_eq!(I18n::new("zh").language_name(), "简体中文");
    }
}
