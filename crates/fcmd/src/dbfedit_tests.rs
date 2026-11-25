// Writing a dBase table from table mode (SPEC 05 §1.11): the dialogs, in the order the
// author asked for them, and what is on disk after each answer.
//
// Included into the application's own test module (see the end of `app.rs`), which is where
// `app()`, `press` and `type_text` live.

use super::*;
use fcmd_view::dbf_write::Stamp;

/// The moment in the author's own example of a backup's name.
const THEN: Stamp = Stamp {
    year: 2026,
    month: 11,
    day: 25,
    hour: 8,
    minute: 30,
    second: 15,
};

/// `NAME C(10)`, `BORN D`, `NOTE M`: three records, the second deleted, in Windows-1252 so
/// that an accented letter has a byte to go to.
fn table_bytes() -> Vec<u8> {
    let fields: [(&str, u8, u8); 3] = [("NAME", b'C', 10), ("BORN", b'D', 8), ("NOTE", b'M', 10)];
    let header_len = 32 + fields.len() * 32 + 1;
    let record_len = 1 + 10 + 8 + 10;
    let mut out = vec![0x83, 99, 1, 15];
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(header_len as u16).to_le_bytes());
    out.extend_from_slice(&(record_len as u16).to_le_bytes());
    let mut rest = [0u8; 20];
    rest[29 - 12] = 0x03;
    out.extend_from_slice(&rest);
    for (name, kind, len) in fields {
        let mut raw = [0u8; 32];
        raw[..name.len()].copy_from_slice(name.as_bytes());
        raw[11] = kind;
        raw[16] = len;
        out.extend_from_slice(&raw);
    }
    out.push(0x0d);
    for (flag, record) in [
        (b' ', "mb        19750312          "),
        (b'*', "gone                        "),
        (b' ', "ada       18151210          "),
    ] {
        out.push(flag);
        out.extend_from_slice(record.as_bytes());
    }
    out.push(0x1a);
    out
}

/// A folder with `CLIENTI.DBF` and its memo file in it, and the viewer open on the table
/// with the clock stopped at [`THEN`].
fn open_table() -> (tempfile::TempDir, PathBuf, App) {
    let dir = tempfile::tempdir().unwrap();
    let table = dir.path().join("CLIENTI.DBF");
    std::fs::write(&table, table_bytes()).unwrap();
    std::fs::write(dir.path().join("CLIENTI.DBT"), b"the memos").unwrap();
    let mut app = app();
    app.dbf.clock = Some(THEN);
    app.view_path(&table.display().to_string());
    let mode = app.live_viewer_mut().map(|v| v.mode);
    assert_eq!(
        mode,
        Some(fcmd_view::Mode::Table),
        "a .dbf opens as a table"
    );
    (dir, table, app)
}

/// Hand these requests to the worker's own function, the way the event loop does, and the
/// answers back to the application. How many were about a table.
fn answer(app: &mut App, requests: Vec<Request>) -> usize {
    let mut done = 0;
    for request in requests {
        if let Request::Dbf(ask) = request {
            let result = dbf_perform(&ask);
            app.handle_event(Event::Dbf {
                table: ask.table.clone(),
                result,
            });
            done += 1;
        }
    }
    done
}

/// Do the work the application has asked for so far.
fn work(app: &mut App) -> usize {
    let requests = app.take_requests();
    answer(app, requests)
}

/// The names in the table's folder, sorted.
fn folder(dir: &tempfile::TempDir) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// What the record under the cursor shows, field by field.
fn cells(app: &mut App) -> Vec<String> {
    app.live_viewer_mut()
        .and_then(|v| v.dbf_here())
        .map(|here| here.cells)
        .unwrap_or_default()
}

