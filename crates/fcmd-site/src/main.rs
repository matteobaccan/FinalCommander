//! The generator for the project's web site.
//!
//! `cargo run -p fcmd-site -- --out site` writes a static site into `site/`: a landing page in
//! each of the seven languages the program speaks, the README's own argument, the survey of the
//! other commanders, the help pages that `F1` shows inside the program, and the changelog.
//! `.github/workflows/pages.yml` runs it and publishes the result to GitHub Pages.
//!
//! Three rules it keeps:
//!
//! **The content is the repository's own documents.** Nothing on the site is written twice: the
//! help pages are `docs/help/*.md`, the changelog is `CHANGELOG.md`, the argument is the
//! README's. A document that changes changes the site, and there is no second copy to forget.
//!
//! **The site is files.** No JavaScript, no framework, nothing fetched from anywhere else. It
//! works offline and out of a `file://` URL. A program that is one executable with everything
//! in it should not need a build pipeline to say so.
//!
//! **It says what it does not have.** The long documents are in English; a page that shows one
//! says so in the reader's own language instead of pretending otherwise — and the page *around*
//! it stays in that language, so following a link inside one's own site does not turn the site
//! English.
//!
//! **Nothing here names a version.** Where to get the binaries is the releases page, which is
//! always the latest one; a page of ours listing the archives of one version would be wrong the
//! day after a release and would make every release a reason to publish the site again.

mod md;
mod page;
mod text;

use std::fs;
use std::path::{Path, PathBuf};

use page::{escape, suffix_of, Page};
use text::{Text, LANGUAGES};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repo = default_repo();
    let mut out = PathBuf::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--repo" => repo = PathBuf::from(rest.next().cloned().unwrap_or_default()),
            "--out" => out = PathBuf::from(rest.next().cloned().unwrap_or_default()),
            "--help" | "-h" => {
                println!("fcmd-site [--repo <checkout>] [--out <directory>]");
                return;
            }
            other => {
                eprintln!("fcmd-site: unknown argument `{other}`");
                std::process::exit(2);
            }
        }
    }
    if out.as_os_str().is_empty() {
        out = repo.join("site");
    }
    match build(&repo, &out) {
        Ok(count) => println!("{} pages written to {}", count, out.display()),
        Err(e) => {
            eprintln!("fcmd-site: {e}");
            std::process::exit(1);
        }
    }
}

/// The checkout this crate lives in, which is where it is run from in practice.
fn default_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// What goes on the site, and under what name. The order is the order of the navigation.
const HELP_PAGES: [(&str, &str); 7] = [
    ("panels.md", "Panels"),
    ("files.md", "Files and archives"),
    ("viewer.md", "Viewer"),
    ("editor.md", "Editor"),
    ("network.md", "Servers and buckets"),
    ("config.md", "Configuration"),
    ("about.md", "About"),
];

/// What the site publishes, so a link to one of these documents becomes a link to its page
/// rather than one into the repository — **in the reader's own language**.
///
/// This is per language and not a constant because the pages are: `help/files.html` is the
/// English chapter and `help/files.it.html` the Italian one. A single table meant that a link
/// from one Italian page to another landed on the English page, and the reader lost their
/// language by following a link inside their own site.
fn published_for(lang: &Text) -> Vec<(&'static str, String)> {
    let suffix = suffix_of(lang.code);
    let mut out: Vec<(&'static str, String)> = vec![
        ("CHANGELOG.md", format!("changelog{suffix}.html")),
        ("docs/00-landscape.md", format!("commanders{suffix}.html")),
        ("README.md", format!("index{suffix}.html")),
    ];
    for (file, _) in HELP_PAGES {
        let name = file.trim_end_matches(".md");
        out.push((
            // The leaked string is the generator's own table living as long as it runs.
            Box::leak(format!("docs/help/{file}").into_boxed_str()),
            format!("help/{name}{suffix}.html"),
        ));
    }
    out
}

