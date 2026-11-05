//! A JSON document as a folder (FarNet's JsonKit, SPEC 04 §4c).
//!
//! `Enter` on a `.json` walks it the way a panel walks a disk: an object or an array is a
//! folder, a key is a name — an array's items are `[0]`, `[1]`, … — and a value that is not
//! a container is a file. A string's file holds the string itself, so a value that is a
//! script, a certificate or a path reads as what it is; any other value holds its JSON. Read
//! only, like the other containers that are documents rather than archives.

use std::path::Path;

use fcmd_core::{CancelToken, EntryKind};
use serde_json::Value;

use crate::archive::ArcEntry;
use crate::limits::Limits;
use crate::{ArcError, Result};

/// The largest document opened as a folder. Past this a JSON file is data to be queried,
/// not a tree to walk, and the viewer is the better place for it.
pub const LIMIT: u64 = 64 * 1024 * 1024;

fn read(path: &Path, name: &str) -> Result<Value> {
    let meta = std::fs::metadata(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    if meta.len() > LIMIT {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!(
                "it is over {} MiB, which is a lot of JSON to hold",
                LIMIT >> 20
            ),
        });
    }
    let bytes = std::fs::read(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    serde_json::from_slice(&bytes).map_err(|e| ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    })
}

/// The children of a container, each with the name it has in the panel. Names are unique:
/// a key that would collide with another once `/` is replaced gets `~2`, `~3`.
pub fn children(value: &Value) -> Vec<(String, &Value)> {
    match value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("[{i}]"), v))
            .collect(),
        Value::Object(map) => {
            let mut out: Vec<(String, &Value)> = Vec::with_capacity(map.len());
            for (key, v) in map {
                let base = match key.replace(['/', '\0'], "_") {
                    k if k.is_empty() => "(empty)".to_string(),
                    k if k == "." || k == ".." => format!("({k})"),
                    k => k,
                };
                let mut name = base.clone();
                let mut n = 2;
                while out.iter().any(|(m, _)| *m == name) {
                    name = format!("{base}~{n}");
                    n += 1;
                }
                out.push((name, v));
            }
            out
        }
        _ => Vec::new(),
    }
}

/// What a leaf's file holds.
pub fn leaf_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::String(s) => s.as_bytes().to_vec(),
        other => serde_json::to_vec_pretty(other).unwrap_or_default(),
    }
}

fn is_container(v: &Value) -> bool {
    matches!(v, Value::Array(_) | Value::Object(_))
}

pub fn index(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let doc = read(path, name)?;
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut out = Vec::new();
    let mut stack: Vec<(String, &Value)> = vec![(String::new(), &doc)];
    while let Some((prefix, value)) = stack.pop() {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        for (child, v) in children(value) {
            if out.len() >= limits.entries {
                return Err(ArcError::Refused {
                    path: name.to_string(),
                    reason: format!("it holds more than {} values", limits.entries),
                });
            }
            let at = format!("{prefix}/{child}");
            let dir = is_container(v);
            out.push(ArcEntry {
                path: at.clone(),
                size: if dir { 0 } else { leaf_bytes(v).len() as u64 },
                packed: None,
                mtime,
                mode: Some(if dir { 0o555 } else { 0o444 }),
                kind: if dir { EntryKind::Dir } else { EntryKind::File },
                link_target: None,
                at: None,
            });
            if dir {
                stack.push((at, v));
            }
        }
    }
    Ok(out)
}

/// The value at `/a/[2]/b`, walked by the same names the index gave.
pub fn lookup<'a>(doc: &'a Value, entry: &str) -> Option<&'a Value> {
    let mut at = doc;
    for part in entry.split('/').filter(|p| !p.is_empty()) {
        at = children(at).into_iter().find(|(n, _)| n == part)?.1;
    }
    Some(at)
}

pub fn copy_out(path: &Path, name: &str, entry: &str, out: &mut dyn std::io::Write) -> Result<u64> {
    let doc = read(path, name)?;
    let value = lookup(&doc, entry).ok_or_else(|| ArcError::NoEntry {
        path: name.to_string(),
        entry: entry.to_string(),
    })?;
    let bytes = leaf_bytes(value);
    out.write_all(&bytes).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    Ok(bytes.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_and_arrays_are_folders_and_values_are_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.json");
        std::fs::write(
            &file,
            r#"{"name":"fcmd","tags":["a","b"],"deps":{"x/y":{"v":1}},"ok":true,"":0}"#,
        )
        .unwrap();
        let entries = index(&file, "a.json", &Limits::default(), &CancelToken::new()).unwrap();
        let mut paths: Vec<(String, EntryKind)> =
            entries.iter().map(|e| (e.path.clone(), e.kind)).collect();
        paths.sort();
        let want = [
            ("/(empty)", EntryKind::File),
            ("/deps", EntryKind::Dir),
            ("/deps/x_y", EntryKind::Dir),
            ("/deps/x_y/v", EntryKind::File),
            ("/name", EntryKind::File),
            ("/ok", EntryKind::File),
            ("/tags", EntryKind::Dir),
            ("/tags/[0]", EntryKind::File),
            ("/tags/[1]", EntryKind::File),
        ];
        let want: Vec<(String, EntryKind)> =
            want.iter().map(|(p, k)| (p.to_string(), *k)).collect();
        assert_eq!(paths, want);
        let mut out = Vec::new();
        copy_out(&file, "a.json", "/name", &mut out).unwrap();
        assert_eq!(out, b"fcmd", "a string is itself");
        out.clear();
        copy_out(&file, "a.json", "/deps/x_y/v", &mut out).unwrap();
        assert_eq!(out, b"1");
        let size = entries.iter().find(|e| e.path == "/ok").unwrap().size;
        assert_eq!(size, 4, "true");
    }

    #[test]
    fn broken_json_says_where() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("b.json");
        std::fs::write(&file, "{\"a\": 1,").unwrap();
        let err = index(&file, "b.json", &Limits::default(), &CancelToken::new()).unwrap_err();
        assert!(err.to_string().contains("line 1"), "{err}");
    }
}
