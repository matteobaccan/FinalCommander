//! Core domain types shared by every other `sc-*` crate.
//!
//! This crate is deliberately dependency-light and knows nothing about terminals, async
//! runtimes or configuration files. See `docs/spec/01-architecture.md`.

pub mod cancel;
pub mod entry;
pub mod error;
pub mod listing;
pub mod selection;
pub mod size;
pub mod sort;
pub mod view;
pub mod vpath;

pub use cancel::CancelToken;
pub use entry::{Entry, EntryFlags, EntryId, EntryKind};
pub use error::{Error, Result};
pub use listing::{Listing, Totals};
pub use selection::{MaskScope, Selection};
pub use size::{format_count, format_size};
pub use sort::{Sort, SortKey};
pub use view::ViewMode;
pub use vpath::{Scheme, VPath};