fn build(repo: &Path, out: &Path) -> Result<usize, String> {
    let readme = read(repo, "README.md")?;
    let changelog = read(repo, "CHANGELOG.md")?;
    let landscape = read(repo, "docs/00-landscape.md")?;

    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    write(out, "style.css", page::STYLE)?;
    copy_assets(repo, out)?;

    let mut pages = 0usize;

    // The landing page, the features and what it replaces: one of each per language, because
    // these are the pages somebody arrives at.
    let replaces = md::section(&readme, "What the one file replaces")
        .ok_or("README.md: the section `What the one file replaces` is gone")?;
    let works = md::section(&readme, "What works today")
        .ok_or("README.md: the section `What works today` is gone")?;
    for lang in LANGUAGES {
        let published = published_for(&lang);
        let translations = |stem: &str| -> Vec<(&'static str, String)> {
            LANGUAGES
                .iter()
                .map(|t| (t.code, format!("{stem}{}.html", suffix_of(t.code))))
                .collect()
        };
        write_page(
            out,
            Page {
                path: format!("index{}.html", suffix_of(lang.code)),
                title: lang.tagline,
                lang: &lang,
                body: landing(&lang),
                translations: translations("index"),
            },
        )?;
        pages += 1;

        write_page(
            out,
            Page {
                path: format!("features{}.html", suffix_of(lang.code)),
                title: lang.nav_features,
                lang: &lang,
                body: document(&lang, lang.nav_features, &works, &published, 0),
                translations: translations("features"),
            },
        )?;
        pages += 1;

        write_page(
            out,
            Page {
                path: format!("replaces{}.html", suffix_of(lang.code)),
                title: lang.nav_replaces,
                lang: &lang,
                body: document(&lang, lang.nav_replaces, &replaces, &published, 0),
                translations: translations("replaces"),
            },
        )?;
        pages += 1;

        // The long documents, in this reader's language's chrome. The text itself is
        // English and says so; what is translated is the page around it, so following
        // `Changelog` from the Italian site does not turn the whole site English.
        write_page(
            out,
            Page {
                path: format!("commanders{}.html", suffix_of(lang.code)),
                title: lang.nav_commanders,
                lang: &lang,
                body: document_in(&lang, lang.nav_commanders, &landscape, &published, 0, true),
                translations: translations("commanders"),
            },
        )?;
        pages += 1;

        write_page(
            out,
            Page {
                path: format!("changelog{}.html", suffix_of(lang.code)),
                title: lang.nav_changelog,
                lang: &lang,
                body: document_in(&lang, lang.nav_changelog, &changelog, &published, 0, true),
                translations: translations("changelog"),
            },
        )?;
        pages += 1;
    }

    // The help: an index and the chapters, in every language. The chapters are written in
    // English and translated one at a time, so a language whose chapter is not translated yet
    // gets the English with a line in its own language saying so — rather than an index in
    // Italian that silently opens an English page, which is what this used to do.
    for lang in LANGUAGES {
        let suffix = suffix_of(lang.code);
        let published = published_for(&lang);
        let mut index = format!(
            "<h1>{}</h1>\n<p>{}</p>\n<ul class=\"pages\">\n",
            escape(lang.help_title),
            escape(lang.help_intro)
        );
        for (n, (file, _)) in HELP_PAGES.iter().enumerate() {
            let name = file.trim_end_matches(".md");
            index.push_str(&format!(
                "<li><a href=\"{name}{suffix}.html\">{}</a></li>\n",
                escape(lang.help_chapters[n])
            ));
        }
        index.push_str("</ul>\n");
        write_page(
            out,
            Page {
                path: format!("help/index{suffix}.html"),
                title: lang.help_title,
                lang: &lang,
                body: index,
                translations: LANGUAGES
                    .iter()
                    .map(|other| {
                        (
                            other.code,
                            format!("help/index{}.html", suffix_of(other.code)),
                        )
                    })
                    .collect(),
            },
        )?;
        pages += 1;

        for (n, (file, _)) in HELP_PAGES.iter().enumerate() {
            let (source, in_english) = help_source(repo, &lang, file)?;
            let name = file.trim_end_matches(".md");
            let title = lang.help_chapters[n];
            write_page(
                out,
                Page {
                    path: format!("help/{name}{suffix}.html"),
                    title,
                    lang: &lang,
                    body: with_shots(
                        file,
                        &document_in(&lang, title, &source, &published, 1, in_english),
                        1,
                    ),
                    translations: LANGUAGES
                        .iter()
                        .map(|other| {
                            (
                                other.code,
                                format!("help/{name}{}.html", suffix_of(other.code)),
                            )
                        })
                        .collect(),
                },
            )?;
            pages += 1;
        }
    }

    Ok(pages)
}

