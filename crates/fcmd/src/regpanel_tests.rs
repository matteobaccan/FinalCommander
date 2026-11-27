// The registry in a panel (SPEC 06 §17), against a registry kept in memory: what the panel
// lists, the dialog each type gets, and — the part that matters — that the backup is on
// disk before anything changes and that nothing changes when it cannot be written.
//
// Included into the application's own test module (see the end of `app.rs`), which is where
// `app_on_disk`, `press` and `typed` live.

use super::*;
use fcmd_platform::regfile::{self, KeyBlock, Line, Value};
use fcmd_platform::registry::{self, MemoryRegistry, Registry};
use regpanel::{RegFailed, RegOk, RegWork};

const KEY: &str = r"HKEY_CURRENT_USER\Software\Example";
/// The moment every backup of these tests is named after.
const THEN: &str = "20261127-101500";

/// A folder for the settings, a registry with one application's keys in it, and the program
/// looking at that registry with the clock stopped. The backups go where they go for real:
/// `registry-backups/` beside the settings, which is the folder.
fn with_registry() -> (tempfile::TempDir, MemoryRegistry, App) {
    let dir = tempfile::tempdir().unwrap();
    let registry = MemoryRegistry::new()
        .with(
            KEY,
            vec![
                Value::sz("", "hello"),
                Value::dword("Count", 3),
                Value::expand_sz("Path", r"%TEMP%\x"),
                Value::multi_sz("List", &["one", "two"]),
                Value::qword("Big", 5),
                Value::binary("Blob", &[1, 0xab, 0xff]),
            ],
        )
        .with(&registry::join(KEY, "Deep"), vec![Value::sz("a", "b")])
        .with(&registry::join(KEY, r"Deep\Deeper"), Vec::new())
        .with(&registry::join(KEY, "Other"), Vec::new());
    let mut app = app_on_disk(dir.path());
    app.reg.registry = Box::new(registry.clone());
    app.reg.available = true;
    app.reg.clock = Some(THEN.into());
    (dir, registry, app)
}

/// The left panel on this key, as `Alt+F1` → Registry and a few `Enter`s would leave it.
fn show(app: &mut App, key: &str) {
    app.open_registry_panel(0);
    assert!(app.reg_show(0, key, None), "{}", app.status);
    app.status.clear();
    app.status_error = false;
}

fn rows(app: &App) -> Vec<String> {
    let panel = &app.panels[0];
    (0..panel.listing.len())
        .filter_map(|row| panel.listing.row(row))
        .map(|e| e.name_str().into_owned())
        .collect()
}

/// The cursor on the row that starts with this.
fn cursor_on(app: &mut App, start: &str) {
    let at = rows(app)
        .iter()
        .position(|name| name.starts_with(start))
        .unwrap_or_else(|| panic!("no row {start}: {:?}", rows(app)));
    app.panels[0].cursor_to(at);
}

/// What the worker would do with what was asked, done here on the same registry, and the
/// answers handed back — until nothing more is asked.
fn settle(app: &mut App, registry: &MemoryRegistry) -> Vec<Result<RegOk, RegFailed>> {
    let mut results = Vec::new();
    loop {
        let asked: Vec<RegAsk> = app
            .take_requests()
            .into_iter()
            .filter_map(|r| match r {
                Request::Registry(ask) => Some(ask),
                _ => None,
            })
            .collect();
        if asked.is_empty() {
            return results;
        }
        for ask in asked {
            let result = regpanel::perform(registry, &ask);
            results.push(result.clone());
            app.handle_event(Event::Registry(RegDone { ask, result }));
        }
    }
}

fn form(app: &App) -> &Form {
    match &app.mode {
        Mode::Form(form) => form,
        other => panic!("no dialog: {other:?}"),
    }
}

/// What the dialog's text rows of this key hold.
fn texts(app: &App, want: &str) -> Vec<String> {
    form(app)
        .fields
        .iter()
        .filter_map(|f| match f {
            crate::form::Field::Text { key, value, .. } if *key == want => Some(value.clone()),
            _ => None,
        })
        .collect()
}

/// Write into the `nth` text row of this key, as typing over it would.
fn set_text(app: &mut App, want: &str, nth: usize, text: &str) {
    let Mode::Form(form) = &mut app.mode else {
        panic!("no dialog")
    };
    let field = form
        .fields
        .iter_mut()
        .filter(|f| matches!(f, crate::form::Field::Text { key, .. } if *key == want))
        .nth(nth)
        .expect("the row");
    if let crate::form::Field::Text { value, caret, .. } = field {
        *value = text.to_string();
        *caret = text.chars().count();
    }
}

/// Answer the question in front with its first button, the one that does it.
fn agree(app: &mut App) {
    match &mut app.mode {
        Mode::Confirm(confirm) => confirm.button = 0,
        other => panic!("no question: {other:?}"),
    }
    press(app, "Enter");
}

