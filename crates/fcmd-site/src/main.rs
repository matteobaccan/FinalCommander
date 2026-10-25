//! The generator for the project's web site.
//!
//! `cargo run -p fcmd-site -- --out site` writes a static site into `site/`: a landing page in
//! each of the seven languages the program speaks, the README's own argument, the survey of the
//! other commanders, the help pages that `F1` shows inside the program, the changelog, and
//! where to download the release. `.github/workflows/pages.yml` runs it and publishes the
//! result to GitHub Pages.
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
//! says so in the reader's own language instead of pretending otherwise.

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

fn build(repo: &Path, out: &Path) -> Result<usize, String> {
    let version = version_of(repo)?;
    let readme = read(repo, "README.md")?;
    let changelog = read(repo, "CHANGELOG.md")?;
    let landscape = read(repo, "docs/00-landscape.md")?;

    // What the site publishes, so a link to one of these documents becomes a link to its page
    // rather than one into the repository.
    let mut published: Vec<(&str, String)> = vec![
        ("CHANGELOG.md", "changelog.html".to_string()),
        ("docs/00-landscape.md", "commanders.html".to_string()),
        ("README.md", "index.html".to_string()),
    ];
    for (file, _) in HELP_PAGES {
        let name = file.trim_end_matches(".md");
        published.push((
            // The leaked string is the generator's own table living as long as it runs.
            Box::leak(format!("docs/help/{file}").into_boxed_str()),
            format!("help/{name}.html"),
        ));
    }

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
                body: landing(&lang, &version),
                translations: translations("index"),
                version: &version,
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
                version: &version,
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
                version: &version,
            },
        )?;
        pages += 1;

        write_page(
            out,
            Page {
                path: format!("download{}.html", suffix_of(lang.code)),
                title: lang.download_title,
                lang: &lang,
                body: download(&lang, &version),
                translations: translations("download"),
                version: &version,
            },
        )?;
        pages += 1;
    }

    // The long documents, once, in English — with the note on each saying so.
    write_page(
        out,
        Page {
            path: "commanders.html".to_string(),
            title: text::english().nav_commanders,
            lang: text::english(),
            body: document(
                text::english(),
                text::english().nav_commanders,
                &landscape,
                &published,
                0,
            ),
            translations: Vec::new(),
            version: &version,
        },
    )?;
    pages += 1;

    write_page(
        out,
        Page {
            path: "changelog.html".to_string(),
            title: text::english().nav_changelog,
            lang: text::english(),
            body: document(
                text::english(),
                text::english().nav_changelog,
                &changelog,
                &published,
                0,
            ),
            translations: Vec::new(),
            version: &version,
        },
    )?;
    pages += 1;

    // The help: an index, and the pages the program itself shows.
    let mut index = format!(
        "<h1>{}</h1>\n<p>{}</p>\n<ul class=\"pages\">\n",
        escape(text::english().help_title),
        escape(text::english().help_intro)
    );
    for (file, title) in HELP_PAGES {
        let name = file.trim_end_matches(".md");
        index.push_str(&format!(
            "<li><a href=\"{name}.html\">{}</a></li>\n",
            escape(title)
        ));
    }
    index.push_str("</ul>\n");
    write_page(
        out,
        Page {
            path: "help/index.html".to_string(),
            title: text::english().help_title,
            lang: text::english(),
            body: index,
            translations: Vec::new(),
            version: &version,
        },
    )?;
    pages += 1;

    for (file, title) in HELP_PAGES {
        let source = read(repo, &format!("docs/help/{file}"))?;
        let name = file.trim_end_matches(".md");
        write_page(
            out,
            Page {
                path: format!("help/{name}.html"),
                title,
                lang: text::english(),
                body: md::rewrite_links(&md::to_html(&source), &published, 1),
                translations: Vec::new(),
                version: &version,
            },
        )?;
        pages += 1;
    }

    Ok(pages)
}