/// The landing page: the promise, a picture of it, and the way in.
fn landing(lang: &Text) -> String {
    let suffix = suffix_of(lang.code);
    let mut body = String::new();
    body.push_str(&format!(
        "<h1>FinalCommander</h1>\n<p class=\"tagline\">{}</p>\n<p class=\"pitch\">{}</p>\n",
        escape(lang.tagline),
        inline_code(lang.pitch)
    ));
    body.push_str(&format!(
        "<div class=\"buttons\">\
         <a href=\"{}/releases/latest\">{}</a>\
         <a href=\"features{suffix}.html\">{}</a>\
         <a href=\"{}\">{}</a>\
         </div>\n",
        md::REPO,
        escape(lang.nav_download),
        escape(lang.nav_features),
        md::REPO,
        escape(lang.nav_source),
    ));
    // The one thing this site asks for, where somebody who read the buttons will see it: an
    // idea and an issue, not a patch (`README.md` says it at length, and says why).
    body.push_str(&format!(
        "<p class=\"ideas\">{} <a href=\"{}/issues/new/choose\">{}</a></p>\n",
        escape(lang.ideas),
        md::REPO,
        escape(lang.ideas_link),
    ));
    body.push_str(&format!(
        "<figure><img src=\"docs/shots/panels.svg\" alt=\"{}\" width=\"880\"></figure>\n",
        escape(lang.tagline)
    ));
    body.push_str(&format!(
        "<h2>{}</h2>\n<div class=\"shots\">\n",
        escape(lang.shots_title)
    ));
    for (file, caption) in [
        ("viewer.svg", "F3"),
        ("hex.svg", "F4"),
        ("disasm.svg", "F5"),
        ("hexedit.svg", "F4 · hex"),
        ("picture.svg", "F3 · BMP"),
        ("drives.svg", "Alt+F1"),
        ("branch.svg", "Ctrl+Alt+B"),
        ("split.svg", ".001 · .002 · .crc"),
        ("editor.svg", "F4"),
        ("menu.svg", "F9"),
        ("s3.svg", "s3://"),
        ("git.svg", "Alt+F1 · Git"),
        ("ai.svg", "Alt+F1 · AI"),
        ("diff.svg", "Ctrl+Shift+D"),
        ("usage.svg", "Alt+Shift+F3 · Tab"),
        ("help.svg", "F1"),
    ] {
        body.push_str(&format!(
            "<figure><img src=\"docs/shots/{file}\" alt=\"{caption}\" loading=\"lazy\">\
             <figcaption>{caption}</figcaption></figure>\n"
        ));
    }
    body.push_str("</div>\n");
    // The themes, each on the same scene, so that what differs is the colour and nothing else.
    body.push_str("<h2 id=\"themes\">Themes</h2>\n<div class=\"shots\">\n");
    for id in THEMES {
        body.push_str(&format!(
            "<figure><img src=\"docs/shots/theme-{id}.svg\" alt=\"{id}\" loading=\"lazy\">\
             <figcaption>{id}</figcaption></figure>\n"
        ));
    }
    body.push_str("</div>\n");
    body.push_str(&format!("<p class=\"note\">{}</p>\n", escape(lang.footer)));
    body
}

/// The themes the program ships, in its own order; each has its picture in `docs/shots`,
/// drawn by the renderer's screenshot test.
const THEMES: &[&str] = &[
    "commander",
    "far",
    "black",
    "white",
    "matrix",
    "tron",
    "blade",
    "github-dark",
    "delphi-dark-2027",
    "delphi-inspiration",
];

/// A page built from one of the repository's documents, with the note that says it is English.
fn document(
    lang: &Text,
    title: &str,
    markdown: &str,
    published: &[(&str, String)],
    depth: usize,
) -> String {
    document_in(lang, title, markdown, published, depth, lang.code != "en")
}

/// The same, told whether the text really is the English one.
///
/// The repository's long documents are English whatever page they are on, so `document` can
/// work that out from the language. A help chapter cannot: some are translated and some are
/// not, and the note has to appear on exactly the ones that are not.
fn document_in(
    lang: &Text,
    title: &str,
    markdown: &str,
    published: &[(&str, String)],
    depth: usize,
    in_english: bool,
) -> String {
    let mut body = format!("<h1>{}</h1>\n", escape(title));
    if in_english {
        body.push_str(&format!(
            "<p class=\"note\">{}</p>\n",
            escape(lang.in_english)
        ));
    }
    body.push_str(&md::rewrite_links(&md::to_html(markdown), published, depth));
    body
}

/// The pictures that go into the help chapters on the site: which page, under which of its
/// `#` chapters (counted from 0, so a translation with the same chapters gets the same
/// pictures), the picture, and the key it shows, which is the caption in every language.
///
/// Only on the site. Inside the program `F1` is the same text drawn in the terminal, and a
/// picture of the terminal there would be a picture of the screen one is already looking at.
const HELP_SHOTS: &[(&str, usize, &str, &str)] = &[
    ("panels.md", 0, "panels.svg", "Ctrl+1…0"),
    ("panels.md", 0, "drives.svg", "Alt+F1"),
    ("panels.md", 1, "branch.svg", "Ctrl+Alt+B"),
    ("files.md", 0, "menu.svg", "F9"),
    ("files.md", 4, "split.svg", ".001 · .002 · .crc"),
    ("files.md", 8, "usage.svg", "Alt+Shift+F3 · Tab"),
    ("files.md", 14, "git.svg", "Alt+F1 · Git"),
    ("viewer.md", 0, "viewer.svg", "F3"),
    ("viewer.md", 2, "picture.svg", "F3 · BMP"),
    ("viewer.md", 6, "hex.svg", "F4"),
    ("viewer.md", 6, "disasm.svg", "F5"),
    ("viewer.md", 7, "diff.svg", "Ctrl+Shift+D"),
    ("config.md", 1, "ai.svg", "Alt+F1 · AI"),
    ("editor.md", 0, "editor.svg", "F4"),
    ("editor.md", 2, "hexedit.svg", "F4 · hex"),
    ("network.md", 0, "s3.svg", "s3://"),
    ("about.md", 0, "help.svg", "F1"),
];