fn backup_file(app: &App, stem: &str) -> PathBuf {
    app.reg_backup_dir().join(format!("{stem}-{THEN}.reg"))
}

fn read_reg(file: &std::path::Path) -> Vec<KeyBlock> {
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    assert_eq!(&bytes[..2], [0xff, 0xfe], "UTF-16 with its mark");
    regfile::parse(&regfile::decode(&bytes)).unwrap()
}

const KEY_STEM: &str = "HKEY_CURRENT_USER_Software_Example";

#[test]
fn the_drive_menu_offers_the_registry_only_where_there_is_one() {
    let (_dir, _registry, mut app) = with_registry();
    let offered = |app: &App| {
        app.drive_pick(0, Vec::new())
            .rows
            .iter()
            .any(|r| r.value == ":registry")
    };
    assert!(offered(&app));
    app.reg.available = false;
    assert!(!offered(&app), "no row where there is no registry");
    // And asked for all the same, it says why not rather than opening an empty panel.
    app.open_registry_panel(0);
    assert!(app.status_error && app.panels[0].temp_origin().is_none());
}

#[test]
fn the_roots_are_folders_enter_goes_into_a_key_and_dotdot_comes_back() {
    let (_dir, _registry, mut app) = with_registry();
    app.open_registry_panel(0);
    assert_eq!(app.panels[0].temp_origin(), Some(":registry"));
    assert_eq!(app.panels[0].temp.as_ref().unwrap().title, "Registry");
    let mut roots = rows(&app);
    roots.sort();
    let mut want: Vec<String> = registry::ROOTS.iter().map(|r| r.to_string()).collect();
    want.sort();
    assert_eq!(roots, want);

    cursor_on(&mut app, "HKEY_CURRENT_USER");
    press(&mut app, "Enter");
    assert_eq!(app.reg_key(0).as_deref(), Some("HKEY_CURRENT_USER"));
    assert_eq!(rows(&app), ["..", "Software"]);
    assert_eq!(app.panels[0].cursor, 0, "on the way back out");
    cursor_on(&mut app, "Software");
    press(&mut app, "Enter");
    cursor_on(&mut app, "Example");
    press(&mut app, "Enter");
    // The title is the key, and the rows are its keys and then its values, each with its
    // name, its type and its data in columns.
    assert_eq!(app.panels[0].temp.as_ref().unwrap().title, KEY);
    let listed = rows(&app);
    assert_eq!(&listed[..3], ["..", "Deep", "Other"]);
    assert!(listed.contains(&"(Default)  REG_SZ         hello".to_string()), "{listed:?}");
    assert!(listed.contains(&"Count      REG_DWORD      0x00000003 (3)".to_string()));
    assert!(listed.contains(&r"Path       REG_EXPAND_SZ  %TEMP%\x".to_string()));
    assert!(listed.contains(&"List       REG_MULTI_SZ   one | two".to_string()));
    assert!(listed.contains(&"Blob       REG_BINARY     01 ab ff".to_string()));
    // The size column is the length of the data.
    cursor_on(&mut app, "Blob");
    assert_eq!(app.panel().current().unwrap().size, 3);

    // `..` and the key that goes up both land on the key come out of.
    cursor_on(&mut app, "..");
    press(&mut app, "Enter");
    assert_eq!(app.reg_key(0).as_deref(), Some(r"HKEY_CURRENT_USER\Software"));
    assert_eq!(app.panel().current().unwrap().name_str(), "Example");
    app.run(Command::NavParent);
    assert_eq!(app.reg_key(0).as_deref(), Some("HKEY_CURRENT_USER"));
    assert_eq!(app.panel().current().unwrap().name_str(), "Software");
    app.run(Command::NavRoot);
    assert_eq!(app.reg_key(0).as_deref(), Some(""));
    assert_eq!(app.panel().current().unwrap().name_str(), "HKEY_CURRENT_USER");
    // Left and come back to, the panel is on the key it was last on.
    show(&mut app, KEY);
    app.run(Command::NavRoot);
    show(&mut app, KEY);
    app.open_registry_panel(1);
    assert_eq!(app.reg_key(1).as_deref(), Some(KEY));
}

#[test]
fn a_key_that_cannot_be_listed_says_so_and_the_panel_stays() {
    let (_dir, _registry, mut app) = with_registry();
    show(&mut app, KEY);
    assert!(!app.reg_show(0, r"HKEY_CURRENT_USER\Nowhere", None));
    assert!(app.status_error && app.status.contains("Nowhere"), "{}", app.status);
    assert_eq!(app.reg_key(0).as_deref(), Some(KEY));
}

