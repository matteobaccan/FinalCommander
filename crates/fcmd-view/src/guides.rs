//! The lines the Delphi IDE draws down the left of a block, from the word that opens it to the
//! `end` that closes it: one per open block, in the column where the opening word starts.
//!
//! Worked out from the words alone, line by line, with the colouring's own scanner so that a
//! `begin` in a comment or a string opens nothing. It is a reading for the eye, not a parser:
//! a file that does not balance shows lines that do not either, which is also what the IDE does.

use fcmd_syntax::{Class, State, Syntax};

/// One line's part in the blocks: the columns a guide passes through, and the column where a
/// block that closes on this line had its guide, for the little hook at its foot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Guides {
    /// Columns where a guide runs down past this line.
    pub through: Vec<usize>,
    /// Columns where a guide ends on this line, at its `end`.
    pub ends: Vec<usize>,
}

/// Whether a file's syntax has blocks worth a guide: Pascal and the forms written in it.
pub fn applies(syntax: &Syntax) -> bool {
    matches!(syntax.name, "Pascal" | "Delphi form")
}

/// The guides of `lines[from..]`, after reading the lines before `from` for the blocks that
/// are already open when the first of them starts.
pub fn guides(syntax: &Syntax, lines: &[String], from: usize) -> Vec<Guides> {
    let form = syntax.name == "Delphi form";
    let mut stack: Vec<(usize, &'static str)> = Vec::new();
    let mut state = State::default();
    let mut out = Vec::with_capacity(lines.len().saturating_sub(from));
    for (n, line) in lines.iter().enumerate() {
        let (spans, next) = syntax.line(line, state);
        state = next;
        let open_before: Vec<usize> = stack.iter().map(|(c, _)| *c).collect();
        let mut ends = Vec::new();
        // The IDE hangs a block's line from the start of the line that opens it — the
        // `while` of `while … do begin`, the name of `TFoo = class(…)` — not from the word.
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        for span in spans.iter().filter(|s| s.class == Class::Keyword) {
            let word = line[span.start..span.end].to_ascii_lowercase();
            let rest = line[span.end..].trim_start();
            let opens: Option<&'static str> = if form {
                match word.as_str() {
                    "object" | "inherited" | "inline" | "item" => Some("end"),
                    _ => None,
                }
            } else {
                match word.as_str() {
                    "begin" | "try" | "case" | "asm" => Some("end"),
                    "repeat" => Some("until"),
                    // A type that is declared here and not only named: `= class(TObject)`
                    // with a body, `= record`; not `class;`, `class of`, `class function`.
                    "class" | "record" | "object" | "interface" | "dispinterface"
                        if line[..span.start].trim_end().ends_with('=')
                            || line[..span.start].trim_end().ends_with("packed") =>
                    {
                        let closed_here = rest.starts_with(';')
                            || rest.to_ascii_lowercase().starts_with("of ")
                            || (rest.starts_with('(') && rest.trim_end().ends_with(';'));
                        (!closed_here).then_some("end")
                    }
                    _ => None,
                }
            };
            if let Some(closer) = opens {
                stack.push((indent, closer));
            } else if word == "end" || word == "until" {
                // The nearest block this word closes; a stray `end` closes nothing.
                if let Some(at) = stack.iter().rposition(|(_, c)| *c == word) {
                    let (column, _) = stack.remove(at);
                    stack.truncate(at);
                    ends.push(column);
                }
            }
        }
        if n >= from {
            // A guide runs past this line when its block was open before it and still is,
            // and only through what is blank: it never draws over text.
            let still: Vec<usize> = stack.iter().map(|(c, _)| *c).collect();
            let blank = |c: usize| line.chars().nth(c).is_none_or(char::is_whitespace);
            let through = open_before
                .iter()
                .copied()
                .filter(|c| still.contains(c) && blank(*c))
                .collect();
            let ends = ends
                .into_iter()
                .filter(|c| open_before.contains(c))
                .collect();
            out.push(Guides { through, ends });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, text: &str) -> Vec<Guides> {
        let syntax = fcmd_syntax::of_name(name).expect("a syntax");
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        guides(syntax, &lines, 0)
    }

    /// The IDE's picture: a line from `begin` down to its `end`, one per nested block, in
    /// the opening word's column, and none through a comment that says `begin`.
    #[test]
    fn a_guide_runs_from_each_begin_to_its_end() {
        let g = run(
            "Unit1.pas",
            "begin\n  try\n    x := 1; // begin\n  finally\n    y;\n  end;\nend;\n",
        );
        assert_eq!(g[0], Guides::default(), "the opening line itself has none");
        assert_eq!(g[1].through, [0], "inside begin");
        assert_eq!(
            g[2].through,
            [0, 2],
            "inside begin and try; the comment opens nothing"
        );
        assert_eq!(g[3].through, [0], "`finally` stands in try's column");
        assert_eq!(g[4].through, [0, 2]);
        assert_eq!(g[5].ends, [2], "try ends here");
        assert_eq!(g[6].ends, [0], "and begin here");
    }

    /// A class declared with a body opens a block; one only named, or of a class type, not.
    #[test]
    fn a_class_with_a_body_is_a_block_and_a_forward_one_is_not() {
        let g = run(
            "Unit1.pas",
            "type\n  TFoo = class;\n  TBar = class(TObject)\n    X: Integer;\n  end;\n",
        );
        assert!(g[2].through.is_empty(), "the forward one closed itself");
        assert_eq!(g[3].through, [2], "inside TBar's class, under its name");
        assert_eq!(g[4].ends, [2]);
    }

    /// A form's objects nest the same way.
    #[test]
    fn a_form_draws_a_guide_per_object() {
        let g = run(
            "Main.fmx",
            "object Form1: TForm1\n  Left = 0\n  object Button1: TButton\n    Text = 'Ok'\n  end\nend\n",
        );
        assert_eq!(g[1].through, [0]);
        assert_eq!(g[3].through, [0, 2]);
        assert_eq!(g[4].ends, [2]);
        assert_eq!(g[5].ends, [0]);
    }
}
