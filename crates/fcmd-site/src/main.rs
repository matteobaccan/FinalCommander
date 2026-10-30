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
                    body: document_in(&lang, title, &source, &published, 1, in_english),
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
        ("editor.svg", "F4"),
        ("menu.svg", "F9"),
        ("s3.svg", "s3://"),
        ("help.svg", "F1"),
    ] {
        body.push_str(&format!(
            "<figure><img src=\"docs/shots/{file}\" alt=\"{caption}\" loading=\"lazy\">\
             <figcaption>{caption}</figcaption></figure>\n"
        ));
    }
    body.push_str("</div>\n");
    body.push_str(&format!("<p class=\"note\">{}</p>\n", escape(lang.footer)));
    body
}

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