#[test]
fn ctrl_r_reads_the_key_again() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    registry.set_value(KEY, &Value::sz("Late", "x")).unwrap();
    assert!(!rows(&app).iter().any(|r| r.starts_with("Late")));
    cursor_on(&mut app, "Count");
    app.run(Command::PanelReread);
    assert!(rows(&app).iter().any(|r| r.starts_with("Late")));
    assert!(app.panel().current().unwrap().name_str().starts_with("Count"));
    // And a key that went away behind the panel's back leaves it on the one above.
    registry.delete_tree(KEY).unwrap();
    app.run(Command::PanelReread);
    assert_eq!(app.reg_key(0).as_deref(), Some(r"HKEY_CURRENT_USER\Software"));
}

#[test]
fn f3_shows_a_value_whole() {
    let (_dir, _registry, mut app) = with_registry();
    show(&mut app, KEY);
    let shown = |app: &mut App, row: &str| {
        cursor_on(app, row);
        press(app, "F3");
        let Mode::Message { title, lines } = std::mem::replace(&mut app.mode, Mode::Normal)
        else {
            panic!("no card for {row}")
        };
        (title, lines)
    };
    let (title, lines) = shown(&mut app, "List");
    assert_eq!(title, "List");
    assert_eq!(lines, [KEY, "REG_MULTI_SZ  18 B", "", "one", "two"]);
    let (_, lines) = shown(&mut app, "Count");
    assert_eq!(lines[3], "3  (0x00000003)");
    let (_, lines) = shown(&mut app, "Blob");
    assert!(lines[3].starts_with("00000000  01 ab ff"), "{lines:?}");
    let (title, lines) = shown(&mut app, "(Default)");
    assert_eq!((title.as_str(), lines[3].as_str()), ("(Default)", "hello"));
    // A key says how much it holds.
    let (title, lines) = shown(&mut app, "Deep");
    assert_eq!(title, "Deep");
    assert_eq!(lines, [registry::join(KEY, "Deep"), "Keys  1".into(), "Values  1".into()]);
}

#[test]
fn f4_opens_the_dialog_the_type_wants() {
    let (_dir, _registry, mut app) = with_registry();
    show(&mut app, KEY);
    let opened = |app: &mut App, row: &str| {
        cursor_on(app, row);
        press(app, "F4");
        assert_eq!(form(app).kind, FormKind::Registry);
        let rows = texts(app, "reg.data");
        press(app, "Esc");
        rows
    };
    assert_eq!(opened(&mut app, "(Default)"), ["hello"]);
    assert_eq!(opened(&mut app, "Path"), [r"%TEMP%\x"]);
    assert_eq!(opened(&mut app, "Count"), ["3"]);
    assert_eq!(opened(&mut app, "Big"), ["5"]);
    assert_eq!(opened(&mut app, "Blob"), ["01 ab ff"]);
    // A list is a row a string, and empty rows to add to.
    assert_eq!(opened(&mut app, "List"), ["one", "two", "", "", "", ""]);
    // The dialog names the value and its type above the rows.
    cursor_on(&mut app, "Count");
    press(&mut app, "F4");
    let notes: Vec<&str> = form(&app)
        .fields
        .iter()
        .filter_map(|f| match f {
            crate::form::Field::Note(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(notes, [format!("{KEY}\\Count").as_str(), "REG_DWORD"]);
    // `Enter` on a value is the same dialog; on a key there is nothing to edit.
    press(&mut app, "Esc");
    press(&mut app, "Enter");
    assert_eq!(texts(&app, "reg.data"), ["3"]);
    press(&mut app, "Esc");
    cursor_on(&mut app, "Deep");
    press(&mut app, "F4");
    assert!(app.mode.is_normal() && app.status_error);
}

#[test]
fn an_edit_is_written_after_the_key_was_backed_up_as_it_was() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    cursor_on(&mut app, "Count");
    press(&mut app, "F4");
    set_text(&mut app, "reg.data", 0, "");
    typed(&mut app, "0x10");
    press(&mut app, "Enter");
    assert!(registry.log().is_empty(), "nothing is written on the UI thread");
    let results = settle(&mut app, &registry);
    let backup = backup_file(&app, KEY_STEM);
    assert_eq!(results, [Ok(RegOk::Changed { backup: backup.clone() })]);
    assert_eq!(registry.value(KEY, "Count"), Some(Value::dword("Count", 16)));
    assert_eq!(registry.log(), [format!("set {KEY}\\Count")]);
    // The backup is the key as it was before: its own values, and the old number.
    let kept = read_reg(&backup);
    assert_eq!(kept.len(), 1, "the key alone, not what is under it");
    assert_eq!(kept[0].path, KEY);
    assert!(kept[0].lines.contains(&Line::Set(Value::dword("Count", 3))));
    assert_eq!(kept[0].lines.len(), 6);
    // The status line says where it went, and the row shows the new value under the cursor.
    assert!(!app.status_error, "{}", app.status);
    assert!(app.status.contains(&backup.display().to_string()), "{}", app.status);
    assert!(app.status.contains("Count"));
    assert_eq!(
        app.panel().current().unwrap().name_str(),
        "Count      REG_DWORD      0x00000010 (16)"
    );

    // A second change in the same second is a second backup, not the first overwritten.
    press(&mut app, "F4");
    set_text(&mut app, "reg.data", 0, "17");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    let second = app.reg_backup_dir()
        .join(format!("{KEY_STEM}-{THEN}-2.reg"));
    assert!(read_reg(&second)[0].lines.contains(&Line::Set(Value::dword("Count", 16))));
    assert!(read_reg(&backup)[0].lines.contains(&Line::Set(Value::dword("Count", 3))));
}

#[test]
fn each_type_is_written_back_as_its_type() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    let edit = |app: &mut App, row: &str, rows: &[&str]| {
        cursor_on(app, row);
        press(app, "F4");
        for (i, text) in rows.iter().enumerate() {
            set_text(app, "reg.data", i, text);
        }
        press(app, "Enter");
        settle(app, &registry);
        assert!(!app.status_error, "{row}: {}", app.status);
    };
    // A string keeps the spaces it was typed with.
    edit(&mut app, "(Default)", &[" spaced "]);
    edit(&mut app, "Path", &[r"%APPDATA%\y"]);
    edit(&mut app, "Big", &["0xffffffffffffffff"]);
    edit(&mut app, "Blob", &["de ad,BE ef"]);
    // A list: the second string emptied, one added in a spare row.
    edit(&mut app, "List", &["one", "", "three"]);
    assert_eq!(registry.value(KEY, ""), Some(Value::sz("", " spaced ")));
    assert_eq!(
        registry.value(KEY, "Path"),
        Some(Value::expand_sz("Path", r"%APPDATA%\y"))
    );
    assert_eq!(registry.value(KEY, "Big"), Some(Value::qword("Big", u64::MAX)));
    assert_eq!(
        registry.value(KEY, "Blob"),
        Some(Value::binary("Blob", &[0xde, 0xad, 0xbe, 0xef]))
    );
    assert_eq!(
        registry.value(KEY, "List"),
        Some(Value::multi_sz("List", &["one", "three"]))
    );
}

#[test]
fn what_does_not_fit_the_type_is_refused_and_the_dialog_stays() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    for (row, typed, says) in [
        ("Count", "4294967296", "REG_DWORD"),
        ("Count", "three", "0x"),
        ("Big", "18446744073709551616", "REG_QWORD"),
        ("Blob", "0g", "01 0a ff"),
    ] {
        cursor_on(&mut app, row);
        press(&mut app, "F4");
        set_text(&mut app, "reg.data", 0, typed);
        press(&mut app, "Enter");
        assert!(app.status_error && app.status.contains(says), "{row}: {}", app.status);
        assert_eq!(texts(&app, "reg.data"), [typed], "what was typed is still there");
        assert!(app.take_requests().is_empty());
        // Put right in the same dialog, it goes through.
        set_text(&mut app, "reg.data", 0, "01");
        press(&mut app, "Enter");
        assert_eq!(settle(&mut app, &registry).len(), 1);
    }
    assert_eq!(registry.value(KEY, "Count"), Some(Value::dword("Count", 1)));
}

