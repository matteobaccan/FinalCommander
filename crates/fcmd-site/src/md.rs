//! The repository's Markdown, as the site's HTML.
//!
//! Three jobs, and the third is the one with the decisions in it:
//!
//! 1. render a document ([`to_html`]);
//! 2. take one section out of a document, by its heading ([`section`]) — the landing page is
//!    built from the README's own words, and the README is not a web page;
//! 3. point the links at the pages this site actually has ([`rewrite_links`]). A link to
//!    `docs/help/files.md` in the repository has to become `help/files.html` here, and a link
//!    to a document the site does not publish has to become a link into the repository on
//!    GitHub rather than a dead one.

use pulldown_cmark::{html, Options, Parser};

/// Where the repository lives, for the links this site does not publish itself.
pub const REPO: &str = "https://github.com/matteobaccan/FinalCommander";

/// Markdown to HTML, with the extensions the repository's documents actually use.
pub fn to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    // Tables are everywhere in these documents — the feature inventory is one big table —
    // and strikethrough is how the backlog marks what has shipped.
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    // Smart quotes are left off on purpose: these documents are full of code, paths and
    // `--flags`, and turning an apostrophe in one of them into a curly quote would be a
    // document that no longer says what it meant.
    let parser = Parser::new_ext(markdown, options);
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, parser);
    out
}

/// The part of `markdown` under the heading whose text is `heading`, up to the next heading of
/// the same level or higher.
///
/// The heading itself is not included: the page puts its own title on. `None` when there is no
/// such heading, which a caller should treat as a mistake in the generator rather than paper
/// over — a section that has been renamed is a page that would silently go blank.
pub fn section(markdown: &str, heading: &str) -> Option<String> {
    let wanted = heading.trim();
    let mut lines = markdown.lines();
    let mut level = 0usize;
    // Find it.
    for line in lines.by_ref() {
        if let Some((hashes, title)) = split_heading(line) {
            if title == wanted {
                level = hashes;
                break;
            }
        }
    }
    if level == 0 {
        return None;
    }
    // Everything up to the next heading that is not *under* it.
    let mut out = String::new();
    for line in lines {
        if let Some((hashes, _)) = split_heading(line) {
            if hashes <= level {
                break;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    Some(out.trim().to_string())
}

/// `## Title` → `(2, "Title")`, and `None` for anything else. A `#` inside a fenced block is
/// not a heading, but these documents put no `#` at the start of a line inside one, and the
/// sections this takes out are prose rather than code.
fn split_heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].trim();
    // `#hashtag` is not a heading.
    line[hashes..].starts_with(' ').then_some((hashes, rest))
}

/// Point the links of a rendered document at what this site publishes.
///
/// `pages` says which repository paths became which pages — `docs/help/files.md` →
/// `help/files.html`. Anything else that points at a file in the repository becomes a link
/// into the repository on GitHub: the reader asked for that document, and sending them to the
/// source is better than a page that is not there.
pub fn rewrite_links(html: &str, pages: &[(&str, String)], depth: usize) -> String {
    let up = "../".repeat(depth);
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find("href=\"") {
        let (before, after) = rest.split_at(at + 6);
        out.push_str(before);
        let Some(end) = after.find('"') else {
            out.push_str(after);
            return out;
        };
        let (target, tail) = after.split_at(end);
        out.push_str(&rewrite_one(target, pages, &up));
        rest = tail;
    }
    out.push_str(rest);
    out
}

fn rewrite_one(target: &str, pages: &[(&str, String)], up: &str) -> String {
    // Absolute, a mail link or a fragment within the page: left alone.
    if target.contains("://") || target.starts_with("mailto:") || target.starts_with('#') {
        return target.to_string();
    }
    let (path, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, format!("#{fragment}")),
        None => (target, String::new()),
    };
    let clean = path.trim_start_matches("./");
    // A document this site publishes: the page, relative to where this one sits.
    for (source, page) in pages {
        if ends_with_path(clean, source) {
            return format!("{up}{page}{fragment}");
        }
    }
    // A picture this site carries, and anything else already relative to the site root.
    if clean.starts_with("docs/shots/") || clean.starts_with("assets/") {
        return format!("{up}{clean}{fragment}");
    }
    // Everything else: the repository itself. A reader who followed a link to a document we
    // do not publish gets the document, not a 404.
    if clean.is_empty() {
        return format!("{REPO}{fragment}");
    }
    format!("{REPO}/blob/main/{clean}{fragment}")
}

