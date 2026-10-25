//! Keeping the syntax scan in step with a buffer that is being edited.
//!
//! [`fcmd_syntax`] colours one line given the state the line before it left behind, so
//! colouring line ten thousand means having scanned the nine thousand nine hundred and
//! ninety-nine above it. Doing that on every keystroke would make a large file unusable, so
//! the state at the start of each line is kept here, extended as the screen asks for lines
//! further down, and thrown away from the first line an edit touched.

use fcmd_syntax::{Span, State, Syntax};

use crate::buffer::Buffer;

/// The scan, and what it has remembered.
#[derive(Debug, Clone)]
pub struct Highlight {
    syntax: Option<&'static Syntax>,
    /// `states[i]` is the state line `i` *starts* in, so `states[0]` is always the start of
    /// the file and the vector is never empty.
    states: Vec<State>,
}

impl Default for Highlight {
    fn default() -> Self {
        Highlight {
            syntax: None,
            states: vec![State::default()],
        }
    }
}

impl Highlight {
    /// The syntax a file's name implies. A name we know nothing about colours nothing, which
    /// costs nothing to ask for.
    pub fn for_name(name: &str) -> Highlight {
        Highlight {
            syntax: fcmd_syntax::of_name(name),
            states: vec![State::default()],
        }
    }

    /// What this is being read as, for the status line.
    pub fn language(&self) -> Option<&'static str> {
        self.syntax.map(|s| s.name)
    }

    pub fn is_on(&self) -> bool {
        self.syntax.is_some()
    }

    /// Read the file as something else, or as nothing at all.
    pub fn set_syntax(&mut self, syntax: Option<&'static Syntax>) {
        self.syntax = syntax;
        self.forget_from(0);
    }

    /// Everything from `line` on may have changed.
    pub fn forget_from(&mut self, line: usize) {
        self.states.truncate(line + 1);
        if self.states.is_empty() {
            self.states.push(State::default());
        }
    }

    /// The coloured runs of one line, scanning whatever is needed to get there.
    pub fn spans(&mut self, buffer: &Buffer, line: usize) -> Vec<Span> {
        let Some(syntax) = self.syntax else {
            return Vec::new();
        };
        if line >= buffer.len() {
            return Vec::new();
        }
        // Catch up, remembering each line's starting state as we pass it.
        while self.states.len() <= line {
            let at = self.states.len() - 1;
            let state = self.states[at];
            let (_, next) = syntax.line(buffer.line(at), state);
            self.states.push(next);
        }
        let (spans, _) = syntax.line(buffer.line(line), self.states[line]);
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_syntax::Class;

    fn buffer(text: &str) -> Buffer {
        Buffer::from_text(text)
    }

    #[test]
    fn a_line_is_coloured_in_the_state_the_lines_above_left() {
        let mut h = Highlight::for_name("main.c");
        assert_eq!(h.language(), Some("C"));
        let buf = buffer("int a = 1;\n/* open\nstill\n*/ int b = 2;\n");
        // `int` is a type in C, and `1` a number.
        assert_eq!(h.spans(&buf, 0).first().map(|s| s.class), Some(Class::Type));
        // Line two is inside the comment only because line one opened it.
        let spans = h.spans(&buf, 2);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].class, Class::Comment);
        assert_eq!(h.spans(&buf, 3)[0].class, Class::Comment);
        // And the scan stopped where it was asked to.
        assert_eq!(h.states.len(), 4);
    }

    /// The cache is the whole point, and a cache that is not thrown away is a bug you see:
    /// delete the line that opened a comment and everything below must stop being one.
    #[test]
    fn an_edit_throws_away_what_was_scanned_below_it() {
        let mut h = Highlight::for_name("main.c");
        let mut buf = buffer("/* open\nstill a comment\nand still\n");
        assert_eq!(h.spans(&buf, 2)[0].class, Class::Comment);
        assert_eq!(h.states.len(), 3);

        buf.delete_line(0).unwrap();
        let touched = buf.take_touched().expect("the buffer says what changed");
        h.forget_from(touched);
        assert_eq!(
            h.states.len(),
            1,
            "the states below the edit are still cached"
        );
        assert!(
            h.spans(&buf, 1).is_empty(),
            "the comment outlived the line that opened it"
        );
    }

    #[test]
    fn a_file_we_know_nothing_about_colours_nothing() {
        let mut h = Highlight::for_name("notes.unknown");
        assert!(!h.is_on());
        assert_eq!(h.language(), None);
        assert!(h.spans(&buffer("fn main() {}\n"), 0).is_empty());

        // And it can be told what to read it as.
        h.set_syntax(fcmd_syntax::of_name("x.rs"));
        assert_eq!(h.language(), Some("Rust"));
        assert!(!h.spans(&buffer("fn main() {}\n"), 0).is_empty());
    }

    #[test]
    fn a_line_past_the_end_is_not_a_panic() {
        let mut h = Highlight::for_name("a.rs");
        assert!(h.spans(&buffer("let x = 1;\n"), 99).is_empty());
    }
}