#[test]
fn nothing_is_changed_when_the_backup_cannot_be_written() {
    let (_dir, registry, mut app) = with_registry();
    // A file where the backup folder would be: the folder cannot be made.
    std::fs::write(app.reg_backup_dir(), b"in the way").unwrap();
    show(&mut app, KEY);

    cursor_on(&mut app, "Count");
    press(&mut app, "F4");
    set_text(&mut app, "reg.data", 0, "9");
    press(&mut app, "Enter");
    let results = settle(&mut app, &registry);
    assert!(matches!(results[..], [Err(RegFailed::Backup(_))]), "{results:?}");
    assert!(app.status_error && app.status.contains("nothing was changed"), "{}", app.status);

    cursor_on(&mut app, "Count");
    press(&mut app, "F8");
    agree(&mut app);
    cursor_on(&mut app, "Deep");
    press(&mut app, "F8");
    agree(&mut app);
    cursor_on(&mut app, "Deep");
    press(&mut app, "F6");
    set_text(&mut app, "reg.name", 0, "Shallow");
    press(&mut app, "Enter");
    press(&mut app, "F7");
    set_text(&mut app, "reg.name", 0, "New");
    press(&mut app, "Enter");
    press(&mut app, "Shift+F4");
    set_text(&mut app, "reg.name", 0, "Fresh");
    press(&mut app, "Enter");
    let results = settle(&mut app, &registry);
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| matches!(r, Err(RegFailed::Backup(_)))), "{results:?}");

    // The registry is exactly as it was.
    assert!(registry.log().is_empty(), "{:?}", registry.log());
    assert_eq!(registry.value(KEY, "Count"), Some(Value::dword("Count", 3)));
    assert!(registry.has_key(&registry::join(KEY, r"Deep\Deeper")));
}

