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

mod archive;
mod format;
mod limits;
mod write;

pub use archive::{ArcEntry, Archive, Cache};
pub use format::Format;
pub use limits::Limits;
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

    /// A guard fired: the archive is or claims to be far bigger than the limits allow. The
    /// message says which limit and what the archive wanted, because a user who meets this
    /// needs to decide whether to raise it.
    #[error("{path}: refused ({reason})")]
    Refused { path: String, reason: String },

    #[error("{path}: no entry {entry}")]
    NoEntry { path: String, entry: String },

    #[error("cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, ArcError>;
