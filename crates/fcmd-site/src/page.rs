//! The page itself: one template, one stylesheet, no framework.
//!
//! There is no JavaScript on this site and nothing is fetched from anywhere else — no fonts,
//! no analytics, no content delivery network. A page is a file; it works offline, out of a
//! `file://` URL, and in a browser that was new ten years ago. The program is one executable
//! with everything in it, and a site that needed a build pipeline to say so would be arguing
//! against itself.

use crate::text::{Text, LANGUAGES};

/// Everything a page needs to be written.
pub struct Page<'a> {
    /// The file this page is written to, relative to the site root (`help/files.html`).
    pub path: String,
    /// What the browser's tab says, before the program's name.
    pub title: &'a str,
    pub lang: &'a Text,
    /// The body, already HTML.
    pub body: String,
    /// The same page in the other languages, as `(code, path)`. Empty for a page that exists
    /// only in English — the switcher then takes the reader to that language's home instead.
    pub translations: Vec<(&'static str, String)>,
}

impl Page<'_> {
    /// How deep this page sits, so a relative link from it can climb out.
    pub fn depth(&self) -> usize {
        self.path.matches('/').count()
    }

    pub fn render(&self) -> String {
        let up = "../".repeat(self.depth());
        let lang = self.lang;
        let suffix = suffix_of(lang.code);
        let nav = [
            (lang.nav_home, format!("index{suffix}.html")),
            (lang.nav_features, format!("features{suffix}.html")),
            (lang.nav_replaces, format!("replaces{suffix}.html")),
            // Out of the site and into the releases: that page is always the latest build,
            // where one of ours would name the archives of whichever version it was last
            // generated from and go stale the day after a release.
            (
                lang.nav_download,
                format!("{}/releases/latest", crate::md::REPO),
            ),
            (lang.nav_help, format!("help/index{suffix}.html")),
            (lang.nav_commanders, format!("commanders{suffix}.html")),
            (lang.nav_changelog, format!("changelog{suffix}.html")),
        ];
        let mut links = String::new();
        for (label, target) in nav {
            let current = target == self.path;
            // A link that leaves the site is absolute and must not be climbed out of.
            let prefix = if target.contains("://") {
                ""
            } else {
                up.as_str()
            };
            links.push_str(&format!(
                "<a href=\"{prefix}{target}\"{}>{}</a>",
                if current { " class=\"here\"" } else { "" },
                escape(label)
            ));
        }
        links.push_str(&format!(
            "<a class=\"source\" href=\"{}\">{}</a>",
            crate::md::REPO,
            escape(lang.nav_source)
        ));

        let mut switcher = String::new();
        for other in LANGUAGES {
            if other.code == lang.code {
                switcher.push_str(&format!("<span>{}</span>", escape(other.name)));
                continue;
            }
            // The same page in that language when there is one, and that language's home when
            // there is not: a reader who switches should land somewhere they can read.
            let target = self
                .translations
                .iter()
                .find(|(code, _)| *code == other.code)
                .map(|(_, path)| path.clone())
                .unwrap_or_else(|| format!("index{}.html", suffix_of(other.code)));
            switcher.push_str(&format!(
                "<a href=\"{up}{target}\" lang=\"{}\">{}</a>",
                other.code,
                escape(other.name)
            ));
        }

        format!(
            r#"<!doctype html>
<html lang="{code}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — FinalCommander</title>
<meta name="description" content="{tagline}">
<link rel="stylesheet" href="{up}style.css">
<link rel="icon" href="{up}assets/icon.svg" type="image/svg+xml">
</head>
<body>
<header>
<a class="wordmark" href="{up}index{suffix}.html"><img src="{up}assets/icon.svg" alt="" width="30" height="30">FinalCommander</a>
<input type="checkbox" id="burger" class="burger" aria-label="{menu}">
<label for="burger" class="burger-label"><span></span><span></span><span></span></label>
<nav>{links}</nav>
</header>
<main>
{body}
</main>
<footer>
<div class="languages">{switcher}</div>
<p><a href="{repo}">{source}</a> · MIT</p>
</footer>
</body>
</html>
"#,
            code = lang.code,
            title = escape(self.title),
            tagline = escape(lang.tagline),
            up = up,
            suffix = suffix,
            links = links,
            body = self.body,
            switcher = switcher,
            repo = crate::md::REPO,
            source = escape(lang.nav_source),
            menu = escape(lang.nav_menu),
        )
    }
}