#[test]
fn a_key_this_user_may_not_write_says_access_denied() {
    let (_dir, registry, mut app) = with_registry();
    registry.deny(KEY);
    show(&mut app, KEY);
    cursor_on(&mut app, "Count");
    press(&mut app, "F4");
    set_text(&mut app, "reg.data", 0, "9");
    press(&mut app, "Enter");
    let results = settle(&mut app, &registry);
    assert_eq!(
        results,
        [Err(RegFailed::Registry {
            key: KEY.into(),
            error: registry::RegError::Denied
        })]
    );
    assert!(app.status_error, "never a silent failure");
    assert!(app.status.contains("Access denied") && app.status.contains(KEY), "{}", app.status);
    assert!(registry.log().is_empty());
    // The same for a key to make and one to delete.
    press(&mut app, "F7");
    set_text(&mut app, "reg.name", 0, "New");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert!(app.status_error && app.status.contains("Access denied"), "{}", app.status);
    cursor_on(&mut app, "Deep");
    press(&mut app, "F8");
    agree(&mut app);
    settle(&mut app, &registry);
    assert!(app.status_error && app.status.contains("Access denied"), "{}", app.status);
    assert!(registry.has_key(&registry::join(KEY, "Deep")));
}

#[test]
fn shift_f4_makes_a_value_of_the_type_chosen_and_opens_the_editor_on_it() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    press(&mut app, "Shift+F4");
    let offered: Vec<String> = form(&app)
        .fields
        .iter()
        .find_map(|f| match f {
            crate::form::Field::Choice { options, .. } => {
                Some(options.iter().map(|(_, label)| label.clone()).collect())
            }
            _ => None,
        })
        .expect("the types");
    assert_eq!(
        offered,
        ["REG_SZ", "REG_EXPAND_SZ", "REG_MULTI_SZ", "REG_DWORD", "REG_QWORD", "REG_BINARY"]
    );
    set_text(&mut app, "reg.name", 0, "Limit");
    if let Mode::Form(form) = &mut app.mode {
        for field in &mut form.fields {
            if let crate::form::Field::Choice { chosen, .. } = field {
                *chosen = 3;
            }
        }
    }
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert_eq!(registry.value(KEY, "Limit"), Some(Value::dword("Limit", 0)));
    assert!(backup_file(&app, KEY_STEM).exists());
    // The editor is open on the new value, which is under the cursor behind it.
    assert_eq!(texts(&app, "reg.data"), ["0"]);
    assert!(app.panel().current().unwrap().name_str().starts_with("Limit"));
    set_text(&mut app, "reg.data", 0, "250");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert_eq!(registry.value(KEY, "Limit"), Some(Value::dword("Limit", 250)));
    assert!(app.mode.is_normal());

    // A name that is taken — in any case — is refused before anything is asked for.
    press(&mut app, "Shift+F4");
    set_text(&mut app, "reg.name", 0, "count");
    press(&mut app, "Enter");
    assert!(app.status_error && app.status.contains("count"), "{}", app.status);
    assert!(app.take_requests().is_empty());
    // And among the roots there is nothing to make a value in.
    app.run(Command::NavRoot);
    press(&mut app, "Shift+F4");
    assert!(app.mode.is_normal() && app.status_error);
    press(&mut app, "F7");
    assert!(app.mode.is_normal() && app.status_error);
}

#[test]
fn f7_makes_a_key_and_puts_the_cursor_on_it() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    press(&mut app, "F7");
    set_text(&mut app, "reg.name", 0, "Settings");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert!(registry.has_key(&registry::join(KEY, "Settings")));
    assert_eq!(app.panel().current().unwrap().name_str(), "Settings");
    assert!(backup_file(&app, KEY_STEM).exists());
    assert_eq!(registry.log(), [format!("create {KEY}\\Settings")]);
    // One that is there already is said to be, and nothing is backed up for it.
    press(&mut app, "F7");
    set_text(&mut app, "reg.name", 0, "deep");
    press(&mut app, "Enter");
    let results = settle(&mut app, &registry);
    assert_eq!(results, [Err(RegFailed::Exists("deep".into()))]);
    assert!(app.status_error && app.status.contains("deep"));
    assert_eq!(std::fs::read_dir(app.reg_backup_dir()).unwrap().count(), 1);
}

