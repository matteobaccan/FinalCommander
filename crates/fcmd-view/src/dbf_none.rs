//! dBase tables, left out of this build (the compact one, `--no-default-features`): a `.dbf`
//! is shown as bytes, and the commands that would change one say why they do not.
//!
//! The type is here so that table mode is written once: it can be named, and never made.

use crate::table::Align;

/// Is the dBase reader in this build?
pub const BUILT_IN: bool = false;

/// A dBase table's header. [`Dbf::parse`] is the only way to one, and in this build it
/// never gives one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dbf {
    pub first: u64,
    pub record_len: usize,
    pub records: usize,
    pub flavour: &'static str,
}

impl Dbf {
    /// No table is read without the reader.
    pub fn parse(_head: &[u8]) -> Option<Dbf> {
        None
    }

    pub fn offset(&self, n: usize) -> u64 {
        self.first + (n as u64) * (self.record_len as u64)
    }

    pub(crate) fn columns(&self) -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn aligns(&self) -> Vec<Align> {
        Vec::new()
    }

    pub(crate) fn widths(&self) -> Vec<usize> {
        Vec::new()
    }

    pub(crate) fn cells(
        &self,
        _record: &[u8],
        _memo: Option<&mut (dyn crate::source::Source + '_)>,
    ) -> Vec<String> {
        Vec::new()
    }
}