#[test]
fn the_first_change_asks_for_a_backup_makes_it_and_then_writes() {
    let (dir, table, mut app) = open_table();
    let before = std::fs::read(&table).unwrap();

    // `Ctrl+F4` on the first field: the warning, and nothing else yet.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+F4");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(matches!(
        confirm.action,
        ConfirmAction::Dbf(DbfStep::Backup { .. })
    ));
    let said = confirm.lines.join(" ");
    assert!(said.contains("CLIENTI.DBF"), "{said}");
    assert!(said.contains("CLIENTI.20261125-083015.DBF"), "{said}");
    assert_eq!(
        confirm.buttons(&app.i18n),
        [app.i18n.t("btn-dbf-backup"), app.i18n.t("btn-cancel")]
    );
    assert_eq!(
        work(&mut app),
        0,
        "nothing is asked of the disk before the answer"
    );
    assert_eq!(folder(&dir), ["CLIENTI.DBF", "CLIENTI.DBT"]);

    // Yes: the copies are made — the table's and its memo file's, the extension last — and
    // only then is the value asked for.
    press(&mut app, "Enter");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1);
    assert_eq!(
        folder(&dir),
        [
            "CLIENTI.20261125-083015.DBF",
            "CLIENTI.20261125-083015.DBT",
            "CLIENTI.DBF",
            "CLIENTI.DBT"
        ]
    );
    let copy = dir.path().join("CLIENTI.20261125-083015.DBF");
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    assert_eq!(std::fs::read(&table).unwrap(), before, "not written yet");
    let Mode::Prompt(prompt) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(
        prompt.kind,
        PromptKind::DbfCell {
            record: 0,
            field: 0
        }
    );
    // The question names the field, its type and the record, and offers what is there.
    assert!(prompt.title.contains("NAME") && prompt.title.contains("C 10"));
    assert_eq!(prompt.text, "mb");

    press(&mut app, "Backspace");
    press(&mut app, "Backspace");
    type_text(&mut app, "matteo");
    press(&mut app, "Enter");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1);
    assert!(!app.status_error, "{}", app.status);
    assert!(app.status.contains("NAME"), "{}", app.status);

    // On disk: the field, padded; the header's date; the same length and count.
    let after = std::fs::read(&table).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(&after[1..8], [126, 11, 25, 3, 0, 0, 0]);
    let first = u16::from_le_bytes([after[8], after[9]]) as usize;
    assert_eq!(&after[first..first + 11], b" matteo    ");
    assert_eq!(after[first + 11..], before[first + 11..]);
    // And on screen, without the viewer being opened again.
    assert_eq!(cells(&mut app)[0], "matteo");

    // The second change of the session goes straight to its question.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+F4");
    let Mode::Prompt(prompt) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(prompt.text, "1975-03-12");
    // A date that is not one is refused where it is typed, and nothing is asked.
    type_text(&mut app, "x");
    press(&mut app, "Enter");
    assert!(app.status_error);
    assert_eq!(app.status, app.i18n.t("dbf-not-date"));
    assert_eq!(work(&mut app), 0);
    assert_eq!(std::fs::read(&table).unwrap(), after);
    assert_eq!(folder(&dir).len(), 4, "one backup for the session");
}

#[test]
fn refusing_the_backup_writes_nothing() {
    let (dir, table, mut app) = open_table();
    let before = std::fs::read(&table).unwrap();
    // Every one of the four asks first, and each way of saying no is a no.
    for (key, no) in [
        ("Ctrl+F4", vec!["Esc"]),
        ("Ctrl+N", vec!["Right", "Enter"]),
        ("Ctrl+F8", vec!["n"]),
        ("Shift+Del", vec!["F10"]),
    ] {
        press(&mut app, key);
        assert!(
            matches!(app.mode, Mode::Confirm(_)),
            "{key}: {:?}",
            app.mode
        );
        for answer in no {
            press(&mut app, answer);
        }
        // The viewer is back, on the table, and nothing went to a worker.
        assert!(matches!(app.mode, Mode::Viewer(_)), "{key}: {:?}", app.mode);
        assert_eq!(work(&mut app), 0, "{key}");
        assert_eq!(folder(&dir), ["CLIENTI.DBF", "CLIENTI.DBT"], "{key}");
        assert_eq!(std::fs::read(&table).unwrap(), before, "{key}");
    }
    // And having said no, one is asked again the next time.
    press(&mut app, "Ctrl+F8");
    assert!(matches!(app.mode, Mode::Confirm(_)));
}

#[test]
fn records_are_appended_deleted_recalled_and_packed() {
    let (dir, table, mut app) = open_table();
    let rows = |app: &mut App| app.live_viewer_mut().map(|v| v.table_rows());

    // Delete the first record: the backup first, then the flag.
    press(&mut app, "Ctrl+F8");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the backup");
    assert_eq!(work(&mut app), 1, "the flag");
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-deleted", &[("record", 1.into())])
    );
    let deleted = app.live_viewer_mut().and_then(|v| v.dbf_here());
    assert!(deleted.is_some_and(|here| here.deleted));
    // The same key recalls it, with no question this time.
    press(&mut app, "Ctrl+F8");
    assert_eq!(work(&mut app), 1);
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-recalled", &[("record", 1.into())])
    );

    // A new record: empty, counted, and under the cursor to be filled in.
    press(&mut app, "Ctrl+N");
    assert_eq!(work(&mut app), 1);
    assert_eq!(rows(&mut app), Some(4));
    let here = app.live_viewer_mut().and_then(|v| v.dbf_here()).unwrap();
    assert_eq!((here.record, here.cells), (3, vec![String::new(); 3]));
    press(&mut app, "Ctrl+F4");
    type_text(&mut app, "caffè");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1);
    assert_eq!(cells(&mut app)[0], "caffè");
    let bytes = std::fs::read(&table).unwrap();
    assert_eq!(bytes[4], 4, "the header counts four");
    assert_eq!(bytes.last(), Some(&0x1a));

    // Pack counts first, and says how many before it removes them.
    press(&mut app, "Shift+Del");
    assert_eq!(work(&mut app), 1, "the count");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(matches!(
        confirm.action,
        ConfirmAction::Dbf(DbfStep::Pack { .. })
    ));
    let said = confirm.lines.join(" ");
    assert!(
        said.contains("CLIENTI.DBF") && said.contains(": 1."),
        "{said}"
    );
    assert_eq!(std::fs::read(&table).unwrap(), bytes, "asked, not done");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the pack");
    assert!(!app.status_error, "{}", app.status);
    assert_eq!(rows(&mut app), Some(3));
    let packed = std::fs::read(&table).unwrap();
    assert_eq!(packed[4], 3);
    assert_eq!(packed.len(), bytes.len() - 29);
    // The viewer reads the new file: the record that was third is second now.
    press(&mut app, "Ctrl+Home");
    press(&mut app, "Down");
    assert_eq!(cells(&mut app)[0], "ada");
    // Nothing is left of the pack beside the table, and one backup was made.
    assert_eq!(
        folder(&dir),
        [
            "CLIENTI.20261125-083015.DBF",
            "CLIENTI.20261125-083015.DBT",
            "CLIENTI.DBF",
            "CLIENTI.DBT"
        ]
    );

    // With nothing deleted there is nothing to ask.
    press(&mut app, "Shift+Del");
    assert_eq!(work(&mut app), 1);
    assert!(matches!(app.mode, Mode::Viewer(_)));
    assert_eq!(app.status, app.i18n.t("dbf-pack-nothing"));
    assert_eq!(std::fs::read(&table).unwrap(), packed);
}