#[test]
fn f8_names_the_key_says_how_many_it_holds_and_starts_on_cancel() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    cursor_on(&mut app, "Deep");
    press(&mut app, "F8");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("no question: {:?}", app.mode)
    };
    assert_eq!(confirm.button, 1, "Cancel is lit");
    assert_eq!(confirm.buttons(&app.i18n), ["Delete", "Cancel"]);
    let deep = registry::join(KEY, "Deep");
    assert!(confirm.lines[0].contains(&deep), "{:?}", confirm.lines);
    assert_eq!(confirm.lines[1], "Keys under it: 1");
    let folder = app.reg_backup_dir().display().to_string();
    assert!(confirm.lines[2].contains(&folder), "{:?}", confirm.lines);
    // `Enter` as the dialog opens is Cancel.
    press(&mut app, "Enter");
    assert!(app.take_requests().is_empty() && registry.has_key(&deep));

    press(&mut app, "F8");
    agree(&mut app);
    assert!(registry.has_key(&deep), "not before the worker has made the backup");
    settle(&mut app, &registry);
    assert!(!registry.has_key(&deep) && !registry.has_key(&registry::join(&deep, "Deeper")));
    assert_eq!(registry.log(), [format!("delete-tree {deep}")]);
    // The backup holds the whole subtree, and brings it back.
    let backup = backup_file(&app, &format!("{KEY_STEM}_Deep"));
    let kept = read_reg(&backup);
    let paths: Vec<&str> = kept.iter().map(|b| b.path.as_str()).collect();
    assert_eq!(paths, [deep.clone(), registry::join(&deep, "Deeper")]);
    assert!(app.status.contains(&backup.display().to_string()), "{}", app.status);
    assert!(!rows(&app).contains(&"Deep".to_string()));
    registry::apply(&registry, &kept).unwrap();
    assert_eq!(registry.value(&deep, "a"), Some(Value::sz("a", "b")));

    // A key with nothing under it is asked about without the count.
    app.run(Command::PanelReread);
    cursor_on(&mut app, "Other");
    press(&mut app, "F8");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("no question")
    };
    assert_eq!(confirm.lines.len(), 2, "{:?}", confirm.lines);
    press(&mut app, "Esc");
    // A root is nobody's to delete.
    app.run(Command::NavRoot);
    press(&mut app, "F8");
    assert!(app.mode.is_normal() && app.status_error);
}

#[test]
fn f8_on_a_value_names_it_and_its_key() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    cursor_on(&mut app, "(Default)");
    press(&mut app, "F8");
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("no question: {:?}", app.mode)
    };
    assert_eq!(confirm.button, 1);
    assert!(
        confirm.lines[0].contains("(Default)") && confirm.lines[0].contains(KEY),
        "{:?}",
        confirm.lines
    );
    agree(&mut app);
    settle(&mut app, &registry);
    assert_eq!(registry.value(KEY, ""), None);
    assert_eq!(registry.value(KEY, "Count"), Some(Value::dword("Count", 3)));
    let kept = read_reg(&backup_file(&app, KEY_STEM));
    assert!(kept[0].lines.contains(&Line::Set(Value::sz("", "hello"))));
}

