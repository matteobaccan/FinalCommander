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

#[test]
fn a_table_with_an_index_beside_it_is_refused_and_says_why() {
    let (dir, table, mut app) = open_table();
    let before = std::fs::read(&table).unwrap();
    std::fs::write(dir.path().join("CLIENTI.CDX"), b"the index").unwrap();
    for key in ["Ctrl+F4", "Ctrl+N", "Ctrl+F8", "Shift+Del"] {
        press(&mut app, key);
        let Mode::Message { lines, .. } = &app.mode else {
            panic!("{key}: {:?}", app.mode)
        };
        let said = lines.join(" ");
        assert!(said.contains("CLIENTI.CDX"), "{said}");
        assert!(said.contains("Clipper"), "the reason is given: {said}");
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Viewer(_)), "{key}: {:?}", app.mode);
        assert_eq!(work(&mut app), 0);
    }
    // No backup either: a copy of a table nobody is going to change is clutter.
    assert_eq!(folder(&dir), ["CLIENTI.CDX", "CLIENTI.DBF", "CLIENTI.DBT"]);
    assert_eq!(std::fs::read(&table).unwrap(), before);

    // An index that arrives between the question and the answer is seen by the worker,
    // which looks again before it copies or writes anything.
    std::fs::remove_file(dir.path().join("CLIENTI.CDX")).unwrap();
    press(&mut app, "Ctrl+F8");
    assert!(matches!(app.mode, Mode::Confirm(_)));
    std::fs::write(dir.path().join("NOMI.NTX"), b"the index").unwrap();
    press(&mut app, "Enter");
    assert_eq!(work(&mut app), 1);
    assert!(matches!(app.mode, Mode::Message { .. }), "{:?}", app.mode);
    assert_eq!(folder(&dir), ["CLIENTI.DBF", "CLIENTI.DBT", "NOMI.NTX"]);
    assert_eq!(std::fs::read(&table).unwrap(), before);
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