/// The question about a table's indexes that is on screen: what it says, and the files it
/// is about.
fn index_question(app: &App) -> (String, Vec<String>) {
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    let ConfirmAction::Dbf(DbfStep::Indexes { files, .. }) = &confirm.action else {
        panic!("{:?}", confirm.action)
    };
    assert_eq!(
        confirm.buttons(&app.i18n),
        [app.i18n.t("btn-dbf-go-on"), app.i18n.t("btn-cancel")]
    );
    let names = files
        .iter()
        .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    (confirm.lines.join(" "), names)
}

#[test]
fn the_indexes_named_after_the_table_are_asked_about_once_and_then_written_past() {
    let (dir, table, mut app) = open_table();
    let before = std::fs::read(&table).unwrap();
    let at = |name: &str| dir.path().join(name);
    // The table's own name, its name with something after it — and another table's.
    std::fs::write(at("CLIENTI.CDX"), b"the index").unwrap();
    std::fs::write(at("clienti_nome.ntx"), b"by name").unwrap();
    std::fs::write(at("FORNIT.NTX"), b"another table's").unwrap();

    // The backup first, as for any table; it takes the indexes with it, dated alike.
    press(&mut app, "Ctrl+F8");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(matches!(
        confirm.action,
        ConfirmAction::Dbf(DbfStep::Backup { .. })
    ));
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the backup");
    assert_eq!(
        folder(&dir),
        [
            "CLIENTI.20261125-083015.CDX",
            "CLIENTI.20261125-083015.DBF",
            "CLIENTI.20261125-083015.DBT",
            "CLIENTI.CDX",
            "CLIENTI.DBF",
            "CLIENTI.DBT",
            "FORNIT.NTX",
            "clienti_nome.20261125-083015.ntx",
            "clienti_nome.ntx"
        ]
    );

    // Right after it, the indexes: named, with what has to happen to them afterwards. The
    // copies the backup has just made are not among them, and neither is the other table's.
    let (said, files) = index_question(&app);
    assert_eq!(files, ["CLIENTI.CDX", "clienti_nome.ntx"]);
    assert!(said.contains("CLIENTI.CDX") && said.contains("clienti_nome.ntx"));
    assert!(!said.contains("FORNIT"), "{said}");
    assert!(said.contains("reindexed"), "{said}");
    assert_eq!(work(&mut app), 0, "asked, and nothing done");

    // Cancel: nothing is written, and the question is asked again the next time — but not
    // the one about the backup, which is made.
    press(&mut app, "Esc");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 0);
    assert_eq!(std::fs::read(&table).unwrap(), before);
    press(&mut app, "Ctrl+F8");
    assert_eq!(index_question(&app).1.len(), 2);

    // An index that arrives while the question stands is not one that was agreed to: the
    // answer is not taken, and the question comes back with it named.
    std::fs::write(at("CLIENTI2.NTX"), b"new").unwrap();
    press(&mut app, "Enter");
    let (said, files) = index_question(&app);
    assert_eq!(files, ["CLIENTI.CDX", "CLIENTI2.NTX", "clienti_nome.ntx"]);
    assert!(said.contains("CLIENTI2.NTX"), "{said}");
    assert_eq!(work(&mut app), 0);
    assert_eq!(std::fs::read(&table).unwrap(), before);

    // Go on: the record is written, and the indexes are left exactly as they were.
    press(&mut app, "Enter");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1, "the flag");
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-deleted", &[("record", 1.into())])
    );
    let first = u16::from_le_bytes([before[8], before[9]]) as usize;
    assert_eq!(std::fs::read(&table).unwrap()[first], b'*');
    assert_eq!(std::fs::read(at("CLIENTI.CDX")).unwrap(), b"the index");
    assert_eq!(std::fs::read(at("clienti_nome.ntx")).unwrap(), b"by name");

    // Asked once for the table in a session: the next change goes straight through.
    press(&mut app, "Ctrl+F8");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1);
    assert_eq!(std::fs::read(&table).unwrap()[first], b' ');

    // And an index made by another program between the key and the write is seen by the
    // worker, which looks again before it writes: nothing is written, and it is asked about.
    press(&mut app, "Ctrl+F8");
    let requests = app.take_requests();
    std::fs::write(at("CLIENTI3.IDX"), b"newer").unwrap();
    assert_eq!(answer(&mut app, requests), 1);
    assert_eq!(std::fs::read(&table).unwrap()[first], b' ', "not written");
    let (said, files) = index_question(&app);
    assert_eq!(files.len(), 4);
    assert!(said.contains("CLIENTI3.IDX"), "{said}");
    press(&mut app, "Enter");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 0, "the key is pressed again by whoever pressed it");
    press(&mut app, "Ctrl+F8");
    assert_eq!(work(&mut app), 1);
    assert_eq!(std::fs::read(&table).unwrap()[first], b'*');
}