/// The landing page: the promise, a picture of it, and the way in.
fn landing(lang: &Text, version: &str) -> String {
    let suffix = suffix_of(lang.code);
    let mut body = String::new();
    body.push_str(&format!(
        "<h1>FinalCommander</h1>\n<p class=\"tagline\">{}</p>\n<p class=\"pitch\">{}</p>\n",
        escape(lang.tagline),
        inline_code(lang.pitch)
    ));
    body.push_str(&format!(
        "<div class=\"buttons\">\
         <a href=\"download{suffix}.html\">{} {version}</a>\
         <a href=\"features{suffix}.html\">{}</a>\
         <a href=\"{}\">{}</a>\
         </div>\n",
        escape(lang.nav_download),
        escape(lang.nav_features),
        md::REPO,
        escape(lang.nav_source),
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
    let mut body = format!("<h1>{}</h1>\n", escape(title));
    if lang.code != "en" {
        body.push_str(&format!(
            "<p class=\"note\">{}</p>\n",
            escape(lang.in_english)
        ));
    }
    body.push_str(&md::rewrite_links(&md::to_html(markdown), published, depth));
    body
}

/// The download page: the archives of the latest release, named as the release names them.
fn download(lang: &Text, version: &str) -> String {
    let rows = [
        (
            "Linux (x86-64)",
            format!("fcmd-linux-x86_64-{version}.tar.gz"),
        ),
        (
            "Linux (x86-64, static)",
            format!("fcmd-linux-x86_64-static-{version}.tar.gz"),
        ),
        (
            "macOS (Intel and Apple Silicon)",
            format!("fcmd-macos-universal-{version}.tar.gz"),
        ),
        (
            "Windows (x86-64)",
            format!("fcmd-windows-x86_64-{version}.zip"),
        ),
    ];
    let mut body = format!(
        "<h1>{}</h1>\n<p>{}</p>\n<table>\n<tr><th>{}</th><th>{}</th></tr>\n",
        escape(lang.download_title),
        inline_code(lang.download_intro),
        escape(lang.download_platform),
        escape(lang.download_file),
    );
    for (platform, file) in rows {
        body.push_str(&format!(
            "<tr><td>{}</td><td><a href=\"{}/releases/latest/download/{file}\"><code>{file}</code></a></td></tr>\n",
            escape(platform),
            md::REPO,
        ));
    }
    body.push_str("</table>\n");
    body.push_str(&format!(
        "<p>{}</p>\n<pre><code>tar xzf fcmd-linux-x86_64-{version}.tar.gz\n\
         sudo install -m755 fcmd /usr/local/bin/fcmd\nfcmd --version</code></pre>\n",
        inline_code(lang.download_checksums)
    ));
    body.push_str(&format!(
        "<h2>{}</h2>\n<pre><code>git clone {}\ncd FinalCommander\n\
         cargo build --release\n</code></pre>\n",
        escape(lang.download_source),
        md::REPO,
    ));
    body.push_str(&format!(
        "<p><a href=\"{}/releases\">{}</a></p>\n",
        md::REPO,
        escape(lang.nav_download)
    ));
    body
}

/// The site's own strings carry `` `code` `` spans, because the sentences they are in are about
/// files and commands. This turns those into `<code>` and escapes everything else.
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

/// The workspace version, which is the version the site is about.
fn version_of(repo: &Path) -> Result<String, String> {
    let manifest = read(repo, "Cargo.toml")?;
    manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = "))
        .map(|value| value.trim().trim_matches('"').to_string())
        .ok_or_else(|| "Cargo.toml: no version".to_string())
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

        // The pages somebody arrives at exist in every language, and English has no suffix.
        for lang in LANGUAGES {
            for stem in ["index", "features", "replaces", "download"] {
                let path = format!("{stem}{}.html", suffix_of(lang.code));
                assert!(out.join(&path).is_file(), "{path} is missing");
            }
        }
        // The long ones exist once.
        for path in [
            "changelog.html",
            "commanders.html",
            "help/index.html",
            "style.css",
        ] {
            assert!(out.join(path).is_file(), "{path} is missing");
        }
        // Every help page the program shows is a page here.
        for (file, _) in HELP_PAGES {
            let name = file.trim_end_matches(".md");
            assert!(
                out.join(format!("help/{name}.html")).is_file(),
                "help/{name}.html is missing"
            );
        }
        // And the pictures came with it.
        assert!(out.join("docs/shots/panels.svg").is_file());
        assert!(out.join("assets/logo.svg").is_file());
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
    }

    #[test]
    fn the_download_page_names_the_archives_of_this_version() {
        let (_dir, out) = built();
        let version = version_of(&default_repo()).expect("a version");
        let page = page_text(&out, "download.html");
        for platform in ["linux-x86_64", "macos-universal", "windows-x86_64"] {
            let file = if platform == "windows-x86_64" {
                format!("fcmd-{platform}-{version}.zip")
            } else {
                format!("fcmd-{platform}-{version}.tar.gz")
            };
            assert!(page.contains(&file), "{file} is not offered: {page}");
        }
        assert!(page.contains("releases/latest/download/"), "{page}");
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

    #[test]
    fn nothing_on_the_site_is_fetched_from_somewhere_else() {
        let (_dir, out) = built();
        for path in [
            "index.html",
            "help/files.html",
            "changelog.html",
            "download.html",
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
