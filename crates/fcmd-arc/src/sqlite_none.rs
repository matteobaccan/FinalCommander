//! SQLite, left out of this build (the compact one, `--no-default-features`): a database is
//! an ordinary file, opened as bytes, and the `sql:` prefix says why it has no answer.

use std::io::Write;
use std::path::Path;

use fcmd_core::CancelToken;

use crate::archive::ArcEntry;
use crate::limits::Limits;
use crate::{ArcError, Result};

/// The entry that would hold the schema.
pub const SCHEMA: &str = "schema.sql";

/// The first sixteen bytes of every SQLite 3 database.
pub const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Never a database here, so it is never entered as a folder.
pub fn is_database(_path: &Path) -> bool {
    false
}

fn absent(name: &str) -> ArcError {
    ArcError::Refused {
        path: name.to_string(),
        reason: "SQLite is not in this build of FinalCommander".into(),
    }
}

/// No tables to list.
pub fn index(
    _path: &Path,
    name: &str,
    _limits: &Limits,
    _cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    Err(absent(name))
}

/// No rows to copy out.
pub fn copy_out(
    _path: &Path,
    name: &str,
    _entry: &str,
    _out: &mut dyn Write,
    _cancel: &CancelToken,
) -> Result<u64> {
    Err(absent(name))
}

/// No statement to run.
pub fn query(path: &Path, _sql: &str, _out: &mut dyn Write, _cancel: &CancelToken) -> Result<u64> {
    Err(absent(&path.display().to_string()))
}

/// What a table would be called as an entry.
pub fn entry_name(object: &str) -> String {
    format!("{}.csv", object.replace('/', "_"))
}