#[test]
fn every_way_of_writing_a_table_asks_about_its_indexes_first() {
    for key in ["Ctrl+F4", "Ctrl+N", "Ctrl+F8", "Shift+Del", "Ctrl+D", "Ctrl+R"] {
        let (dir, table, mut app) = open_table();
        let before = std::fs::read(&table).unwrap();
        std::fs::write(dir.path().join("clienti1.ntx"), b"the index").unwrap();
        press(&mut app, "Ins");
        press(&mut app, key);
        press(&mut app, "Enter");
        assert_eq!(work(&mut app), 1, "{key}: the backup");
        let (_, files) = index_question(&app);
        assert_eq!(files, ["clienti1.ntx"], "{key}");
        // No: nothing reaches a worker, and the table is as it was.
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Viewer(_)), "{key}: {:?}", app.mode);
        assert_eq!(work(&mut app), 0, "{key}");
        assert_eq!(std::fs::read(&table).unwrap(), before, "{key}");
    }
    // An index of another table is nobody's question here.
    let (dir, _table, mut app) = open_table();
    std::fs::write(dir.path().join("ORDINI.NTX"), b"another table's").unwrap();
    std::fs::write(dir.path().join("XCLIENTI.CDX"), b"another table's").unwrap();
    press(&mut app, "Ctrl+F8");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the backup");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1, "the flag, with nothing asked");
}

#[test]
fn what_cannot_be_written_is_said_before_any_dialog() {
    let (dir, table, mut app) = open_table();
    // The memo field is shown and not written, and says so without a backup.
    for _ in 0..3 {
        press(&mut app, "Ctrl+Right");
    }
    press(&mut app, "Ctrl+F4");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert!(app.status_error);
    assert_eq!(
        app.status,
        app.i18n
            .t_args("dbf-read-only-type", &[("type", "M".into())])
    );
    assert_eq!(work(&mut app), 0);
    assert_eq!(folder(&dir), ["CLIENTI.DBF", "CLIENTI.DBT"]);
    press(&mut app, "Esc");

    // A table that is not a dBase table is not written by these keys.
    let csv = dir.path().join("people.csv");
    std::fs::write(&csv, "name,town\nzoe,Torino\n").unwrap();
    app.view_path(&csv.display().to_string());
    for key in ["Ctrl+F4", "Ctrl+N", "Ctrl+F8", "Shift+Del"] {
        press(&mut app, key);
        assert!(matches!(app.mode, Mode::Viewer(_)), "{key}");
        assert_eq!(app.status, app.i18n.t("dbf-only"), "{key}");
    }
    assert_eq!(work(&mut app), 0);
    assert_eq!(std::fs::read(&csv).unwrap(), b"name,town\nzoe,Torino\n");
    assert_eq!(std::fs::read(&table).unwrap(), table_bytes());
}

#[test]
fn a_table_that_cannot_be_opened_for_writing_says_so() {
    let (_dir, table, mut app) = open_table();
    press(&mut app, "Ctrl+N");
    press(&mut app, "Enter");
    // The backup is made and the append is asked for; before a worker gets to it the table
    // goes away, which is as near as a test comes on every platform to another program
    // holding it.
    assert_eq!(work(&mut app), 1, "the backup");
    let requests = app.take_requests();
    std::fs::remove_file(&table).unwrap();
    assert_eq!(answer(&mut app, requests), 1, "the append");
    assert!(app.status_error, "{}", app.status);
    assert!(app.status.contains("CLIENTI.DBF"), "{}", app.status);
    assert!(app.status.contains("opened for writing"), "{}", app.status);
}

// ── what LookDBF taught: memos, sums, the marked rows, code pages, exports ─────────────────

/// A table from its parts, as the reader's own tests build one: `(name, type, width,
/// decimals)` for each field, and the records without their deletion flag.
fn table_of(language: u8, fields: &[(&str, u8, u8, u8)], records: &[&[u8]]) -> Vec<u8> {
    let header_len = 32 + fields.len() * 32 + 1;
    let record_len = 1 + fields.iter().map(|f| f.2 as usize).sum::<usize>();
    let mut out = vec![0x83, 99, 1, 15];
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    out.extend_from_slice(&(header_len as u16).to_le_bytes());
    out.extend_from_slice(&(record_len as u16).to_le_bytes());
    let mut rest = [0u8; 20];
    rest[29 - 12] = language;
    out.extend_from_slice(&rest);
    for (name, kind, len, decimals) in fields {
        let mut raw = [0u8; 32];
        raw[..name.len()].copy_from_slice(name.as_bytes());
        raw[11] = *kind;
        raw[16] = *len;
        raw[17] = *decimals;
        out.extend_from_slice(&raw);
    }
    out.push(0x0d);
    for record in records {
        assert_eq!(record.len() + 1, record_len);
        out.push(b' ');
        out.extend_from_slice(record);
    }
    out.push(0x1a);
    out
}

