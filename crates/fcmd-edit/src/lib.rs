//! The internal editor: a line-based buffer with undo, blocks, search and an atomic save.
//!
//! Line-based rather than a rope because that is what this class of editor does (mcedit, FAR),
//! it keeps column blocks and line operations simple, and the size limit is explicit: above
//! [`buffer::MAX_OPEN`] a file opens read-only instead of being pulled into memory silently.
//! See `docs/spec/05-viewer-editor.md` §2.

pub mod buffer;
pub mod editor;
pub mod editorconfig;
pub mod hex;
pub mod highlight;

pub use buffer::{Buffer, BufferError, Eol, Position, Range, MAX_OPEN, UNDO_DEPTH};
pub use editor::{BlockKind, Case, Editor, Hit, SaveOutcome};
pub use hex::{Change, HexEdit, HexError, Hit as HexHit, Pane};
pub use highlight::Highlight;
