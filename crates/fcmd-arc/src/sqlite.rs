//! An SQLite database as a folder (SPEC 04 §4b).
//!
//! FAR's SQLiteDB plugin shows a database's tables as a panel and a table as rows. Here the
//! database is one more read-only container, so everything a panel already does with one
//! works: `Enter` goes in, each table and view is an entry `name.csv` whose bytes are the
//! rows as CSV — so `F3` opens it in the viewer's table mode, `F5` writes it out as a `.csv`,
//! `Alt+F7` searches it — and `schema.sql` holds the statements that made it all.
//!
//! The database is opened read-only and never written: not even its journal is touched
//! (`SQLITE_OPEN_READ_ONLY`, and `immutable` is *not* used, so a database another program is
//! writing is read in a consistent state rather than half-way).

use std::io::Write;
use std::path::Path;

use fcmd_core::{CancelToken, EntryKind};
use rusqlite::{types::ValueRef, Connection, OpenFlags};

use crate::archive::ArcEntry;
use crate::limits::Limits;
use crate::{ArcError, Result};

/// The entry that holds the schema.
pub const SCHEMA: &str = "schema.sql";

/// The first sixteen bytes of every SQLite 3 database.
pub const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Does this file start the way an SQLite database starts?
pub fn is_database(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 16];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && &head == MAGIC
}

fn open(path: &Path, name: &str) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| broken(name, e))
}

fn broken(name: &str, e: impl std::fmt::Display) -> ArcError {
    ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    }
}

/// The tables and views, in the order SQLite lists them, without its own `sqlite_*` tables.
pub fn objects(conn: &Connection, name: &str) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn
        .prepare(
            "SELECT type, name, COALESCE(sql, '') FROM sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|e| broken(name, e))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| broken(name, e))?;
    rows.collect::<std::result::Result<_, _>>()
        .map_err(|e| broken(name, e))
}

/// Every table and view as `name.csv`, and `schema.sql`. The size of a table's entry is
/// the size of its CSV, which means reading it once here: a panel that showed a wrong size
/// would be lying, and `F3` reads the entry by its size.
pub fn index(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let conn = open(path, name)?;
    let objects = objects(&conn, name)?;
    if objects.len() > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!("it holds more than {} tables", limits.entries),
        });
    }
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut out = Vec::with_capacity(objects.len() + 1);
    let schema = schema_text(&objects);
    out.push(ArcEntry {
        path: format!("/{SCHEMA}"),
        size: schema.len() as u64,
        packed: None,
        mtime,
        mode: Some(0o444),
        kind: EntryKind::File,
        link_target: None,
        at: None,
    });
    for (_, object, _) in &objects {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let mut counter = Counter(0);
        write_csv(&conn, name, &select_all(object), &mut counter, cancel)?;
        out.push(ArcEntry {
            path: format!("/{}", entry_name(object)),
            size: counter.0,
            packed: None,
            mtime,
            mode: Some(0o444),
            kind: EntryKind::File,
            link_target: None,
            at: None,
        });
    }
    Ok(out)
}

/// The bytes of one entry: the schema, or a table's rows as CSV.
pub fn copy_out(
    path: &Path,
    name: &str,
    entry: &str,
    out: &mut dyn Write,
    cancel: &CancelToken,
) -> Result<u64> {
    let conn = open(path, name)?;
    let entry = entry.trim_start_matches('/');
    let objects = objects(&conn, name)?;
    if entry == SCHEMA {
        let text = schema_text(&objects);
        out.write_all(text.as_bytes()).map_err(|e| ArcError::Io {
            path: name.to_string(),
            source: e,
        })?;
        return Ok(text.len() as u64);
    }
    let Some((_, object, _)) = objects.iter().find(|(_, o, _)| entry_name(o) == entry) else {
        return Err(ArcError::NoEntry {
            path: name.to_string(),
            entry: entry.to_string(),
        });
    };
    let mut counted = Counting { inner: out, n: 0 };
    write_csv(&conn, name, &select_all(object), &mut counted, cancel)?;
    Ok(counted.n)
}

/// Run a statement of the user's and write what it returns as CSV: the `sql:` prefix.
///
/// The connection is read-only, so a statement that would change the database is refused by
/// SQLite itself — `attempt to write a readonly database` — rather than by a guess here
/// about which words are safe.
pub fn query(path: &Path, sql: &str, out: &mut dyn Write, cancel: &CancelToken) -> Result<u64> {
    let name = path.display().to_string();
    let conn = open(path, &name)?;
    let mut counted = Counting { inner: out, n: 0 };
    write_csv(&conn, &name, sql, &mut counted, cancel)?;
    Ok(counted.n)
}

