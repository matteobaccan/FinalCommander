//! The archive provider: what is inside a container, and how to get it out.
//!
//! The crate answers three questions and nothing else, so that the panel does not know how a
//! zip is laid out and the archive code does not know what a panel is (SPEC 04 §3, §4):
//!
//! - what format is this file ([`Format`]),
//! - what entries does it hold ([`Archive::open`], [`Archive::list`]),
//! - give me one of them ([`Archive::read`], [`Archive::extract`]).
//!
//! Reading and writing a whole archive ([`create`]); changing one that exists is the rebuild
//! of SPEC 04 §4 and is not here yet, so an archive opened for reading is never modified.
//!
//! A container that is not a file on this disk — one inside another archive, one on a server —
//! is copied here first and read from the copy; `nest` is where that happens and what it
//! costs.
//!
//! And one thing beside all that: `ext` drives an archiver that is already on the machine,
//! from a table of command lines, for the formats this program does not implement and the
//! operating system does. It is a port of FAR's MultiArc; `docs/05-multiarc.md` is its
//! specification.

mod archive;
mod cpio;
mod ext;
mod format;
pub mod json;
mod limits;
pub mod mail;
mod nest;
mod rebuild;
pub mod sqlite;
mod write;

pub use archive::{ArcEntry, Archive, Cache, Report};
pub use ext::{
    builtin_tools, claimed_by_a_tool, install_tools, merge, missing_program, tables, tool_for,
    which, Anchor, ExtError, Fill, Line, Listing, Magic, Op, Ran, Reader, Row, Run, Spawn, Tool,
};
pub use format::Format;
pub use limits::Limits;
pub use nest::Fetch;
pub use rebuild::{rebuild, rebuild_in, Change};
pub use write::{create, Level, Written};

/// What can go wrong with a container, in the words the UI shows.
#[derive(Debug, thiserror::Error)]
pub enum ArcError {
    #[error("{0}: not an archive, or a format we cannot read")]
    Unknown(String),

    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{path}: broken archive ({reason})")]
    Broken { path: String, reason: String },

    /// A program on the machine was asked to read this and did not. The reason is that
    /// program's own, because it is the only one that knows: a table cannot explain a format
    /// it does not parse (`docs/05-multiarc.md` §9).
    #[error("{path}: {reason}")]
    External { path: String, reason: String },

    /// A guard fired: the archive is or claims to be far bigger than the limits allow. The
    /// message says which limit and what the archive wanted, because a user who meets this
    /// needs to decide whether to raise it.
    #[error("{path}: refused ({reason})")]
    Refused { path: String, reason: String },

    #[error("{path}: no entry {entry}")]
    NoEntry { path: String, entry: String },

    /// The archive, or its header, is encrypted. Said as its own error because the answer is
    /// a password and not a different archive — and because asking for one is a dialog this
    /// does not have yet, so the message has to say why nothing opened.
    #[error("{path}: it is encrypted and wants a password")]
    NeedsPassword { path: String },

    #[error("cancelled")]
    Cancelled,
}

impl ArcError {
    /// Is this the refusal whose answer is a password?
    ///
    /// The application asks the error rather than reading its words, because what it does
    /// about this one is a dialog and not a message.
    pub fn wants_password(&self) -> bool {
        matches!(self, ArcError::NeedsPassword { .. })
    }
}

/// The same question of a message that has already lost its type — a listing comes back to
/// the panel as a string, and this is the one refusal the panel answers with a prompt. The
/// sentence is the one [`ArcError::NeedsPassword`] writes, which is why both live here.
pub fn wants_password(message: &str) -> bool {
    message.contains(NEEDS_PASSWORD)
}

/// The words that say a password is what is missing. Kept beside the error that writes them.
pub(crate) const NEEDS_PASSWORD: &str = "it is encrypted and wants a password";

/// Which program a listing failed for want of, when that is why it failed.
///
/// The same shape and the same reason as [`wants_password`]: by the time a listing reaches
/// the panel it is a string, and this is the one other refusal the panel has something of its
/// own to say about. "Install `7z`" is a sentence the reader can act on, in their own
/// language; "this needs `7z`, which is not installed" is not, being English.
pub fn needs_program(message: &str) -> Option<&str> {
    let rest = message.split_once(NEEDS_PROGRAM_BEFORE)?.1;
    Some(rest.split_once(NEEDS_PROGRAM_AFTER)?.0)
}

/// The two halves of that sentence, so the error that writes it and the function that reads
/// it cannot drift apart. A test asserts they still agree.
pub(crate) const NEEDS_PROGRAM_BEFORE: &str = "this needs `";
pub(crate) const NEEDS_PROGRAM_AFTER: &str = "`, which is not installed";

pub type Result<T> = std::result::Result<T, ArcError>;