/// `index.html` for English and `index.it.html` for the rest: the English page is the one a
/// bare URL lands on, so it carries no suffix.
pub fn suffix_of(code: &str) -> String {
    if code == "en" {
        String::new()
    } else {
        format!(".{code}")
    }
}

/// The five characters that have to be escaped in text that goes into a page.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// The stylesheet. Two palettes — the terminal's own colours, light and dark — chosen by the
/// reader's system rather than by a switch nobody finds.
pub const STYLE: &str = r#"/* FinalCommander's site: one file, no framework, no fetching. */
:root {
  --bg: #11131a;
  --fg: #d7dae0;
  --dim: #8b93a7;
  --accent: #55ffff;
  --frame: #2b3040;
  --code-bg: #1a1e29;
  --link: #8fd3ff;
}
@media (prefers-color-scheme: light) {
  :root {
    --bg: #fbfbfd;
    --fg: #1c2027;
    --dim: #5d6473;
    --accent: #006f84;
    --frame: #d8dce6;
    --code-bg: #f1f3f8;
    --link: #0b5fa4;
  }
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.6 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 1.5rem;
  align-items: center;
  padding: 0.9rem 1.25rem;
  border-bottom: 1px solid var(--frame);
}
.wordmark {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--fg);
  text-decoration: none;
}
/* The mark carries its own background and its own corners, so it needs nothing from the
   page and reads the same under either colour scheme. */
.wordmark img { width: 30px; height: 30px; border-radius: 6px; flex: none; }
nav { display: flex; flex-wrap: wrap; gap: 0.25rem 1rem; }
nav a { color: var(--dim); text-decoration: none; }
nav a:hover, nav a:focus { color: var(--accent); }
nav a.here { color: var(--fg); border-bottom: 2px solid var(--accent); }
nav a.source { margin-left: auto; }
/* On a wide screen there is no button: the links are the navigation, and the checkbox that
   would hold its state is not in the page at all. */
.burger, .burger-label { display: none; }
/* Below this the eight links wrap into three lines, and three lines of links above the page
   is a menu that has eaten the page. There is no JavaScript on this site — a test makes sure
   of it — so the open state is a checkbox, and the thing a reader presses is its label. */
@media (max-width: 48rem) {
  header { flex-wrap: wrap; gap: 0; }
  /* Off the screen rather than `display: none`, so that it stays in the tab order and a
     keyboard can open the menu. */
  .burger {
    display: block;
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    opacity: 0;
  }
  .burger-label {
    display: block;
    margin-left: auto;
    padding: 0.55rem;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
  }
  .burger-label span {
    display: block;
    width: 24px;
    height: 2px;
    border-radius: 2px;
    background: var(--fg);
  }
  .burger-label span + span { margin-top: 6px; }
  .burger:focus-visible + .burger-label { outline: 2px solid var(--accent); outline-offset: 2px; }
  header nav { display: none; flex-direction: column; width: 100%; gap: 0; margin-top: 0.6rem; }
  .burger:checked ~ nav { display: flex; }
  /* A row to press rather than a word to hit: each link is as tall as a finger and the
     separators say where one ends. */
  header nav a { padding: 0.7rem 0.1rem; border-top: 1px solid var(--frame); }
  header nav a.here { border-bottom: none; border-left: 3px solid var(--accent); padding-left: 0.6rem; }
  header nav a.source { margin-left: 0; }
}
main { max-width: 56rem; margin: 0 auto; padding: 1.5rem 1.25rem 3rem; }
a { color: var(--link); }
h1, h2, h3, h4 { line-height: 1.25; }
h1 { font-size: 2rem; margin: 1.5rem 0 0.5rem; }
h2 { font-size: 1.4rem; margin-top: 2.5rem; border-bottom: 1px solid var(--frame); padding-bottom: 0.3rem; }
h3 { font-size: 1.1rem; margin-top: 1.75rem; }
.tagline { font-size: 1.3rem; color: var(--accent); margin: 0.25rem 0 1rem; }
.pitch { font-size: 1.05rem; }
.note { color: var(--dim); font-size: 0.9rem; font-style: italic; }
/* What the site asks of a reader. Set apart because it is the one sentence here that asks for
   something rather than describing something. */
