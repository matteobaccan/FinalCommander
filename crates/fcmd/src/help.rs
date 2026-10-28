//! The help: a book with chapters, `F1` from anywhere (SPEC 00 §4).
//!
//! Half of it is written prose, under `docs/help/`, compiled into the binary so the program
//! carries its own manual and a release is still one file. The other half is **generated from
//! what the program is actually running** — the keymap as loaded, every command in the
//! vocabulary, every setting with its current value — because a hand-written list of keys is
//! wrong the day after it is written, and a help page that lies is worse than none.
//!
//! The prose is wrapped by the renderer rather than by hand — the same page has to read on an
//! eighty-column terminal and on a wide one — and the renderer is the viewer's own Markdown
//! view, so a table in a page is drawn as a table rather than forbidden.

/// A chapter. `Prose` is a file under `docs/help/`; the rest are built at the moment they are
/// opened, from the live keymap, the command list and the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chapter {
    /// The list of chapters.
    Index,
    Prose(usize),
    Keys,
    Commands,
    Options,
    /// The keys of one place, with what each one does: what `F1` opens, chosen from where
    /// you were standing when you pressed it.
    Here(Where),
}

/// Which screen the help was asked from. FAR's help is contextual, and the reason is that
/// the question behind `F1` is almost never "tell me about this program" — it is "what do
/// the keys do *here*".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    Panels,
    Viewer,
    Editor,
    Hex,
    Dialog,
}

impl Where {
    /// The message id of this place's name. An id rather than the words, because the help
    /// is read in the language the program is running in.
    pub fn title_id(self) -> &'static str {
        match self {
            Where::Panels => "help-here-panels",
            Where::Viewer => "help-here-viewer",
            Where::Editor => "help-here-editor",
            Where::Hex => "help-here-hex",
            Where::Dialog => "help-here-dialog",
        }
    }

    /// The scopes whose keys apply here, innermost first — the same stack the keymap
    /// resolves through, so the page cannot describe a key the program would not obey.
    pub fn scopes(self) -> &'static [fcmd_cmd::Scope] {
        use fcmd_cmd::Scope;
        match self {
            Where::Panels => &[Scope::Panel, Scope::Global],
            Where::Viewer => &[Scope::Viewer, Scope::Global],
            Where::Editor => &[Scope::Editor, Scope::Global],
            Where::Hex => &[Scope::Hex, Scope::Editor, Scope::Global],
            Where::Dialog => &[Scope::Dialog, Scope::Global],
        }
    }

    /// The written chapter that explains this place at length, by the chapter's own name —
    /// which is the file's stem and does not move when a title is translated.
    pub fn prose(self) -> Option<usize> {
        let id = match self {
            Where::Panels | Where::Dialog => "panels",
            Where::Viewer => "viewer",
            Where::Editor | Where::Hex => "editor",
        };
        PROSE.iter().position(|p| p.id == id)
    }
}

/// One written chapter: its own name, the message id of the title the index shows, and the
/// text in each language that has one.
///
/// The chapters are written in English and translated after the fact, so a language with no
/// translation of a chapter yet reads the English — with a line in its own language saying
/// so, rather than an index in Italian that silently opens an English page.
pub struct Prose {
    /// The file's stem: `panels`, `files`, `config`. Stable, and what the code matches on.
    pub id: &'static str,
    pub title_id: &'static str,
    /// `(language tag, text)`, English first and always present.
    pub texts: &'static [(&'static str, &'static str)],
}

impl Prose {
    /// The text to show in `lang`, and whether it is the English standing in for a
    /// translation that does not exist yet.
    pub fn text(&self, lang: &str) -> (&'static str, bool) {
        match self.texts.iter().find(|(code, _)| *code == lang) {
            Some((_, text)) => (text, false),
            None => (self.english(), lang != "en"),
        }
    }

    pub fn english(&self) -> &'static str {
        self.texts
            .iter()
            .find(|(code, _)| *code == "en")
            .map(|(_, text)| *text)
            .unwrap_or_default()
    }
}

/// The written chapters, in the order the index lists them — which is the order someone meets
/// the program in, not alphabetical.
pub const PROSE: &[Prose] = &[
    Prose {
        id: "about",
        title_id: "help-chapter-about",
        texts: &[
            ("en", include_str!("../../../docs/help/about.md")),
            ("it", include_str!("../../../docs/help/it/about.md")),
        ],
    },
    Prose {
        id: "panels",
        title_id: "help-chapter-panels",
        texts: &[
            ("en", include_str!("../../../docs/help/panels.md")),
            ("it", include_str!("../../../docs/help/it/panels.md")),
        ],
    },
    Prose {
        id: "files",
        title_id: "help-chapter-files",
        texts: &[
            ("en", include_str!("../../../docs/help/files.md")),
            ("it", include_str!("../../../docs/help/it/files.md")),
        ],
    },
    Prose {
        id: "viewer",
        title_id: "help-chapter-viewer",
        texts: &[
            ("en", include_str!("../../../docs/help/viewer.md")),
            ("it", include_str!("../../../docs/help/it/viewer.md")),
        ],
    },
    Prose {
        id: "editor",
        title_id: "help-chapter-editor",
        texts: &[
            ("en", include_str!("../../../docs/help/editor.md")),
            ("it", include_str!("../../../docs/help/it/editor.md")),
        ],
    },
    Prose {
        id: "network",
        title_id: "help-chapter-network",
        texts: &[
            ("en", include_str!("../../../docs/help/network.md")),
            ("it", include_str!("../../../docs/help/it/network.md")),
        ],
    },
    Prose {
        id: "config",
        title_id: "help-chapter-config",
        texts: &[
            ("en", include_str!("../../../docs/help/config.md")),
            ("it", include_str!("../../../docs/help/it/config.md")),
        ],
    },
];

