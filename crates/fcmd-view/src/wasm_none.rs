//! WebAssembly, left out of this build (the compact one, `--no-default-features`): a module
//! is shown as bytes, and `F5` says why there is no listing.

use crate::structure::Structure;

/// No text format without `wasmprinter`.
pub fn listing(_bytes: &[u8]) -> Result<String, String> {
    Err("WebAssembly is not in this build of FinalCommander".into())
}

/// No sections without `wasmparser`.
pub(crate) fn read(_bytes: &[u8], _len: u64) -> Option<Structure> {
    None
}