.ideas { border-left: 3px solid var(--accent); padding: 0.6rem 0 0.6rem 0.9rem; margin: 1.4rem 0; }
code, pre, kbd {
  font-family: ui-monospace, SFMono-Regular, "Cascadia Mono", Menlo, Consolas, monospace;
  font-size: 0.92em;
}
code { background: var(--code-bg); padding: 0.1em 0.35em; border-radius: 3px; }
pre {
  background: var(--code-bg);
  border: 1px solid var(--frame);
  border-radius: 6px;
  padding: 0.85rem 1rem;
  overflow-x: auto;
}
pre code { background: none; padding: 0; }
table { border-collapse: collapse; width: 100%; display: block; overflow-x: auto; }
th, td { border: 1px solid var(--frame); padding: 0.4rem 0.6rem; text-align: left; vertical-align: top; }
th { background: var(--code-bg); }
img, svg { max-width: 100%; height: auto; }
figure { margin: 1.5rem 0; }
figcaption { color: var(--dim); font-size: 0.9rem; }
.shots { display: grid; grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr)); gap: 1rem; }
.shots figure { margin: 0; }
.buttons { display: flex; flex-wrap: wrap; gap: 0.75rem; margin: 1.5rem 0; }
.buttons a {
  display: inline-block;
  padding: 0.6rem 1.1rem;
  border: 1px solid var(--accent);
  border-radius: 6px;
  color: var(--accent);
  text-decoration: none;
  font-weight: 600;
}
.buttons a:hover, .buttons a:focus { background: var(--accent); color: var(--bg); }
.pages { columns: 2 16rem; gap: 1.5rem; }
footer {
  border-top: 1px solid var(--frame);
  padding: 1.25rem;
  color: var(--dim);
  font-size: 0.9rem;
  text-align: center;
}
.languages { display: flex; flex-wrap: wrap; gap: 0.5rem 1rem; justify-content: center; margin-bottom: 0.5rem; }
.languages span { color: var(--fg); }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LANGUAGES;

    fn page(path: &str, code: &str) -> Page<'static> {
        Page {
            path: path.to_string(),
            title: "Title",
            lang: LANGUAGES
                .iter()
                .find(|t| t.code == code)
                .expect("the language"),
            body: "<p>body</p>".to_string(),
            translations: Vec::new(),
        }
    }

    #[test]
    fn a_page_is_one_html_file_with_nothing_fetched_from_anywhere() {
        let html = page("index.html", "en").render();
        assert!(html.starts_with("<!doctype html>"), "{html}");
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<p>body</p>"));
        // No version anywhere on a page: the site is built from the documents and not from a
        // release, so nothing on it goes stale the day one is cut.
        assert!(
            !html.contains("0.2"),
            "a version number crept back in: {html}"
        );
        // The promise the site makes about itself.
        assert!(!html.contains("<script"), "there is no JavaScript here");
        assert!(!html.contains("//fonts."), "and no fonts from elsewhere");
        assert!(
            !html.to_lowercase().contains("http://"),
            "and nothing over plain HTTP"
        );
    }

    /// On a phone the eight links wrapped into three lines above every page. They fold into a
    /// button now, and the mark sits where a reader looks for it.
    #[test]
    fn the_navigation_folds_into_a_button_on_a_narrow_screen() {
        let html = page("index.html", "en").render();
        // The mark, beside the name rather than instead of it, and with no alternative text:
        // the word next to it already says what it is, and a reader using a screen reader
        // should hear the name once.
        assert!(
            html.contains(r#"<img src="assets/icon.svg" alt="" width="30" height="30">"#),
            "{html}"
        );
        // The button and what it opens. The state is a checkbox because there is no
        // JavaScript here to hold it — and the checkbox carries the name, in the page's own
        // language, because the label has three lines in it rather than a word.
        assert!(
            html.contains(
                r#"<input type="checkbox" id="burger" class="burger" aria-label="Menu">"#
            ),
            "{html}"
        );
        assert!(
            html.contains(r#"<label for="burger" class="burger-label">"#),
            "{html}"
        );
        assert!(!html.contains("<script"), "and still no JavaScript");

        // Every language names the button in its own words.
        for lang in LANGUAGES {
            let html = page("index.html", lang.code).render();
            assert!(
                html.contains(&format!(r#"aria-label="{}""#, lang.nav_menu)),
                "{} does not name its menu button: {html}",
                lang.code
            );
        }

        // And the stylesheet is what makes it a button rather than a list: the links are
        // hidden until the checkbox is ticked, and only below the width where they wrapped.
        assert!(STYLE.contains("@media (max-width: 48rem)"), "{STYLE}");
        assert!(
            STYLE.contains(".burger:checked ~ nav { display: flex; }"),
            "{STYLE}"
        );
        assert!(
            STYLE.contains(".burger, .burger-label { display: none; }"),
            "the button has to be gone on a wide screen: {STYLE}"
        );
    }

    #[test]
    fn a_page_in_a_directory_climbs_out_for_the_stylesheet_and_the_navigation() {
        let html = page("help/files.html", "en").render();
        assert!(html.contains(r#"href="../style.css""#), "{html}");
        assert!(html.contains(r#"href="../index.html""#), "{html}");
        // But a link that leaves the site is absolute and is not climbed out of.
        assert!(
            html.contains(&format!(r#"href="{}/releases/latest""#, crate::md::REPO)),
            "{html}"
        );
        assert_eq!(page("help/files.html", "en").depth(), 1);
        assert_eq!(page("index.html", "en").depth(), 0);
    }

    #[test]
    fn the_language_a_page_is_in_is_the_one_not_offered_in_the_switcher() {
        let html = page("index.it.html", "it").render();
        assert!(html.contains("<html lang=\"it\">"), "{html}");
        // Italian is the page, so it is text; the other six are links.
        assert!(html.contains("<span>Italiano</span>"), "{html}");
        assert!(
            html.contains(r#"<a href="index.html" lang="en">English</a>"#),
            "{html}"
        );
        assert!(html.contains(r#"lang="zh""#), "{html}");
        // Its own navigation stays in Italian.
        assert!(html.contains("Scarica"), "{html}");
    }

    #[test]
    fn a_page_with_no_translation_sends_the_reader_to_that_languages_home() {
        // The changelog exists once, in English. Switching to German from it should land on
        // the German home page rather than on nothing.
        let html = page("changelog.html", "en").render();
        assert!(
            html.contains(r#"<a href="index.de.html" lang="de">"#),
            "{html}"
        );
    }

    #[test]
    fn the_current_page_is_marked_in_the_navigation() {
        let html = page("features.html", "en").render();
        assert!(html.contains(r#"class="here">What works<"#), "{html}");
    }

    #[test]
    fn text_that_goes_into_a_page_is_escaped() {
        assert_eq!(escape("a & b"), "a &amp; b");
        assert_eq!(escape("<b>"), "&lt;b&gt;");
        assert_eq!(
            escape(r#"say "x" & 'y'"#),
            "say &quot;x&quot; &amp; &#39;y&#39;"
        );
        assert_eq!(escape("nothing"), "nothing");
    }

    #[test]
    fn english_has_no_suffix_and_every_other_language_does() {
        assert_eq!(suffix_of("en"), "");
        assert_eq!(suffix_of("it"), ".it");
        assert_eq!(suffix_of("zh"), ".zh");
    }
}