/// Make a database from a script, for the tests of the crates above this one, which do not
/// depend on SQLite themselves.
#[doc(hidden)]
pub fn create_for_tests(path: &Path, script: &str) {
    Connection::open(path)
        .and_then(|c| c.execute_batch(script))
        .expect("a test database");
}

/// What a table is called as an entry. A `/` in a table's name would make a folder of it.
pub fn entry_name(object: &str) -> String {
    format!("{}.csv", object.replace('/', "_"))
}

fn select_all(object: &str) -> String {
    format!("SELECT * FROM \"{}\"", object.replace('"', "\"\""))
}

fn schema_text(objects: &[(String, String, String)]) -> String {
    let mut text = String::new();
    for (_, _, sql) in objects {
        if !sql.is_empty() {
            text.push_str(sql);
            text.push_str(";\n\n");
        }
    }
    text
}

/// The rows of a statement as CSV, RFC 4180: a header of the column names, fields quoted when
/// they hold a comma, a quote or a line break, a blob as `x'…'` hex as SQLite writes one.
fn write_csv(
    conn: &Connection,
    name: &str,
    sql: &str,
    out: &mut dyn Write,
    cancel: &CancelToken,
) -> Result<()> {
    let io = |e: std::io::Error| ArcError::Io {
        path: name.to_string(),
        source: e,
    };
    let mut stmt = conn.prepare(sql).map_err(|e| broken(name, e))?;
    let columns: Vec<String> = stmt.column_names().map(|c| c.to_string()).collect();
    let header: Vec<String> = columns.iter().map(|c| field(c)).collect();
    writeln!(out, "{}", header.join(",")).map_err(io)?;
    let mut rows = stmt.query([]).map_err(|e| broken(name, e))?;
    let mut line = String::new();
    while let Some(row) = rows.next().map_err(|e| broken(name, e))? {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        line.clear();
        for i in 0..columns.len() {
            if i > 0 {
                line.push(',');
            }
            match row.get_ref(i).map_err(|e| broken(name, e))? {
                ValueRef::Null => {}
                ValueRef::Integer(n) => line.push_str(&n.to_string()),
                ValueRef::Real(x) => line.push_str(&x.to_string()),
                ValueRef::Text(t) => line.push_str(&field(&String::from_utf8_lossy(t))),
                ValueRef::Blob(b) => {
                    line.push_str("x'");
                    for byte in b {
                        line.push_str(&format!("{byte:02x}"));
                    }
                    line.push('\'');
                }
            }
        }
        line.push('\n');
        out.write_all(line.as_bytes()).map_err(io)?;
    }
    Ok(())
}

fn field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// A writer that only counts.
struct Counter(u64);

impl Write for Counter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Counting<'a> {
    inner: &'a mut dyn Write,
    n: u64,
}

impl Write for Counting<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.n += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small database: two tables, one with a text that needs quoting and a blob, a view.
    pub fn sample(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT, note TEXT, pic BLOB);
             INSERT INTO people VALUES (1, 'Ada', 'first, of all', x'0102');
             INSERT INTO people VALUES (2, 'Linus', 'said \"hi\"', NULL);
             CREATE TABLE empty (x);
             CREATE VIEW names AS SELECT name FROM people;",
        )
        .unwrap();
    }

    #[test]
    fn tables_and_views_are_csv_entries_with_their_real_size() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        sample(&db);
        let entries = index(&db, "t.db", &Limits::default(), &CancelToken::new()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            names,
            ["/schema.sql", "/empty.csv", "/names.csv", "/people.csv"]
        );
        let mut csv = Vec::new();
        let n = copy_out(&db, "t.db", "/people.csv", &mut csv, &CancelToken::new()).unwrap();
        let text = String::from_utf8(csv).unwrap();
        assert_eq!(
            text,
            "id,name,note,pic\n1,Ada,\"first, of all\",x'0102'\n2,Linus,\"said \"\"hi\"\"\",\n"
        );
        assert_eq!(n, entries[3].size, "the index said the size the bytes have");
        let mut schema = Vec::new();
        copy_out(&db, "t.db", "/schema.sql", &mut schema, &CancelToken::new()).unwrap();
        assert!(String::from_utf8(schema)
            .unwrap()
            .contains("CREATE VIEW names"));
    }

    #[test]
    fn a_query_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        sample(&db);
        let mut out = Vec::new();
        query(
            &db,
            "SELECT count(*) AS n FROM people",
            &mut out,
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "n\n2\n");
        let err = query(
            &db,
            "DELETE FROM people",
            &mut Vec::new(),
            &CancelToken::new(),
        );
        assert!(err.is_err(), "the connection cannot write");
        let conn = Connection::open(&db).unwrap();
        let n: i64 = conn
            .query_row("SELECT count(*) FROM people", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }
}