/// A dBase III memo file with one memo in its first block.
fn memo_of(text: &str) -> Vec<u8> {
    let mut out = vec![0u8; 512];
    let mut block = text.as_bytes().to_vec();
    block.push(0x1a);
    block.resize(512, 0);
    out.extend_from_slice(&block);
    out
}

/// The viewer on `bytes` written as `name` in a folder of its own, with `memo` beside it
/// under `memo_name` when there is one.
fn open_bytes(
    name: &str,
    bytes: &[u8],
    memo: Option<(&str, &[u8])>,
) -> (tempfile::TempDir, PathBuf, App) {
    let dir = tempfile::tempdir().unwrap();
    let table = dir.path().join(name);
    std::fs::write(&table, bytes).unwrap();
    if let Some((memo_name, memo)) = memo {
        std::fs::write(dir.path().join(memo_name), memo).unwrap();
    }
    let mut app = app();
    app.dbf.clock = Some(THEN);
    app.view_path(&table.display().to_string());
    let mode = app.live_viewer_mut().map(|v| v.mode);
    assert_eq!(mode, Some(fcmd_view::Mode::Table), "{name} opens as a table");
    (dir, table, app)
}

/// `NAME C(10)`, `QTY N(7,2)`, `NOTE M(10)`: four records, the first with a memo.
fn stock() -> Vec<u8> {
    let record = |name: &str, qty: &str, note: &str| format!("{name:<10}{qty:>7}{note:>10}");
    let records = [
        record("bolts", "0.10", "1"),
        record("nuts", "0.20", ""),
        record("washers", "10.00", ""),
        record("screws", "-1.05", ""),
    ];
    let records: Vec<&[u8]> = records.iter().map(|r| r.as_bytes()).collect();
    table_of(
        0x03,
        &[
            ("NAME", b'C', 10, 0),
            ("QTY", b'N', 7, 2),
            ("NOTE", b'M', 10, 0),
        ],
        &records,
    )
}

