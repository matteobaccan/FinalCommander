//! `--web-terminal`, left out of this build (the compact one, `--no-default-features`): the
//! option is still understood, and says why it does nothing.

use std::io;
use std::path::PathBuf;

/// What the program in the terminal would be started with.
// Filled in by the caller as in the full build, and read by nothing here.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// No server in this build.
pub fn run(_addr: &str, _program: Program) -> io::Result<()> {
    Err(io::Error::other(
        "the web terminal is not in this build of FinalCommander",
    ))
}

/// The arguments, unchanged: there is nothing to start them with.
pub fn passed_on(args: impl IntoIterator<Item = String>) -> Vec<String> {
    args.into_iter().collect()
}
