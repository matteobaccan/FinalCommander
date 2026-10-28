//! The find-files engine: which files match, and nothing about the screen.
//!
//! `docs/spec/06-tools.md` §1 is the specification. This crate is the half of it that has no
//! opinion about dialogs: a [`Query`] goes in, [`Event`]s come out through a callback as they
//! are found, and a [`Stats`] says what the search did. The dialog, the keys, the result list
//! and panelizing are the caller's.
//!
//! Three things it deliberately does not do, because something else here already does:
//!
//! - **matching names, sizes, dates and attributes** is [`Filter`](fcmd_core::Filter) from
//!   `fcmd-core`, mask grammar and all. There is one mask matcher in this project;
//! - **listing a directory** goes through the VFS ([`FindFs`], implemented for
//!   `fcmd_vfs::LocalVfs` in terms of `fcmd_vfs::local`), so a remote panel becomes one more
//!   `impl` rather than a second walker;
//! - **code pages** are [`Codepage`](fcmd_view::Codepage) from the viewer, so "search in this
//!   code page" means in find what it means in the viewer's `F7`.
//!
//! ```no_run
//! use fcmd_core::{CancelToken, Filter, MaskSet, VPath};
//! use fcmd_find::{find, Content, Event, Needle, Query};
//! use fcmd_vfs::LocalVfs;
//!
//! let query = Query::under(VPath::local("/usr/include"))
//!     .with_filter(Filter::from_masks(MaskSet::parse("*.h").unwrap()))
//!     .containing(Content::containing(Needle::text("SOCK_STREAM")));
//!
//! let cancel = CancelToken::new();
//! let stats = find(&LocalVfs, &query, &cancel, &mut |event| match event {
//!     Event::Hit(hit) => println!("{}", hit.path),
//!     Event::Scanning(dir) => eprintln!("… {dir}"),
//!     Event::Problem(p) => eprintln!("{}: {}", p.path, p.message),
//! })
//! .unwrap();
//! println!("{} files, {} bytes read", stats.files_seen, stats.bytes_read);
//! ```
//!
//! **Memory.** A search costs the directories still to visit, the directories already
//! visited, and one fixed buffer per file. The buffer is a constant plus the length of the
//! needle; the length a file claims for itself never enters into it, so a 40 GB log and a
//! truncated one are read the same way.
//!
//! **Nothing is collected.** The engine never builds the result list, so a caller that wants
//! to show hits while the search is still running simply does, and a caller that wants a
//! `Vec` builds one in its own callback.

mod arc;
mod content;
mod fs;
mod walk;

pub use arc::{ArcFind, BIGGEST};
pub use content::{Content, ContentError, Needle};
pub use fs::{FindFs, VisitId};
pub use walk::{find, Event, Hit, Problem, Query, Stats, Trouble};