/// Does `link` name the file `source`? A link may be written from anywhere in the tree —
/// `docs/help/files.md`, `help/files.md`, `files.md` — and all three mean the same page.
fn ends_with_path(link: &str, source: &str) -> bool {
    if link == source {
        return true;
    }
    match source.strip_suffix(link) {
        Some(prefix) => prefix.is_empty() || prefix.ends_with('/'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_and_a_strikethrough_survive_the_trip() {
        let html = to_html("| a | b |\n| --- | --- |\n| 1 | 2 |\n\n~~gone~~\n");
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("<del>gone</del>"), "{html}");
    }

    #[test]
    fn markdown_that_looks_like_html_is_escaped_rather_than_run() {
        // The documents are full of `<path>` placeholders and `-D warnings`; none of them is
        // markup, and a generator that let them through would be writing broken pages.
        let html = to_html("a `<script>` in code, and a <b>bare tag</b>\n");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn a_section_is_taken_out_by_its_heading_and_stops_at_the_next_one() {
        let doc =
            "# Title\n\nintro\n\n## One\n\nfirst\n\n### Deeper\n\nstill one\n\n## Two\n\nsecond\n";
        let one = section(doc, "One").expect("the section is there");
        assert!(one.contains("first"), "{one}");
        assert!(
            one.contains("### Deeper"),
            "a subsection belongs to it: {one}"
        );
        assert!(
            !one.contains("second"),
            "it ran into the next section: {one}"
        );
        assert_eq!(section(doc, "Two").as_deref(), Some("second"));
        // A heading that is not there is a mistake in the generator, not an empty page.
        assert_eq!(section(doc, "Three"), None);
        // `#hashtag` is not a heading.
        assert_eq!(section("#nope\n\nx\n", "nope"), None);
    }

    #[test]
    fn a_link_to_a_document_we_publish_becomes_a_link_to_the_page() {
        let pages = [
            ("docs/help/files.md", "help/files.html".to_string()),
            ("CHANGELOG.md", "changelog.html".to_string()),
        ];
        let html = r#"<a href="docs/help/files.md">a</a> <a href="./CHANGELOG.md#top">b</a>"#;
        let out = rewrite_links(html, &pages, 0);
        assert!(out.contains(r#"href="help/files.html""#), "{out}");
        assert!(out.contains(r#"href="changelog.html#top""#), "{out}");

        // From a page one directory down, the same links climb out first.
        let out = rewrite_links(html, &pages, 1);
        assert!(out.contains(r#"href="../help/files.html""#), "{out}");

        // A link written from inside `docs/help` means the same page.
        let out = rewrite_links(r#"<a href="files.md">a</a>"#, &pages, 0);
        assert!(out.contains(r#"href="help/files.html""#), "{out}");
    }

    #[test]
    fn a_link_to_anything_else_goes_to_the_repository_rather_than_nowhere() {
        let pages: [(&str, String); 0] = [];
        let out = rewrite_links(r#"<a href="docs/spec/02-panels.md">spec</a>"#, &pages, 0);
        assert!(
            out.contains(&format!("{REPO}/blob/main/docs/spec/02-panels.md")),
            "{out}"
        );
        // An absolute link, a fragment and a mail link are left exactly as they were.
        for link in [
            "https://example.com/x",
            "#section",
            "mailto:somebody@example.com",
        ] {
            let html = format!(r#"<a href="{link}">x</a>"#);
            assert_eq!(rewrite_links(&html, &pages, 2), html, "{link}");
        }
        // A picture is carried by the site itself.
        let out = rewrite_links(r#"<a href="docs/shots/panels.svg">p</a>"#, &pages, 1);
        assert!(out.contains(r#"href="../docs/shots/panels.svg""#), "{out}");
    }

    #[test]
    fn a_document_with_no_links_comes_back_unchanged() {
        let pages: [(&str, String); 0] = [];
        let html = "<p>nothing to rewrite</p>";
        assert_eq!(rewrite_links(html, &pages, 0), html);
        // And a malformed one does not panic or lose its tail.
        let broken = r#"<a href="unclosed>"#;
        assert!(rewrite_links(broken, &pages, 0).ends_with("unclosed>"));
    }
}