/// What the viewer has on screen, as text.
fn screen(app: &mut App) -> String {
    let viewer = app.live_viewer_mut().expect("a viewer");
    viewer
        .rows(80, 20)
        .iter()
        .map(|row| row.text.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_memo_cell_is_read_whole_and_the_table_is_one_key_back() {
    let memo = memo_of("first line\r\nsecond line\r\n\r\nand a third, after a gap");
    let (_dir, _table, mut app) = open_bytes("STOCK.DBF", &stock(), Some(("STOCK.DBT", &memo)));
    // The cell has one line of it.
    assert_eq!(
        cells(&mut app)[2],
        "first line second line and a third, after a gap"
    );
    // On a cell that is no memo there is no memo to read.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Enter");
    assert!(app.status_error);
    assert_eq!(app.status, app.i18n.t("dbf-not-memo"));
    // On the memo's own cell, `Enter` opens it: the text as it was written, in a view of
    // its own that says where it came from.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Enter");
    let viewer = app.live_viewer_mut().unwrap();
    assert!(viewer.in_memo());
    assert_eq!(viewer.mode, fcmd_view::Mode::Text);
    assert_eq!(viewer.derived.as_deref(), Some("memo NOTE, record 1"));
    assert_eq!(
        screen(&mut app),
        "first line\nsecond line\n\nand a third, after a gap"
    );
    // The keys that write a table have no table in front of them here.
    press(&mut app, "Ctrl+F8");
    assert!(app.status_error);
    assert_eq!(work(&mut app), 0);

    // `Esc` is the way out of the memo, and it leads to the table — on the row it was.
    press(&mut app, "Esc");
    let viewer = app.live_viewer_mut().expect("still in the viewer");
    assert!(!viewer.in_memo());
    assert_eq!(viewer.mode, fcmd_view::Mode::Table);
    assert_eq!(viewer.derived, None);
    assert_eq!(cells(&mut app)[0], "bolts");
    // `m` is the command's own key, and `Backspace` goes back as well.
    press(&mut app, "m");
    assert!(app.live_viewer_mut().unwrap().in_memo());
    press(&mut app, "Backspace");
    assert!(!app.live_viewer_mut().unwrap().in_memo());
    assert!(screen(&mut app).contains("first"));
    // And from the table `Esc` leaves the viewer, as it always did.
    press(&mut app, "Down");
    press(&mut app, "m");
    assert_eq!(app.status, app.i18n.t("dbf-memo-none"), "an empty memo");
    press(&mut app, "Esc");
    assert!(!matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
}

#[test]
fn a_memo_whose_file_is_not_there_says_so() {
    let (_dir, _table, mut app) = open_bytes("STOCK.DBF", &stock(), None);
    for _ in 0..3 {
        press(&mut app, "Ctrl+Right");
    }
    press(&mut app, "Enter");
    assert!(app.status_error);
    assert_eq!(app.status, app.i18n.t("dbf-memo-none"));
    assert!(!app.live_viewer_mut().unwrap().in_memo());
}

#[test]
fn the_numeric_columns_are_summed_over_the_marked_rows_or_all_of_them() {
    let (_dir, _table, mut app) = open_bytes("STOCK.DBF", &stock(), None);
    press(&mut app, "Ctrl+F6");
    let Mode::Message { title, lines } = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(
        *title,
        app.i18n.t_args("dbf-sums-title", &[("rows", 4.into())])
    );
    // To the cent, and how many rows had a number.
    assert_eq!(lines, &["QTY  9.25  (4)"]);
    press(&mut app, "Esc");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);

    // With rows marked, over those alone.
    press(&mut app, "Ins");
    press(&mut app, "Ins");
    press(&mut app, "Ctrl+F6");
    let Mode::Message { title, lines } = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(
        *title,
        app.i18n.t_args("dbf-sums-title", &[("rows", 2.into())])
    );
    assert_eq!(lines, &["QTY  0.30  (2)"]);
    press(&mut app, "Esc");

    // A table with no numbers says so, and a table that is no dBase table says that.
    let (dir, _table, mut app) = open_table();
    press(&mut app, "Ctrl+F6");
    assert!(matches!(app.mode, Mode::Viewer(_)));
    assert_eq!(app.status, app.i18n.t("dbf-sums-none"));
    let csv = dir.path().join("n.csv");
    std::fs::write(&csv, "a,b\n1,2\n").unwrap();
    app.view_path(&csv.display().to_string());
    press(&mut app, "Ctrl+F6");
    assert_eq!(app.status, app.i18n.t("dbf-not-dbase"));
}

#[test]
fn the_marked_rows_alone_are_shown_and_every_row_again() {
    let (_dir, _table, mut app) = open_bytes("STOCK.DBF", &stock(), None);
    let rows = |app: &mut App| app.live_viewer_mut().map(|v| v.table_rows());
    // With nothing marked there is nothing to show alone.
    press(&mut app, "Ctrl+F5");
    assert!(app.status_error);
    assert_eq!(app.status, app.i18n.t("table-none-marked"));
    assert_eq!(rows(&mut app), Some(4));

    // The first and the third.
    press(&mut app, "Ins");
    press(&mut app, "Down");
    press(&mut app, "Ins");
    press(&mut app, "Ctrl+F5");
    assert_eq!(
        app.status,
        app.i18n.t_args("table-marked-only", &[("rows", 2.into())])
    );
    assert_eq!(rows(&mut app), Some(2));
    let shown = screen(&mut app);
    assert!(shown.contains("bolts") && shown.contains("washers"), "{shown}");
    assert!(!shown.contains("nuts") && !shown.contains("screws"), "{shown}");
    // The status line says what the view is, as it does for a filter.
    let note = app.live_viewer_mut().unwrap().status().table.unwrap();
    assert!(note.contains("marked only"), "{note}");
    // The record under the cursor is the record, not the row's number.
    press(&mut app, "Down");
    let here = app.live_viewer_mut().and_then(|v| v.dbf_here()).unwrap();
    assert_eq!((here.record, here.cells[0].as_str()), (2, "washers"));

    // The same key shows every row again.
    press(&mut app, "Ctrl+F5");
    assert_eq!(app.status, app.i18n.t("table-unfiltered"));
    assert_eq!(rows(&mut app), Some(4));
    let note = app.live_viewer_mut().unwrap().status().table.unwrap();
    assert!(!note.contains("marked only"), "{note}");
}

#[test]
fn a_table_that_names_no_code_page_is_told_one_and_written_in_it() {
    // What Clipper writes: a language byte of zero. `8A` is `è` in 437 and 850.
    let bytes = table_of(0, &[("NAME", b'C', 10, 0)], &[b"caff\x8a     "]);
    let (dir, table, mut app) = open_bytes("CLIENTI.DBF", &bytes, None);
    assert_eq!(cells(&mut app)[0], "caffŠ", "as Windows-1252 has it");

    // `F8` in a dBase table is the table's page: 437 first, then 850.
    press(&mut app, "F8");
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-codepage", &[("page", "CP437".into())])
    );
    assert_eq!(cells(&mut app)[0], "caffè");
    press(&mut app, "Ctrl+T");
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-codepage", &[("page", "CP850".into())])
    );
    assert_eq!(cells(&mut app)[0], "caffè");
    let note = app.live_viewer_mut().unwrap().status().table.unwrap();
    assert!(note.contains("CP850"), "{note}");

    // Written in that page, exactly: `ò` is `95` there. Until a page was chosen an
    // accented letter was refused.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+F4");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the backup");
    for _ in 0..5 {
        press(&mut app, "Backspace");
    }
    type_text(&mut app, "però");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the cell");
    assert!(!app.status_error, "{}", app.status);
    let after = std::fs::read(&table).unwrap();
    let first = u16::from_le_bytes([after[8], after[9]]) as usize;
    assert_eq!(&after[first..first + 11], b" per\x95      ");
    assert_eq!(after[29], 0, "the header still names none");
    assert_eq!(cells(&mut app)[0], "però");

    // The choice is the session's for this table: opened again, it is still 850.
    press(&mut app, "Esc");
    app.view_path(&table.display().to_string());
    assert_eq!(cells(&mut app)[0], "però");
    // The wheel comes round to the header's own silence, and ASCII alone again.
    for _ in 0..7 {
        press(&mut app, "Shift+F8");
    }
    assert_eq!(app.status, app.i18n.t("dbf-codepage-header"));
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+F4");
    for _ in 0..4 {
        press(&mut app, "Backspace");
    }
    type_text(&mut app, "è");
    press(&mut app, "Enter");
    assert!(app.status_error);
    assert_eq!(
        app.status,
        app.i18n
            .t_args("dbf-no-code-page", &[("char", "è".into())])
    );
    assert_eq!(work(&mut app), 0);
    assert_eq!(std::fs::read(&table).unwrap(), after);

    // A header that names its page is not argued with.
    let named = dir.path().join("win.dbf");
    std::fs::write(&named, table_bytes()).unwrap();
    app.view_path(&named.display().to_string());
    press(&mut app, "F8");
    assert!(app.status_error);
    assert_eq!(
        app.status,
        app.i18n
            .t_args("dbf-codepage-named", &[("page", "CP1252".into())])
    );
    // And in a file that is no dBase table `F8` is the viewer's own key still.
    let text = dir.path().join("notes.txt");
    std::fs::write(&text, "plain").unwrap();
    app.view_path(&text.display().to_string());
    let was = app.live_viewer_mut().unwrap().codepage;
    press(&mut app, "F8");
    assert_ne!(app.live_viewer_mut().unwrap().codepage, was);
}