#[test]
fn f6_renames_a_value_and_a_key_after_the_backup() {
    let (_dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    cursor_on(&mut app, "Count");
    press(&mut app, "F6");
    assert_eq!(texts(&app, "reg.name"), ["Count"]);
    set_text(&mut app, "reg.name", 0, "Total");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert_eq!(registry.value(KEY, "Total"), Some(Value::dword("Total", 3)));
    assert_eq!(registry.value(KEY, "Count"), None);
    assert!(app.panel().current().unwrap().name_str().starts_with("Total"));
    assert!(read_reg(&backup_file(&app, KEY_STEM))[0]
        .lines
        .contains(&Line::Set(Value::dword("Count", 3))));
    // Onto a name that is taken: refused, with nothing written.
    let before = registry.log().len();
    press(&mut app, "Shift+F6");
    set_text(&mut app, "reg.name", 0, "blob");
    press(&mut app, "Enter");
    assert_eq!(settle(&mut app, &registry), [Err(RegFailed::Exists("blob".into()))]);
    assert_eq!(registry.log().len(), before);

    // A key goes with everything under it, and the backup is the subtree under its old name.
    cursor_on(&mut app, "Deep");
    press(&mut app, "F6");
    set_text(&mut app, "reg.name", 0, "Shallow");
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    assert!(registry.has_key(&registry::join(KEY, r"Shallow\Deeper")));
    assert!(!registry.has_key(&registry::join(KEY, "Deep")));
    assert_eq!(app.panel().current().unwrap().name_str(), "Shallow");
    assert_eq!(read_reg(&backup_file(&app, &format!("{KEY_STEM}_Deep"))).len(), 2);
    press(&mut app, "F6");
    set_text(&mut app, "reg.name", 0, "other");
    press(&mut app, "Enter");
    assert_eq!(settle(&mut app, &registry), [Err(RegFailed::Exists("other".into()))]);
}

#[test]
fn f5_writes_keys_to_a_reg_file_that_reads_back_as_the_keys() {
    let (dir, registry, mut app) = with_registry();
    show(&mut app, KEY);
    cursor_on(&mut app, "Deep");
    press(&mut app, "F5");
    // The dialog proposes the other panel's folder and the key's name.
    let proposed = texts(&app, "reg.file");
    assert_eq!(proposed, [app.panels[1].cwd.join("Deep.reg").to_string()]);
    let out = dir.path().join("out");
    set_text(&mut app, "reg.file", 0, &out.display().to_string());
    press(&mut app, "Enter");
    assert_eq!(settle(&mut app, &registry), [Ok(RegOk::Exported { keys: 2 })]);
    // The extension is added, and the file is the subtree as regedit would write it.
    let file = dir.path().join("out.reg");
    let deep = registry::join(KEY, "Deep");
    assert_eq!(read_reg(&file), registry::export(&registry, &deep, true).unwrap());
    assert!(app.status.contains("out.reg") && !app.status_error, "{}", app.status);
    assert!(registry.log().is_empty(), "an export changes nothing");
    assert!(!app.reg_backup_dir().exists(), "and backs nothing up");

    // On a value, it is the key the panel is on; with keys selected, those.
    cursor_on(&mut app, "Count");
    press(&mut app, "F5");
    set_text(&mut app, "reg.file", 0, &dir.path().join("all.reg").display().to_string());
    press(&mut app, "Enter");
    assert_eq!(settle(&mut app, &registry), [Ok(RegOk::Exported { keys: 4 })]);
    for name in ["Deep", "Other"] {
        cursor_on(&mut app, name);
        let id = app.panel().current().unwrap().id;
        app.panels[0].selection.toggle(id);
    }
    cursor_on(&mut app, "Count");
    press(&mut app, "F5");
    set_text(&mut app, "reg.file", 0, &dir.path().join("two.reg").display().to_string());
    press(&mut app, "Enter");
    assert_eq!(settle(&mut app, &registry), [Ok(RegOk::Exported { keys: 3 })]);
    // A folder that is not there is said, not swallowed.
    press(&mut app, "F5");
    let nowhere = dir.path().join("no").join("such").join("x.reg");
    set_text(&mut app, "reg.file", 0, &nowhere.display().to_string());
    press(&mut app, "Enter");
    assert!(matches!(settle(&mut app, &registry)[..], [Err(RegFailed::File(_))]));
    assert!(app.status_error && app.status.contains("x.reg"), "{}", app.status);
}

/// An ordinary panel on a folder with this `.reg` file in it, the cursor on the file.
fn with_reg_file(text: &str) -> (tempfile::TempDir, MemoryRegistry, App) {
    let (dir, registry, _) = with_registry();
    std::fs::write(dir.path().join("in.reg"), regfile::encode(text)).unwrap();
    let mut app = editing(dir.path(), "in.reg");
    app.reg.registry = Box::new(registry.clone());
    app.reg.available = true;
    app.reg.clock = Some(THEN.into());
    (dir, registry, app)
}

#[test]
fn enter_on_a_reg_file_says_what_it_writes_and_imports_it_after_a_backup() {
    let text = format!(
        "{}\r\n\r\n[{KEY}]\r\n\"Count\"=dword:00000009\r\n\"Path\"=-\r\n\r\n\
         [{KEY}\\Brand new]\r\n@=\"made\"\r\n\r\n[-{KEY}\\Deep]\r\n",
        regfile::HEADER
    );
    let (_dir, registry, mut app) = with_reg_file(&text);
    press(&mut app, "Enter");
    assert!(app.mode.is_normal(), "the file is read on a worker first");
    settle(&mut app, &registry);
    let Mode::Confirm(confirm) = &app.mode else {
        panic!("no question: {:?} {}", app.mode, app.status)
    };
    assert_eq!(confirm.button, 1, "Cancel is lit");
    assert!(confirm.lines[0].contains("in.reg"), "{:?}", confirm.lines);
    assert_eq!(
        confirm.lines[1],
        "Keys: 3 · values written: 2 · keys and values deleted: 2"
    );
    assert!(confirm.lines[2].contains("registry-backups"));
    assert!(registry.log().is_empty(), "nothing before the answer");
    // Cancel leaves the registry alone.
    press(&mut app, "Enter");
    assert!(settle(&mut app, &registry).is_empty() && registry.log().is_empty());

    press(&mut app, "Enter");
    settle(&mut app, &registry);
    agree(&mut app);
    let results = settle(&mut app, &registry);
    let backup = backup_file(&app, "import-in");
    assert_eq!(results, [Ok(RegOk::Imported { backup: backup.clone() })]);
    assert_eq!(registry.value(KEY, "Count"), Some(Value::dword("Count", 9)));
    assert_eq!(registry.value(KEY, "Path"), None);
    let new = registry::join(KEY, "Brand new");
    assert_eq!(registry.value(&new, ""), Some(Value::sz("", "made")));
    assert!(!registry.has_key(&registry::join(KEY, "Deep")));
    assert!(app.status.contains(&backup.display().to_string()), "{}", app.status);
    // The backup: the key written into with its old values, and the whole of the subtree
    // that was deleted; the key that was not there yet has nothing to keep.
    let kept = read_reg(&backup);
    let paths: Vec<&str> = kept.iter().map(|b| b.path.as_str()).collect();
    let deep = registry::join(KEY, "Deep");
    assert_eq!(paths, [KEY.to_string(), deep.clone(), registry::join(&deep, "Deeper")]);
    assert!(kept[0].lines.contains(&Line::Set(Value::dword("Count", 3))));
    assert!(kept[0].lines.contains(&Line::Set(Value::expand_sz("Path", r"%TEMP%\x"))));
}

#[test]
fn an_import_is_not_made_without_its_backup_and_a_bad_file_is_said_to_be_one() {
    let text = format!("{}\r\n\r\n[{KEY}]\r\n\"Count\"=dword:00000009\r\n", regfile::HEADER);
    let (_dir, registry, mut app) = with_reg_file(&text);
    std::fs::write(app.reg_backup_dir(), b"in the way").unwrap();
    press(&mut app, "Enter");
    settle(&mut app, &registry);
    agree(&mut app);
    let results = settle(&mut app, &registry);
    assert!(matches!(results[..], [Err(RegFailed::Backup(_))]), "{results:?}");
    assert!(app.status_error && registry.log().is_empty());

    let (_dir, registry, mut app) = with_reg_file("not a registry file\r\n");
    press(&mut app, "Enter");
    let results = settle(&mut app, &registry);
    assert!(matches!(results[..], [Err(RegFailed::NotARegFile(_))]), "{results:?}");
    assert!(app.mode.is_normal() && app.status_error);
    assert!(app.status.contains("in.reg"), "{}", app.status);
}

#[test]
fn where_there_is_no_registry_a_reg_file_is_a_file_like_any_other() {
    let (_dir, _registry, mut app) = with_reg_file(regfile::HEADER);
    app.reg.available = false;
    press(&mut app, "Enter");
    assert!(
        !app.take_requests().iter().any(|r| matches!(r, Request::Registry(_))),
        "nothing is asked of a registry that is not there"
    );
}

#[test]
fn the_key_bar_says_what_the_keys_do_in_the_registry() {
    let (_dir, _registry, mut app) = with_registry();
    show(&mut app, KEY);
    let bar: Vec<String> = app.keybar().into_iter().map(|(_, s)| s).collect();
    assert_eq!(
        &bar[2..8],
        ["View", "Edit", "Export", "Rename", "MkKey", "Delete"],
        "{bar:?}"
    );
}

#[test]
fn the_quick_search_and_the_sort_work_on_the_rows() {
    let (_dir, _registry, mut app) = with_registry();
    show(&mut app, KEY);
    // Keys first, then values, each by name: the panel's own order.
    let listed = rows(&app);
    let names: Vec<&str> = listed
        .iter()
        .map(|r| r.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(names, ["..", "Deep", "Other", "(Default)", "Big", "Blob", "Count", "List", "Path"]);
    // A row is found by the start of its name.
    assert_eq!(
        app.panels[0].find_row("pat", 0),
        listed.iter().position(|r| r.starts_with("Path"))
    );
}

#[test]
fn a_row_carries_its_own_name_whatever_is_in_it() {
    let (_dir, registry, mut app) = with_registry();
    let odd = r"a/b\c d";
    registry.set_value(KEY, &Value::sz(odd, "x")).unwrap();
    show(&mut app, KEY);
    cursor_on(&mut app, odd);
    assert_eq!(app.reg_row(), Some(regpanel::RegRow::Value(odd.into())));
    cursor_on(&mut app, "(Default)");
    assert_eq!(app.reg_row(), Some(regpanel::RegRow::Value(String::new())));
    cursor_on(&mut app, "Deep");
    assert_eq!(app.reg_row(), Some(regpanel::RegRow::Key("Deep".into())));
    cursor_on(&mut app, "..");
    assert_eq!(app.reg_row(), Some(regpanel::RegRow::Up));
    // The other panel may show the registry too, on a key of its own.
    app.reg_show(1, &registry::join(KEY, "Deep"), None);
    assert_eq!(app.reg_key(0).as_deref(), Some(KEY));
    assert_eq!(app.reg_key(1), Some(registry::join(KEY, "Deep")));
}

#[test]
fn the_work_refuses_a_name_that_is_taken_before_it_backs_anything_up() {
    let (_dir, registry, app) = with_registry();
    let ask = |work: RegWork| RegAsk {
        side: 0,
        backups: app.reg_backup_dir(),
        stamp: THEN.into(),
        work,
    };
    let taken = regpanel::perform(
        &registry,
        &ask(RegWork::RenameValue {
            key: KEY.into(),
            old: "Count".into(),
            new: "BLOB".into(),
        }),
    );
    assert_eq!(taken, Err(RegFailed::Exists("BLOB".into())));
    assert!(!app.reg_backup_dir().exists());
    // Another case of its own name is not somebody else's.
    let recased = regpanel::perform(
        &registry,
        &ask(RegWork::RenameValue {
            key: KEY.into(),
            old: "Count".into(),
            new: "COUNT".into(),
        }),
    );
    assert!(recased.is_ok(), "{recased:?}");
    assert_eq!(registry.value(KEY, "count").unwrap().name, "COUNT");
}
