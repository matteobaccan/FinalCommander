//! The help: a book with chapters, `F1` from anywhere (SPEC 00 §4).
//!
//! Half of it is written prose, under `docs/help/`, compiled into the binary so the program
//! carries its own manual and a release is still one file. The other half is **generated from
//! what the program is actually running** — the keymap as loaded, every command in the
//! vocabulary, every setting with its current value — because a hand-written list of keys is
//! wrong the day after it is written, and a help page that lies is worse than none.
//!
//! The prose is deliberately table-free and wrapped by the renderer rather than by hand: the
//! same page has to read on an eighty-column terminal and on a wide one.

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
    pub fn title(self) -> &'static str {
        match self {
            Where::Panels => "The panels",
            Where::Viewer => "The viewer",
            Where::Editor => "The editor",
            Where::Hex => "The hex editor",
            Where::Dialog => "This dialog",
        }
    }

    /// The scopes whose keys apply here, innermost first — the same stack the keymap
    /// resolves through, so the page cannot describe a key the program would not obey.
    pub fn scopes(self) -> &'static [scmd_cmd::Scope] {
        use scmd_cmd::Scope;
        match self {
            Where::Panels => &[Scope::Panel, Scope::Global],
            Where::Viewer => &[Scope::Viewer, Scope::Global],
            Where::Editor => &[Scope::Editor, Scope::Global],
            Where::Hex => &[Scope::Hex, Scope::Editor, Scope::Global],
            Where::Dialog => &[Scope::Dialog, Scope::Global],
        }
    }

    /// The written chapter that explains this place at length.
    pub fn prose(self) -> Option<usize> {
        let title = match self {
            Where::Panels | Where::Dialog => "The panels",
            Where::Viewer => "The viewer",
            Where::Editor | Where::Hex => "The editor, the hex pane and the assembler",
        };
        PROSE.iter().position(|p| p.title == title)
    }
}

/// One written chapter: the title the index shows, and the text.
pub struct Prose {
    pub title: &'static str,
    pub text: &'static str,
}

/// The written chapters, in the order the index lists them — which is the order someone meets
/// the program in, not alphabetical.
pub const PROSE: &[Prose] = &[
    Prose {
        title: "What this is",
        text: include_str!("../../../docs/help/about.md"),
    },
    Prose {
        title: "The panels",
        text: include_str!("../../../docs/help/panels.md"),
    },
    Prose {
        title: "Files: copy, move, find, archives",
        text: include_str!("../../../docs/help/files.md"),
    },
    Prose {
        title: "The viewer",
        text: include_str!("../../../docs/help/viewer.md"),
    },
    Prose {
        title: "The editor, the hex pane and the assembler",
        text: include_str!("../../../docs/help/editor.md"),
    },
    Prose {
        title: "Servers",
        text: include_str!("../../../docs/help/network.md"),
    },
    Prose {
        title: "Configuration, keys and colours",
        text: include_str!("../../../docs/help/config.md"),
    },
];

/// Every chapter, in index order. The generated ones come last: they are reference, and
/// reference is what one comes back for rather than what one starts with.
pub fn chapters() -> Vec<(Chapter, String)> {
    let mut out: Vec<(Chapter, String)> = PROSE
        .iter()
        .enumerate()
        .map(|(i, p)| (Chapter::Prose(i), p.title.to_string()))
        .collect();
    out.push((Chapter::Keys, "Keys, as they are bound now".to_string()));
    out.push((Chapter::Commands, "Every command, and its keys".to_string()));
    out.push((Chapter::Options, "Every setting, and its value".to_string()));
    out
}

/// Render a chapter's text to lines of at most `width` columns.
///
/// It understands the little of Markdown the pages use — a `#` heading, a `- ` bullet, an
/// indented block left as it is — and nothing else, on purpose: a help page is read, not
/// typeset, and a renderer with a grammar would be a renderer with bugs.
pub fn render(text: &str, width: usize) -> Vec<String> {
    let width = width.clamp(20, 100);
    let mut out: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end();
        // An indented line is shown as it stands: that is how the pages show a path, a URL or
        // a fragment of TOML, and wrapping one would ruin it.
        if line.starts_with("    ") {
            out.push(line.to_string());
            continue;
        }
        if line.is_empty() {
            // One blank line between paragraphs, never two.
            if out.last().map(|l| !l.is_empty()).unwrap_or(false) {
                out.push(String::new());
            }
            continue;
        }
        if let Some(heading) = line.strip_prefix("# ") {
            if out.last().map(|l| !l.is_empty()).unwrap_or(false) {
                out.push(String::new());
            }
            let heading = strip_marks(heading).to_uppercase();
            out.push(heading);
            continue;
        }
        if let Some(item) = line.strip_prefix("- ") {
            wrap_into(&mut out, &strip_marks(item), width, "  ", "    ");
            continue;
        }
        // A paragraph: glue its lines together, then wrap to the width we have.
        let text = strip_marks(line);
        match out.last_mut() {
            Some(last) if !last.is_empty() && !last.starts_with(' ') && !is_heading(last) => {
                let joined = format!("{last} {text}");
                out.pop();
                wrap_into(&mut out, &joined, width, "", "");
            }
            _ => wrap_into(&mut out, &text, width, "", ""),
        }
    }
    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    out
}