/// Every chapter, in index order. The generated ones come last: they are reference, and
/// reference is what one comes back for rather than what one starts with.
pub fn chapters(i18n: &fcmd_i18n::I18n) -> Vec<(Chapter, String)> {
    let mut out: Vec<(Chapter, String)> = PROSE
        .iter()
        .enumerate()
        .map(|(i, p)| (Chapter::Prose(i), i18n.t(p.title_id)))
        .collect();
    out.push((Chapter::Keys, i18n.t("help-chapter-keys")));
    out.push((Chapter::Commands, i18n.t("help-chapter-commands")));
    out.push((Chapter::Options, i18n.t("help-chapter-options")));
    out
}

/// Render a chapter to lines ready to draw, each carrying the runs that say how to colour it.
///
/// The pages are Markdown, and the viewer already has a Markdown view — headings with a rule
/// under them, bullets, tables drawn with box characters, the inline marks taken off and the
/// text underneath coloured instead. The help used to carry a little renderer of its own that
/// did a third of that; this is the viewer's, so a page looks the same whether one reads it
/// with `F1` or opens the file with `F3`, and the pages may now hold a table.
pub fn rows(text: &str, width: usize) -> Vec<fcmd_view::markdown::Line> {
    fcmd_view::markdown::render(text, width.clamp(20, 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_written_chapter_is_there_and_has_a_heading() {
        assert!(!PROSE.is_empty());
        for page in PROSE {
            assert!(!page.id.is_empty() && !page.title_id.is_empty());
            // Every language's copy of a chapter, English included, has to be a help page:
            // opening with a heading and carrying no table. A translation that drifted into
            // something else is a page that does not render.
            for (lang, text) in page.texts {
                assert!(
                    text.starts_with("# "),
                    "{}/{lang} does not open with a heading",
                    page.id
                );
            }
            // English is the one every language falls back to, so it must be there.
            assert!(!page.english().is_empty(), "{} has no English", page.id);
        }
        // The index names every chapter, written and generated.
        let i18n = fcmd_i18n::I18n::new("en");
        assert_eq!(chapters(&i18n).len(), PROSE.len() + 3);
    }

    #[test]
    fn a_chapter_nobody_has_translated_is_the_english_one_and_says_so() {
        let page = &PROSE[0];
        // English asks for English and is not told anything.
        assert_eq!(page.text("en"), (page.english(), false));
        // A language with no translation of this chapter reads the English, and the caller
        // is told to say so in the reader's own language.
        let (text, in_english) = page.text("ru");
        assert_eq!(text, page.english());
        assert!(in_english);
    }

    #[test]
    fn a_page_is_rendered_the_way_the_viewer_renders_markdown() {
        let text = "# A heading\n\nA paragraph that is\nwritten over two lines and has to come \
                    back together before it is wrapped again.\n\n    sftp://me@host/srv\n\n\
                    - a bullet long enough to need a second line of its own when the terminal \
                    is narrow\n\n| key | does |\n| --- | --- |\n| F5 | copy |\n";
        let lines = rows(text, 40);
        let text_of: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        // A heading is its words with a rule under it, not an ALL CAPS line.
        assert_eq!(text_of[0], "A heading");
        assert!(text_of[1].starts_with('═'), "{:?}", text_of[1]);
        for line in &text_of {
            assert!(
                line.chars().count() <= 40 || line.starts_with("    "),
                "too wide: {line:?}"
            );
        }
        // The indented block is untouched, URL and all.
        assert!(text_of.contains(&"    sftp://me@host/srv"));
        // The two lines of the paragraph became one paragraph and were wrapped again.
        let para = text_of
            .iter()
            .find(|l| l.starts_with("A paragraph"))
            .unwrap();
        assert!(para.chars().count() > 20, "{para:?}");
        // The bullet is a bullet, and a table is drawn rather than left as pipes.
        assert!(text_of.iter().any(|l| l.starts_with("• a bullet")));
        assert!(text_of.iter().any(|l| l.contains('┌')), "{text_of:?}");
        // And every line says how to colour itself, which is what the dialog draws from.
        assert!(lines.iter().any(|l| l
            .spans
            .iter()
            .any(|s| s.class == fcmd_view::markdown::Class::Heading)));
    }

    #[test]
    fn the_marks_a_page_writes_are_not_shown() {
        let lines = rows("Press `F5` to copy, which is **the** point.", 80);
        assert_eq!(
            lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>(),
            vec!["Press F5 to copy, which is the point."]
        );
    }

    /// A path too long for the window is broken rather than left hanging off the edge, and a
    /// path one is meant to copy belongs in an indented block, where nothing touches it —
    /// which is where the pages put them.
    #[test]
    fn a_narrow_terminal_breaks_prose_and_leaves_a_block_alone() {
        let lines = rows(
            "a /very/long/path/that/cannot/be/broken/anywhere sits here\n\n    \
             /very/long/path/that/cannot/be/broken/anywhere\n",
            20,
        );
        for line in &lines {
            assert!(
                line.text.chars().count() <= 20 || line.text.starts_with("    "),
                "too wide: {:?}",
                line.text
            );
        }
        assert!(lines
            .iter()
            .any(|l| l.text == "    /very/long/path/that/cannot/be/broken/anywhere"));
    }
}
