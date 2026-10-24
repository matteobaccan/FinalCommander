//! The command vocabulary and the keymap that resolves keys to it.
//!
//! Nothing in this crate touches a terminal or a filesystem: it turns a [`Key`] plus a scope
//! stack into a [`Command`], and that is all. See `docs/03-keymap.md`.

pub mod command;
pub mod key;
pub mod keymap;

pub use command::{Command, Side};
pub use key::{Key, KeyCode, KeyParseError, Mods};
pub use keymap::{Keymap, KeymapError, Scope};