/// A rendered heading: all capitals and no leading space, which is how `render` draws one.
fn is_heading(line: &str) -> bool {
    !line.is_empty() && line.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
}

/// The inline marks the pages use, taken off: `` `code` `` and `**bold**`. The terminal has
/// its own ways of emphasising, and none of them is a backtick.
fn strip_marks(text: &str) -> String {
    text.replace("**", "").replace('`', "")
}

/// Wrap `text` into `out`, `first` in front of the first line and `rest` in front of the
/// others. Wrapping is by words; a word longer than the width goes on a line of its own
/// rather than being cut, because a cut path is a path nobody can use.
fn wrap_into(out: &mut Vec<String>, text: &str, width: usize, first: &str, rest: &str) {
    let mut line = String::from(first);
    let mut any = false;
    for word in text.split_whitespace() {
        let room = width.saturating_sub(line.chars().count());
        let needed = word.chars().count() + usize::from(any);
        if any && needed > room {
            out.push(std::mem::take(&mut line));
            line.push_str(rest);
            any = false;
        }
        if any {
            line.push(' ');
        }
        line.push_str(word);
        any = true;
    }
    if any {
        out.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_written_chapter_is_there_and_has_a_heading() {
        assert!(!PROSE.is_empty());
        for page in PROSE {
            assert!(!page.title.is_empty());
            assert!(
                page.text.starts_with("# "),
                "{} does not open with a heading",
                page.title
            );
            // The pages are written to be wrapped, so none of them may carry a table: a
            // table in a help page is a table that does not fit on somebody's terminal.
            assert!(
                !page.text.contains("| --- |"),
                "{} has a table in it",
                page.title
            );
        }
        // The index names every chapter, written and generated.
        assert_eq!(chapters().len(), PROSE.len() + 3);
    }

    #[test]
    fn rendering_wraps_to_the_width_and_keeps_what_must_not_move() {
        let text = "# A heading\n\nA paragraph that is\nwritten over two lines and has to come \
                    back together before it is wrapped again.\n\n    sftp://me@host/srv\n\n\
                    - a bullet long enough to need a second line of its own when the terminal \
                    is narrow\n";
        let lines = render(text, 40);
        assert_eq!(lines[0], "A HEADING");
        for line in &lines {
            assert!(
                line.chars().count() <= 40 || line.starts_with("    "),
                "too wide: {line:?}"
            );
        }
        // The indented line is untouched, URL and all.
        assert!(lines.iter().any(|l| l == "    sftp://me@host/srv"));
        // The two lines of the paragraph became one paragraph, so the first wrapped line is
        // full rather than ending where the source line did.
        let para = lines.iter().find(|l| l.starts_with("A paragraph")).unwrap();
        assert!(para.chars().count() > 20, "{para:?}");
        // The bullet is indented, and its continuation more so.
        let bullet = lines.iter().position(|l| l.contains("a bullet")).unwrap();
        assert!(lines[bullet].starts_with("  a bullet"));
        assert!(lines[bullet + 1].starts_with("    "));
    }

    #[test]
    fn the_marks_a_page_writes_are_not_shown() {
        let lines = render("Press `F5` to copy, which is **the** point.", 80);
        assert_eq!(lines, vec!["Press F5 to copy, which is the point."]);
    }

    #[test]
    fn a_narrow_terminal_still_gets_whole_words() {
        let lines = render(
            "a /very/long/path/that/cannot/be/broken/anywhere sits here",
            20,
        );
        assert!(lines
            .iter()
            .any(|l| l.contains("/very/long/path/that/cannot/be/broken/anywhere")));
    }
}