/// `html` with the pictures of `file` put after the headings of their chapters.
fn with_shots(file: &str, html: &str, depth: usize) -> String {
    let up = "../".repeat(depth);
    let mut out = String::with_capacity(html.len() + 512);
    // The first heading is the page's own title, put there by `document_in`; the chapters
    // are the ones after it.
    let mut chapter: Option<usize> = None;
    let mut rest = html;
    while let Some(at) = rest.find("</h1>") {
        let end = at + "</h1>".len();
        out.push_str(&rest[..end]);
        for (_, _, shot, caption) in HELP_SHOTS
            .iter()
            .filter(|(page, n, _, _)| *page == file && Some(*n) == chapter)
        {
            out.push_str(&format!(
                "\n<figure class=\"shot\"><img src=\"{up}docs/shots/{shot}\" alt=\"{caption}\" \
                 loading=\"lazy\"><figcaption>{caption}</figcaption></figure>"
            ));
        }
        chapter = Some(chapter.map_or(0, |n| n + 1));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// A help chapter's source for one language: the translation if the repository has one, the
/// English otherwise — and whether what came back is the English.
fn help_source(repo: &Path, lang: &Text, file: &str) -> Result<(String, bool), String> {
    if lang.code != "en" {
        let translated = format!("docs/help/{}/{file}", lang.code);
        if repo.join(&translated).is_file() {
            return Ok((read(repo, &translated)?, false));
        }
    }
    Ok((read(repo, &format!("docs/help/{file}"))?, lang.code != "en"))
}

fn inline_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code = false;
    // The strings are written on several source lines, so a run of spaces is one space.
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for c in collapsed.chars() {
        if c == '`' {
            out.push_str(if in_code { "</code>" } else { "<code>" });
            in_code = !in_code;
            continue;
        }
        out.push_str(&escape(&c.to_string()));
    }
    if in_code {
        out.push_str("</code>");
    }
    out
}

/// The pictures and the logo, copied as they are: they are drawn by the program itself.
fn copy_assets(repo: &Path, out: &Path) -> Result<(), String> {
    for (from, to) in [("docs/shots", "docs/shots"), ("assets", "assets")] {
        let source = repo.join(from);
        let target = out.join(to);
        fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        let entries = fs::read_dir(&source).map_err(|e| format!("{}: {e}", source.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", source.display()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                continue;
            }
            let name = entry.file_name();
            fs::copy(&path, target.join(&name)).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn write_page(out: &Path, page: Page<'_>) -> Result<(), String> {
    let path = page.path.clone();
    write(out, &path, &page.render())
}

fn write(out: &Path, path: &str, body: &str) -> Result<(), String> {
    let target = out.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(&target, body).map_err(|e| format!("{}: {e}", target.display()))
}

fn read(repo: &Path, path: &str) -> Result<String, String> {
    let full = repo.join(path);
    fs::read_to_string(&full).map_err(|e| format!("{}: {e}", full.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme the gallery shows has its picture, and the list is the program's: a
    /// theme added without one, or a picture of a theme that went, fails here.
    #[test]
    fn every_theme_has_its_picture_and_no_picture_is_left_over() {
        let shots = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots");
        let mut found: Vec<String> = std::fs::read_dir(&shots)
            .expect("the shots")
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.strip_prefix("theme-")?
                    .strip_suffix(".svg")
                    .map(str::to_string)
            })
            .collect();
        found.sort();
        let mut listed: Vec<String> = THEMES.iter().map(|t| t.to_string()).collect();
        listed.sort();
        assert_eq!(found, listed);
        let themes_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fcmd-ui/themes");
        let mut shipped: Vec<String> = std::fs::read_dir(themes_dir)
            .expect("the themes")
            .flatten()
            .filter_map(|e| {
                e.file_name()
                    .to_string_lossy()
                    .strip_suffix(".toml")
                    .map(str::to_string)
            })
            .collect();
        shipped.sort();
        assert_eq!(
            shipped, listed,
            "the gallery lists the themes the program ships"
        );
    }

    /// The number a section heading carries: `### 1.8 Structure pane` is `1.8`, and
    /// `## 2b-quinquies. [[archiver]] blocks` is `2b-quinquies`. `None` for a heading that is
    /// not numbered.
    fn section_number(line: &str) -> Option<String> {
        let rest = line.trim_start_matches('#');
        if rest.len() == line.len() || !rest.starts_with(' ') || line.starts_with("#####") {
            return None;
        }
        let token = rest.split_whitespace().next()?;
        let token = token.trim_end_matches('.');
        token
            .chars()
            .next()
            .filter(char::is_ascii_digit)
            .map(|_| token.to_string())
    }

    /// Every `SPEC NN §X` anywhere in the repository names a section that exists, and no spec
    /// has two sections with one number.
    ///
    /// Written after an audit found forty-two of them pointing nowhere: sections planned and
    /// never written (section 7b of spec 02, whose content went into §14), rows of the feature table
    /// cited as if they were sections (spec 02 "section 1.6", which is row 1.6 and section 8), and
    /// sections renumbered under the references to them. A reader following one landed
    /// nowhere and could not tell whether the code or the spec had moved. It also found two
    /// specs with one number used twice, which makes a correct reference ambiguous.
    #[test]
    fn every_spec_reference_names_a_section_that_exists() {
        let repo = default_repo();
        let mut sections: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            Default::default();
        let mut doubled = Vec::new();
        for entry in std::fs::read_dir(repo.join("docs/spec")).expect("docs/spec") {
            let path = entry.expect("an entry").path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let Some(number) = name
                .get(..2)
                .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
            else {
                continue;
            };
            let mut here = std::collections::BTreeSet::new();
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if let Some(section) = section_number(line) {
                    if !here.insert(section.clone()) {
                        doubled.push(format!("{name}: §{section} twice"));
                    }
                    // A reference to `§1` reaches `1.8`'s chapter as well.
                    if let Some((chapter, _)) = section.split_once('.') {
                        here.insert(chapter.to_string());
                    }
                }
            }
            sections.insert(number.to_string(), here);
        }
        assert!(sections.len() >= 10, "found only {:?}", sections.keys());
        assert!(doubled.is_empty(), "{doubled:#?}");

        let mut broken = Vec::new();
        let mut stack = vec![repo.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("a directory") {
                let path = entry.expect("an entry").path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if path.is_dir() {
                    // Hidden folders are never the repository's own text: `.git`, and the
                    // working copies tools keep beside it, whose docs may be half written.
                    if name != "target" && name != "node_modules" && !name.starts_with('.') {
                        stack.push(path);
                    }
                    continue;
                }
                if !(name.ends_with(".md") || name.ends_with(".rs") || name.ends_with(".toml")) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (n, line) in text.lines().enumerate() {
                    let mut rest = line;
                    while let Some(at) = rest.find("SPEC ") {
                        rest = &rest[at + 5..];
                        let Some(number) = rest
                            .get(..2)
                            .filter(|d| d.chars().all(|c| c.is_ascii_digit()))
                        else {
                            continue;
                        };
                        let Some(after) = rest[2..].strip_prefix(" §") else {
                            continue;
                        };
                        let section: String = after
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                            .collect();
                        let section = section.trim_end_matches(['.', '-']);
                        if section.is_empty() {
                            continue;
                        }
                        let known = sections.get(number).is_some_and(|s| s.contains(section));
                        if !known {
                            let shown = path.strip_prefix(&repo).unwrap_or(&path).display();
                            broken.push(format!("{shown}:{}: SPEC {number} §{section}", n + 1));
                        }
                    }
                }
            }
        }
        assert!(
            broken.is_empty(),
            "{} references to sections that do not exist:\n{}",
            broken.len(),
            broken.join("\n")
        );
    }

    /// A document named in backticks — `docs/BACKLOG.md`, `spec/08-plugins.md`,
    /// `05-multiarc.md` — is a file that exists. These are not links, so the site's link
    /// check never sees them, and two had gone stale: one named a spec by the number it had
    /// before the specs were renumbered, the other the inventory by the wrong number.
    #[test]
    fn every_document_named_in_backticks_exists() {
        let repo = default_repo();
        let mut missing = Vec::new();
        let mut stack = vec![repo.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("a directory") {
                let path = entry.expect("an entry").path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if path.is_dir() {
                    if !matches!(name.as_str(), "target" | ".git" | "node_modules") {
                        stack.push(path);
                    }
                    continue;
                }
                if !(name.ends_with(".md") || name.ends_with(".rs") || name.ends_with(".toml")) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (n, line) in text.lines().enumerate() {
                    for (i, quoted) in line.split('`').enumerate() {
                        // Odd pieces are the ones between a pair of backticks.
                        if i % 2 == 0
                            || !quoted.ends_with(".md")
                            || quoted.contains([' ', '<', '*', '{'])
                        {
                            continue;
                        }
                        let numbered = quoted.rsplit('/').next().is_some_and(|f| {
                            f.len() > 3
                                && f[..2].bytes().all(|b| b.is_ascii_digit())
                                && &f[2..3] == "-"
                        });
                        let candidates: Vec<std::path::PathBuf> = if quoted.starts_with("docs/") {
                            vec![repo.join(quoted)]
                        } else if quoted.starts_with("spec/") {
                            vec![repo.join("docs").join(quoted)]
                        } else if numbered && !quoted.contains('/') {
                            vec![
                                repo.join("docs").join(quoted),
                                repo.join("docs/spec").join(quoted),
                            ]
                        } else {
                            continue;
                        };
                        if !candidates.iter().any(|c| c.exists()) {
                            let shown = path.strip_prefix(&repo).unwrap_or(&path).display();
                            missing.push(format!("{shown}:{}: {quoted}", n + 1));
                        }
                    }
                }
            }
        }
        assert!(missing.is_empty(), "{}", missing.join("\n"));
    }

    /// Every argument a message names is one its caller gives.
    ///
    /// Fluent prints a placeholder nobody filled as `{$bytes}`, literally, in the middle of the
    /// status line — which is what saving in the hex editor said until 0.30.0, because the call
    /// passed `lines` to a message that asks for `bytes`. Nothing fails when that happens, so
    /// nothing noticed. This reads every `t_args("key", &[("name", …), …])` in the workspace
    /// and the English catalogue, and asks that each `{ $name }` of the message is passed. An
    /// argument passed and not used is allowed: a message may say less than it is told.
    #[test]
    fn every_placeholder_of_a_message_is_given_by_its_caller() {
        let repo = default_repo();
        let catalogue =
            std::fs::read_to_string(repo.join("crates/fcmd-i18n/i18n/en/fc.ftl")).unwrap();
        // Key to the placeholders it uses, a message running on over indented lines.
        let mut wants: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
            Default::default();
        let mut current: Option<String> = None;
        for line in catalogue.lines() {
            let continued = line.starts_with([' ', '\t']);
            if !continued {
                current = line
                    .split_once('=')
                    .map(|(k, _)| k.trim().to_string())
                    .filter(|k| !k.is_empty() && !k.starts_with('#'));
                if let Some(key) = &current {
                    wants.entry(key.clone()).or_default();
                }
            }
            let Some(key) = &current else { continue };
            let mut rest = line;
            while let Some(at) = rest.find("$") {
                rest = &rest[at + 1..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    wants.get_mut(key).expect("entered").insert(name);
                }
            }
        }

        let mut missing = Vec::new();
        let mut calls = 0;
        let mut stack = vec![repo.join("crates")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let mut from = 0;
                while let Some(at) = text[from..].find("t_args(") {
                    let start = from + at;
                    from = start + 7;
                    let rest = text[from..].trim_start();
                    let Some(rest) = rest.strip_prefix('"') else {
                        continue;
                    };
                    let Some(end) = rest.find('"') else { continue };
                    let key = &rest[..end];
                    let rest = &rest[end + 1..];
                    let Some(open) = rest.find("&[") else {
                        continue;
                    };
                    // The argument list, to its own closing bracket.
                    let mut depth = 0;
                    let mut close = None;
                    for (i, c) in rest[open + 1..].char_indices() {
                        match c {
                            '[' => depth += 1,
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    close = Some(open + 1 + i);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let Some(close) = close else { continue };
                    let args = &rest[open..close];
                    // Each `(` followed, perhaps over a line break, by a quoted name.
                    let given: std::collections::BTreeSet<String> = args
                        .split('(')
                        .skip(1)
                        .filter_map(|piece| piece.trim_start().strip_prefix('"'))
                        .filter_map(|piece| piece.split('"').next())
                        .map(str::to_string)
                        .collect();
                    let Some(want) = wants.get(key) else {
                        // A key built at run time, or one the catalogue test covers.
                        continue;
                    };
                    calls += 1;
                    let absent: Vec<_> = want.difference(&given).cloned().collect();
                    if !absent.is_empty() {
                        let line = text[..start].matches('\n').count() + 1;
                        let shown = path.strip_prefix(&repo).unwrap_or(&path).display();
                        missing.push(format!("{shown}:{line}: {key} is not given {absent:?}"));
                    }
                }
            }
        }
        assert!(calls > 100, "found only {calls} calls");
        assert!(missing.is_empty(), "{}", missing.join("\n"));
    }

    /// Every row of the feature table says its state with one of the marks its legend
    /// defines. A row that invented its own (`✗` had crept in for a decision) reads as a
    /// fourth state to whoever counts them, and the table is what gets counted.
    #[test]
    fn every_feature_row_has_a_state_the_legend_defines() {
        let text = std::fs::read_to_string(default_repo().join("docs/02-features.md")).unwrap();
        let marks = ["✓", "~", "M2", "M3", "M4", "—"];
        let mut rows = 0;
        let mut odd = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // `| 1.4 | feature | origin | state |` splits into six, the ends empty.
            let numbered = cells
                .get(1)
                .is_some_and(|c| c.starts_with(|ch: char| ch.is_ascii_digit()));
            if !numbered || cells.len() < 6 {
                continue;
            }
            rows += 1;
            let state = cells[cells.len() - 2];
            if !marks.iter().any(|m| state.starts_with(m)) {
                odd.push(format!("line {}: {state}", n + 1));
            }
        }
        assert!(rows > 150, "read only {rows} rows");
        assert!(odd.is_empty(), "{odd:#?}");
    }

    /// Build the whole site into a temporary directory, from this checkout.
    fn built() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a directory");
        let out = dir.path().join("site");
        let count = build(&default_repo(), &out).expect("the site is built");
        assert!(count > 30, "only {count} pages");
        (dir, out)
    }

    fn page_text(out: &Path, path: &str) -> String {
        fs::read_to_string(out.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn the_whole_site_is_written_from_the_repositorys_own_documents() {
        let (_dir, out) = built();

        // Every page a reader can reach exists in every language, and English has no
        // suffix. The long documents are among them: their text is English and their page
        // is not, so a link to one does not take somebody out of their own language.
        for lang in LANGUAGES {
            for stem in ["index", "features", "replaces", "commanders", "changelog"] {
                let path = format!("{stem}{}.html", suffix_of(lang.code));
                assert!(out.join(&path).is_file(), "{path} is missing");
            }
        }
        assert!(out.join("style.css").is_file());
        // Every help page the program shows is a page here, in every language.
        for lang in LANGUAGES {
            let suffix = suffix_of(lang.code);
            assert!(
                out.join(format!("help/index{suffix}.html")).is_file(),
                "help/index{suffix}.html is missing"
            );
            for (file, _) in HELP_PAGES {
                let name = file.trim_end_matches(".md");
                assert!(
                    out.join(format!("help/{name}{suffix}.html")).is_file(),
                    "help/{name}{suffix}.html is missing"
                );
            }
        }
        // And the pictures came with it.
        assert!(out.join("docs/shots/panels.svg").is_file());
        assert!(out.join("assets/logo.svg").is_file());
    }

    /// The complaint this answers: the Italian menu used to link to the English help index,
    /// and the chapters said nothing about being in English — so a reader who switched
    /// language got a program that looked translated and a manual that was not.
    /// The help chapters on the site have pictures under their headings, in every language,
    /// and every picture they name is one the generator writes.
    #[test]
    fn the_help_chapters_show_the_screens_they_describe() {
        let (_dir, out) = built();
        let viewer = page_text(&out, "help/viewer.html");
        let binaries = viewer.find("Binaries").expect("the chapter");
        let disasm = viewer.find("docs/shots/disasm.svg").expect("the picture");
        assert!(disasm > binaries, "the picture is under its chapter");
        assert!(
            viewer.contains(r#"src="../docs/shots/viewer.svg""#),
            "{viewer}"
        );
        let italian = page_text(&out, "help/viewer.it.html");
        assert!(italian.contains("docs/shots/disasm.svg"), "{italian}");
        for (_, _, shot, _) in HELP_SHOTS {
            assert!(
                default_repo().join("docs/shots").join(shot).is_file(),
                "{shot} is not in docs/shots"
            );
        }
    }

    #[test]
    fn the_help_has_an_index_per_language_that_says_which_chapters_are_still_english() {
        let (_dir, out) = built();

        let italian = page_text(&out, "help/index.it.html");
        // The chapters are named in Italian, and the links are the Italian pages.
        assert!(italian.contains("I pannelli"), "{italian}");
        assert!(italian.contains("panels.it.html"), "{italian}");
        // The navigation of an Italian page goes to the Italian index, not the English one.
        let landing = page_text(&out, "index.it.html");
        assert!(landing.contains("help/index.it.html"), "{landing}");

        // A chapter with no translation yet is the English one, and says so in Italian.
        let chapter = page_text(&out, "help/panels.it.html");
        assert!(chapter.contains("I pannelli"), "{chapter}");
        let note = crate::text::LANGUAGES
            .iter()
            .find(|t| t.code == "it")
            .expect("italian")
            .in_english;
        let translated = std::path::Path::new("docs/help/it/panels.md");
        if default_repo().join(translated).is_file() {
            // Once it is translated the note goes, and nothing has to be changed here.
            assert!(!chapter.contains(note), "the note outlived the translation");
        } else {
            assert!(chapter.contains(note), "{chapter}");
        }

        // English says nothing of the sort, because there is nothing to say.
        let english = page_text(&out, "help/panels.html");
        assert!(!english.contains(note), "{english}");
    }

    #[test]
    fn the_landing_page_says_what_the_program_is_in_the_readers_language() {
        let (_dir, out) = built();
        let english = page_text(&out, "index.html");
        assert!(english.contains("One file. Every system."), "{english}");
        assert!(english.contains("docs/shots/panels.svg"));

        let italian = page_text(&out, "index.it.html");
        assert!(italian.contains("lang=\"it\""), "{italian}");
        assert!(italian.contains("Un file. Ogni sistema."), "{italian}");
        // The pitch's `scp` is code rather than a backtick on the page.
        assert!(italian.contains("<code>scp</code>"), "{italian}");
        assert!(!italian.contains('`'), "a backtick reached the page");
    }

    #[test]
    fn a_page_built_from_a_document_carries_the_document_and_says_it_is_english() {
        let (_dir, out) = built();
        let features = page_text(&out, "features.html");
        // From the README's own section, not written a second time.
        assert!(
            features.contains("<strong>File operations</strong>"),
            "{features}"
        );
        assert!(
            !features.contains("class=\"note\""),
            "English needs no note"
        );

        let french = page_text(&out, "features.fr.html");
        assert!(french.contains("est en anglais"), "{french}");
        assert!(
            french.contains("<strong>File operations</strong>"),
            "and the document is still there"
        );

        // The README's table of what one file replaces survives as a table.
        let replaces = page_text(&out, "replaces.html");
        assert!(replaces.contains("<table>"), "{replaces}");
    }

    #[test]
    fn the_changelog_and_the_survey_are_published_whole() {
        let (_dir, out) = built();
        let changelog = page_text(&out, "changelog.html");
        assert!(
            changelog.contains("Unreleased") || changelog.contains("0.3.0"),
            "{}",
            &changelog[..200]
        );
        let commanders = page_text(&out, "commanders.html");
        assert!(
            commanders.contains("FAR Manager"),
            "the survey is not there"
        );
        assert!(commanders.contains("Norton Commander"));

        // Both are English documents, and both exist in every language's *chrome*: the text
        // says it is in English, and the navigation around it stays in the reader's language
        // instead of turning the whole site English for anybody who followed a link to it.
        let italian = page_text(&out, "changelog.it.html");
        assert!(italian.contains("Unreleased") || italian.contains("0.3.0"));
        assert!(
            italian.contains("<html lang=\"it\">"),
            "the chrome is Italian"
        );
        assert!(
            italian.contains("in inglese") || italian.contains("inglese"),
            "and it says the document itself is English"
        );
        let italian = page_text(&out, "commanders.it.html");
        assert!(italian.contains("FAR Manager"));
        assert!(italian.contains("<html lang=\"it\">"));
    }

    /// There is no download page: the navigation and the landing button lead to the
    /// releases, which are always the latest without anybody rebuilding this site.
    #[test]
    fn the_way_to_the_binaries_is_the_releases_page_and_not_a_page_of_ours() {
        let (_dir, out) = built();
        assert!(
            !out.join("download.html").exists() && !out.join("download.it.html").exists(),
            "the download page is gone"
        );
        for page in ["index.html", "index.it.html", "help/files.it.html"] {
            let text = page_text(&out, page);
            assert!(
                text.contains(&format!("{}/releases/latest", md::REPO)),
                "{page} does not lead to the releases"
            );
        }
        // And no page names a version, so a release does not make this site wrong.
        for path in pages_of(&out) {
            let text = fs::read_to_string(&path).expect("a page");
            // The changelog is the one page that names versions, because that is what it is.
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("changelog"))
            {
                continue;
            }
            for line in text.lines() {
                assert!(
                    !line.contains("FinalCommander-linux-x86_64-0."),
                    "{}: an archive of one version is named: {line}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn a_link_between_documents_lands_on_this_sites_own_page() {
        let (_dir, out) = built();
        // The help index reaches every chapter with a link that stays on the site.
        let index = page_text(&out, "help/index.html");
        let targets = hrefs(&index);
        for (page, _) in HELP_PAGES {
            let file = page.replace(".md", ".html");
            assert!(
                targets.iter().any(|t| *t == file),
                "the help index does not reach {file}: {targets:?}"
            );
        }
        // And nowhere on the site does a link of its own point at a Markdown file,
        // which a browser would download rather than show. A document we do not
        // publish is linked on the repository instead, so an absolute URL may end
        // in `.md`.
        for path in pages_of(&out) {
            let text = fs::read_to_string(&path).expect("a page");
            for target in hrefs(&text) {
                assert!(
                    target.contains("://") || !target.ends_with(".md"),
                    "{}: a link to a Markdown file survived: {target}",
                    path.display()
                );
            }
        }
    }

    /// The `href="…"` targets of a page, in the order they appear.
    fn hrefs(html: &str) -> Vec<&str> {
        html.split("href=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect()
    }

    /// Every HTML page the site holds, the help chapters included.
    fn pages_of(out: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![out.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).expect("a directory").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "html") {
                    found.push(path);
                }
            }
        }
        found
    }

    /// The defect this test exists for: a reader on the Italian site followed a link and
    /// landed on the English page, because the table of what the site publishes named one
    /// page per document instead of one per document *per language*.
    #[test]
    fn every_link_on_a_page_stays_in_the_language_of_that_page() {
        let (_dir, out) = built();
        for lang in LANGUAGES {
            if lang.code == "en" {
                continue;
            }
            let suffix = suffix_of(lang.code);
            for path in pages_of(&out) {
                let name = path.strip_prefix(&out).expect("under the site").to_owned();
                let name = name.to_string_lossy().replace('\\', "/");
                if !name.ends_with(&format!("{suffix}.html")) {
                    continue;
                }
                let whole = fs::read_to_string(&path).expect("a page");
                // The language switcher points at the other languages on purpose, which is
                // the one place in a page that is allowed to, so it is cut out first.
                let text = match (
                    whole.find("<div class=\"languages\">"),
                    whole.find("</div>"),
                ) {
                    (Some(from), _) => {
                        let end = whole[from..]
                            .find("</div>")
                            .map(|n| from + n)
                            .unwrap_or(whole.len());
                        format!("{}{}", &whole[..from], &whole[end..])
                    }
                    _ => whole.clone(),
                };
                for target in hrefs(&text) {
                    // What leaves the site, the stylesheet and the pictures are not this
                    // test's business.
                    if target.contains("://")
                        || target.starts_with('#')
                        || target.ends_with(".css")
                        || target.ends_with(".svg")
                    {
                        continue;
                    }
                    let file = target.rsplit('/').next().unwrap_or(target);
                    assert!(
                        file.ends_with(&format!("{suffix}.html")),
                        "{name}: {target} leaves {}",
                        lang.code
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_on_the_site_is_fetched_from_somewhere_else() {
        let (_dir, out) = built();
        for path in [
            "index.html",
            "index.it.html",
            "help/files.html",
            "changelog.html",
            "commanders.it.html",
        ] {
            let text = page_text(&out, path);
            assert!(!text.contains("<script"), "{path} has a script in it");
            assert!(
                !text.contains("https://fonts.") && !text.contains("cdn."),
                "{path} fetches something"
            );
        }
    }
}