#[test]
fn the_marked_records_are_deleted_or_recalled_behind_one_question_that_counts_them() {
    let (_dir, table, mut app) = open_bytes("STOCK.DBF", &stock(), None);
    let before = std::fs::read(&table).unwrap();
    let first = u16::from_le_bytes([before[8], before[9]]) as usize;
    let flags = |table: &Path| -> Vec<u8> {
        let bytes = std::fs::read(table).unwrap();
        (0..4).map(|n| bytes[first + n * 28]).collect()
    };
    // Nothing marked, nothing asked.
    for key in ["Ctrl+D", "Ctrl+R"] {
        press(&mut app, key);
        assert!(matches!(app.mode, Mode::Viewer(_)), "{key}");
        assert_eq!(app.status, app.i18n.t("table-none-marked"), "{key}");
    }
    // The first, the second and the fourth.
    press(&mut app, "Ins");
    press(&mut app, "Ins");
    press(&mut app, "Down");
    press(&mut app, "Ins");

    press(&mut app, "Ctrl+D");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "the backup");
    // One question, which says how many.
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(matches!(
        &confirm.action,
        ConfirmAction::Dbf(DbfStep::Marked { records, deleted: true, .. }) if records == &[0, 1, 3]
    ));
    let said = confirm.lines.join(" ");
    assert!(said.contains("STOCK.DBF") && said.contains(": 3."), "{said}");
    assert_eq!(
        confirm.buttons(&app.i18n),
        [app.i18n.t("btn-delete"), app.i18n.t("btn-cancel")]
    );
    assert_eq!(work(&mut app), 0);
    assert_eq!(flags(&table), b"    ", "asked, not done");
    // No is no.
    press(&mut app, "Esc");
    assert_eq!(work(&mut app), 0);
    assert_eq!(flags(&table), b"    ");

    press(&mut app, "Ctrl+D");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1, "all three in one write");
    assert_eq!(flags(&table), b"** *");
    assert_eq!(
        app.status,
        app.i18n.t_args("dbf-marked-deleted", &[("count", 3.into())])
    );
    // On screen at once, and the marks are still there for the next thing.
    assert!(app
        .live_viewer_mut()
        .and_then(|v| v.dbf_here())
        .is_some_and(|here| here.deleted));

    // `Ctrl+R` recalls them, behind its own question.
    press(&mut app, "Ctrl+R");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(
        confirm.buttons(&app.i18n),
        [app.i18n.t("btn-dbf-recall"), app.i18n.t("btn-cancel")]
    );
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1);
    assert_eq!(flags(&table), b"    ");
    assert_eq!(
        app.status,
        app.i18n
            .t_args("dbf-marked-recalled", &[("count", 3.into())])
    );
    // Nothing but the flags and the header's date moved.
    assert_eq!(std::fs::read(&table).unwrap()[8..], before[8..]);
}

