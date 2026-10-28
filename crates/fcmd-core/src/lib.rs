//! Core domain types shared by every other `sc-*` crate.
//!
//! This crate is deliberately dependency-light and knows nothing about terminals, async
//! runtimes or configuration files. See `docs/spec/01-architecture.md`.

pub mod calc;
pub mod cancel;
pub mod compare;
pub mod descriptions;
pub mod device;
pub mod entry;
pub mod error;
pub mod filter;
pub mod listing;
pub mod longpath;
pub mod selection;
pub mod size;
pub mod sort;
pub mod view;
pub mod vpath;

pub use cancel::CancelToken;
pub use device::{is_device, Length};
pub use entry::{Entry, EntryFlags, EntryId, EntryKind};
pub use error::{Error, Result};
pub use filter::{
    default_groups, parse_size, AttrConditions, AttrState, DateCondition, DateRange, Filter,
    FilterError, Groups, HardLinks, KindFilter, MaskSet, SizeRange, TimeField,
};
pub use listing::{Listing, Totals};
pub use longpath::{is_long, os_path};
pub use selection::{MaskScope, Selection};
pub use size::{allocated, format_count, format_size};
pub use sort::{Sort, SortKey};
pub use view::ViewMode;
pub use vpath::{Scheme, VPath};
