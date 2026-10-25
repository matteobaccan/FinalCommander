//! A real terminal, inside the program.
//!
//! Running a command and keeping its output is one thing, and the command line under the
//! panels has done it from the start. Running `vim`, `top` or `ssh` is another: those
//! programs ask the operating system what they are talking to, lay themselves out from its
//! size, and draw with escape sequences that move a cursor about. They need a *terminal*, and
//! this crate is one — a pseudo-terminal from [`pty`] and, reading what comes out of it, the
//! grid in [`screen`].
//!
//! The two halves are separable on purpose: [`screen::Screen`] is a pure function of the bytes
//! it is given, so what a sequence does is tested by handing it the sequence, with no shell,
//! no thread and no timing in the way. [`pty::Terminal`] is the part that cannot be tested
//! that way, and it is deliberately thin.

pub mod pty;
pub mod screen;

pub use pty::{default_shell, TermError, Terminal};
pub use screen::{Screen, SCROLLBACK};