#[test]
fn the_rows_are_exported_to_a_new_csv_or_table_and_never_over_a_file() {
    let memo = memo_of("a note");
    let (dir, table, mut app) = open_bytes("STOCK.DBF", &stock(), Some(("STOCK.DBT", &memo)));
    let before = std::fs::read(&table).unwrap();
    let at = |name: &str| dir.path().join(name);

    // The whole view, in the order of the view: sorted by quantity here.
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+Right");
    press(&mut app, "Ctrl+F3");
    press(&mut app, "Ctrl+F9");
    let Mode::Prompt(prompt) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(prompt.kind, PromptKind::DbfExport);
    assert_eq!(
        prompt.title,
        app.i18n.t_args("dbf-prompt-export", &[("rows", 4.into())])
    );
    // A name that is not the table's own is offered.
    assert_eq!(prompt.text, "STOCK-export.csv");
    press(&mut app, "Enter");
    assert!(matches!(app.mode, Mode::Viewer(_)), "{:?}", app.mode);
    assert_eq!(work(&mut app), 1);
    assert!(!app.status_error, "{}", app.status);
    assert_eq!(
        app.status,
        app.i18n.t_args(
            "dbf-exported",
            &[("rows", 4.into()), ("name", "STOCK-export.csv".into())]
        )
    );
    assert_eq!(
        std::fs::read_to_string(at("STOCK-export.csv")).unwrap(),
        "*,NAME,QTY,NOTE\r\n,screws,-1.05,\r\n,bolts,0.10,a note\r\n,nuts,0.20,\r\n,washers,10.00,\r\n"
    );

    // The same name again: said at once, and the file is as it was.
    press(&mut app, "Ctrl+F9");
    press(&mut app, "Enter");
    assert!(app.status_error);
    assert_eq!(
        app.status,
        app.i18n
            .t_args("dbf-export-exists", &[("name", "STOCK-export.csv".into())])
    );
    assert_eq!(work(&mut app), 0);

    // The marked rows alone, to a table: the same structure, the records as they are, and
    // the memo file beside it under the new name.
    press(&mut app, "Ins");
    press(&mut app, "Ins");
    press(&mut app, "Ctrl+F9");
    let Mode::Prompt(prompt) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert_eq!(
        prompt.title,
        app.i18n.t_args("dbf-prompt-export", &[("rows", 2.into())])
    );
    for _ in 0.."STOCK-export.csv".len() {
        press(&mut app, "Backspace");
    }
    // Typed in small letters, and the memo file is named in small letters with it.
    type_text(&mut app, "part.dbf");
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1);
    assert_eq!(
        app.status,
        app.i18n.t_args(
            "dbf-exported-memo",
            &[
                ("rows", 2.into()),
                ("name", "part.dbf".into()),
                ("memo", "part.dbt".into())
            ]
        )
    );
    let part = std::fs::read(at("part.dbf")).unwrap();
    let first = u16::from_le_bytes([before[8], before[9]]) as usize;
    assert_eq!(part[4], 2);
    assert_eq!(part[8..first], before[8..first]);
    // `screws` and `bolts`, which the sort put first: records 3 and 0 of the table.
    assert_eq!(part[first..first + 28], before[first + 3 * 28..first + 4 * 28]);
    assert_eq!(part[first + 28..first + 56], before[first..first + 28]);
    assert_eq!(std::fs::read(at("part.dbt")).unwrap(), memo);
    // The new table opens, and its memo with it.
    app.view_path(&at("part.dbf").display().to_string());
    press(&mut app, "Down");
    assert_eq!(cells(&mut app), ["bolts", "0.10", "a note"]);

    // The table that was exported from was only read, and nothing asked for a backup.
    assert_eq!(std::fs::read(&table).unwrap(), before);
    assert_eq!(
        folder(&dir),
        ["STOCK-export.csv", "STOCK.DBF", "STOCK.DBT", "part.dbf", "part.dbt"]
    );
}

#[test]
fn a_container_of_visual_foxpro_opens_as_the_table_it_is_with_its_memo_file() {
    // A database container is a table called `.dbc`, and its memo file is a `.dct`.
    let memo = memo_of("the properties of the table");
    let (_dir, _table, mut app) = open_bytes("shop.dbc", &stock(), Some(("shop.DCT", &memo)));
    assert_eq!(cells(&mut app)[2], "the properties of the table");
    // The structure pane says what the header does: every field, its type and its place.
    press(&mut app, "Shift+F11");
    let viewer = app.live_viewer_mut().unwrap();
    assert!(viewer.show_structure);
    let rows = &viewer.structure().expect("a table has a structure").rows;
    let qty = rows.iter().find(|r| r.label == "QTY").expect("the field");
    assert_eq!((qty.raw.as_str(), qty.value.as_str()), ("N 7.2", "offset 11"));
    // The other names Visual FoxPro gives its tables, each with its memo file's.
    for (name, memo_name) in [
        ("form.scx", "form.sct"),
        ("REPORT.FRX", "REPORT.FRT"),
        ("lib.vcx", "lib.VCT"),
        ("menu.mnx", "menu.mnt"),
        ("app.pjx", "app.pjt"),
        ("label.lbx", "label.lbt"),
    ] {
        let (_dir, _table, mut app) = open_bytes(name, &stock(), Some((memo_name, &memo)));
        assert_eq!(cells(&mut app)[2], "the properties of the table", "{name}");
    }
}
