//! The internal viewer: text, dump and hex, on files of any size.
//!
//! Position is a **byte offset**, never a line number, so opening a 40 GB log costs one read
//! and `End` costs one seek — there is no index to build and no line count to compute. See
//! `docs/spec/05-viewer-editor.md` §1.

pub mod asm;
pub mod decode;
pub mod disasm;
pub mod il;
pub mod jvm;
pub mod markdown;
pub mod nav;
pub mod search;
pub mod source;
pub mod structure;
pub mod view;

pub use asm::{assemble, AsmError, Assembled};
pub use decode::{detect, looks_binary, sniff_bom, Codepage};
pub use disasm::{
    analyze, disassemble, probe, resolve_targets, wasm_text, Arch, Binary, DisasmError, Format,
    Line, Target, TargetKind,
};
pub use search::{compile, Match, Pattern, SearchError, SearchMode};
pub use source::{BytesSource, FileSource, Source};
pub use view::{Mode, Row, RowKind, Started, Status, Viewer, Wrap};
