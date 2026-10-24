# SPEC 10 — Menu bar, settings dialogs, choose-one lists

FAR's menu bar is the discoverable half of the program: everything the keys do, a user can also
find by reading. This spec says what the menus contain, what the dialogs contain, what each row
writes into the configuration, and how all of it behaves. It is implemented in
`crates/scmd/src/menu.rs` (the model), `crates/scmd/src/form.rs` (dialog state) and
`crates/scmd-ui/src/widgets.rs` (drawing).

## 1. Principles

1. **A menu entry is a command.** Every entry resolves to a `Command` from the vocabulary
   (SPEC 01 §4), the same value a key press resolves to. There is no behaviour reachable only
   from a menu, and none reachable only from a key.
2. **The shortcut shown is the shortcut that works.** The key next to an entry is read from the
   keymap in force, so a rebinding in `scmd.toml` shows up in the menu. Entries for features
   that are not here yet carry FAR's key as a *reserved* hint, and a test
   (`the_menu_never_advertises_a_key_that_does_something_else`) fails if that key has meanwhile
   been given to something else.
3. **No holes, no lies.** An entry whose feature is missing stays in the menu, is drawn dim, and
   when chosen says `not implemented: <command>`. A menu is also a map of the program; a map
   with gaps marked is more useful than a map that pretends.
4. **A dialog is a view of the configuration.** Every settable row carries the configuration key
   it edits and applies through `Config::set(key, value)` — the same code path as
   `scmd --set key=value`. One place decides what a setting means; a dialog cannot drift from
   the file format.
5. **Applying has an effect now.** `OK` writes the configuration *and* pushes it into the
   running panels (sort, view, hidden files). Nothing waits for a restart. Nothing is written to
   disk until `Shift+F9` (`Save setup`), so experiments are free.

## 2. The menu bar (`F9`)

Five menus, in FAR's order: **Left · Files · Commands · Options · Right**. The bar is drawn on
the top line in the key-bar colours; the open menu is pulled down under its title.

`Left` and `Right` are the same menu aimed at different panels: choosing an entry there acts on
*that* panel and moves the focus to it, whichever panel had the focus before. `F9` opens the
menu of the panel that has the focus.

Keys inside the bar: `←`/`→` move between menus, `↑`/`↓` move over the entries and step over
separators, `Home`/`End` jump to the first and last entry, a letter picks the first entry whose
(translated) label starts with it, `Enter` chooses, `Esc`/`F9`/`F10` close.

A `√` marks the entry that describes the state the panel is in now — the current view mode, and
`Hidden and system files` when they are shown.

### 2.1 Left / Right

| Entry | Does | State |
| --- | --- | --- |
| Brief / Full / Detailed / Wide | `panel.view.set(...)` | `√` on the current view |
| Info panel · Quick view · Tree panel | — (backlog) | |
| Sort modes | opens the sort list (§4.5) | |
| Hidden and system files | `panel.hidden.toggle` | `√` when shown |
| File filter | `panel.filter.quick` | |
| Panel on/off | — (backlog) | |
| Re-read | `panel.reread` | |
| Change drive | opens the drive list (§4.3) | |

### 2.2 Files

View, Edit, Edit new file, Copy, Rename or move, Create link (backlog), Make folder, Delete,
Wipe (backlog) · Add to archive / Extract files / Archive commands (backlog) · File attributes
(backlog), Apply command (backlog) · Select group, Unselect group, Invert selection, Select all,
Restore selection · Folder size.

### 2.3 Commands

Find file, Command history (backlog), View and edit history (backlog), Folders history (§4.4) ·
Swap panels, Sync other panel, Panels on/off, Compare folders (backlog) · Connections,
Disconnect · User menu, File associations (backlog), Folder shortcuts (backlog) · Plugin
commands (backlog), Screens list (backlog), Task list (backlog).

### 2.4 Options

System settings, Panel settings, Interface settings, Languages, Themes · Confirmations, Viewer
settings (backlog), Editor settings (backlog), Code pages (backlog), Files highlighting
(backlog), Plugins configuration (backlog) · Help, Save setup, Quit.

## 3. Settings dialogs

A dialog is a framed list of rows above a row of buttons. Row kinds:

* `[x] Label` — a boolean configuration key. `Space` or `Enter` flips it.
* `Label: value ↓` — a key with a fixed set of values. `Space`/`→` take the next value,
  `←` the previous one.
* a dim `[ ] Label` — in the dialog for completeness, not settable yet (its setting does not
  exist in the configuration either).
* a separator line.

Keys: `↑`/`↓` move over the settable rows, `Tab` moves to the buttons and between them, `Enter`
on the buttons answers the dialog, `F10` is `OK` from anywhere, `Esc` cancels and keeps nothing.

### 3.1 System settings

| Row | Key | Values |
| --- | --- | --- |
| Natural number order | `panel.natural_sort` | on/off |
| Allow reverse sort | `panel.allow_reverse_sort` | on/off |
| Use the system recycle bin | — | dim (backlog) |
| Use the system copy routine | — | dim (backlog) |
| Keep history between sessions | — | dim (backlog) |
| Remote file size limit | `net.fetch_limit_mb` | 64 / 256 / 512 / 2048 MB |
| SSH keep-alive | `net.keepalive_secs` | off / 15 / 30 / 60 s |

### 3.2 Panel settings

| Row | Key | Values |
| --- | --- | --- |
| Show hidden and system files | `panel.show_hidden` | on/off |
| Folders first | `panel.dirs_first` | on/off |
| Selected files first | `panel.selected_first` | on/off |
| Reverse order | `panel.reverse` | on/off |
| Panel view | `panel.view` | brief / full / detail / wide |
| Sort by | `panel.sort` | name / ext / size / mtime / none |
| Quick search | `panel.quick_search` | with Alt / by letters / off |

### 3.3 Interface settings

| Row | Key | Values |
| --- | --- | --- |
| Show the key bar | `ui.key_bar` | on/off |
| Show the status bar | `ui.status_bar` | on/off |
| Keymap | `ui.keymap` | far / mc |
| Clock in the corner | — | dim (backlog) |
| Mouse support | — | dim (backlog) |
| Always show the menu bar | — | dim (backlog) |

### 3.4 Confirmations

| Row | Key |
| --- | --- |
| Delete | `general.confirm_delete` |
| Overwrite | `general.confirm_overwrite` |
| Exit | `general.confirm_exit` |
| Leaving the editor with unsaved changes | — dim: the editor always asks |

### 3.5 Session editor (`Alt+N` → `Ins`/`F7` new, `F4` edit)

The only dialog with text in it, and FAR's NetBox session editor is the model. Rows: name, host,
port, user · authentication (SSH agent / key file / ask for a password) and the key file ·
remote folder, group, keep-alive seconds, compression.

A text row takes the editing keys — printable characters, `Backspace`, `Del`, `←`, `→`, `Home`,
`End` — and shows a real caret; `↑`/`↓` leave the row, `Enter` in a text row means `OK` because
there is nothing to toggle. No password or passphrase is ever a field: a session says *how* to
authenticate and the secret is asked for when the connection needs it (SPEC 04 §6).

`OK` validates before it stores: a host is required, a port and a keep-alive must be numbers, a
key file is required when the key authentication is chosen, and what comes out has to be a site
the netbox can actually use (`Site::from_config`). A dialog that cannot be applied stays open
with the reason in the status line — nothing unusable is ever written. On success the site is
added to, or replaced in, `[[site]]`, the site list is rebuilt, and the list reopens on the row
that was just stored. The site lives in the running configuration; `Shift+F9` makes it permanent,
and the status line says so.

Defaults left empty mean what they mean in `scmd.toml`: no port is 22, no user is the local
account name, no name is the host itself.

## 4. Choose-one lists

One framed list, one `√` on the row in force, `↑`/`↓`/`PgUp`/`PgDn`/`Home`/`End` to move,
`Enter` to apply, `Esc` to close. The hint line is in the bottom frame.

### 4.1 Languages

The seven shipped catalogues by endonym with their tag. `Enter` switches the interface
immediately (menus, dialogs, key bar, messages) and sets `ui.language`.

### 4.2 Themes

The built-in themes plus any declared in `scmd.toml`. `Enter` applies it and sets `ui.theme`.
`Alt+T` still cycles through them without opening the list.

### 4.3 Change drive (`Alt+F1` left, `Alt+F2` right)

The roots this platform has — `/` and the home directory on Unix, the drive letters that answer
on Windows — followed by every open SFTP connection as `sftp://user@host`. `Enter` takes *that*
panel there; for a connection that is configured but not open, it connects first.

### 4.4 Folders history (`Alt+F12`)

Where this panel has been, newest first, without repeats. `Enter` goes back to it.

### 4.5 Sort modes (`Ctrl+F12`)

Name · Extension · Size · Modified · Unsorted, `√` on the one in force. Choosing the order that
is already in force turns it round, the way FAR does it — unless `panel.allow_reverse_sort` is
off, in which case nothing changes. `Ctrl+Shift+F12` reverses without opening the list.

## 5. Save setup (`Shift+F9`)

Writes the configuration as it stands now to `paths.config_file` (`scmd.toml`), creating the
directory if needed, and appends any theme that came from the configuration file so nothing is
lost on the way through. The status line names the file it wrote. What is written parses back
into the same configuration — there is a test for exactly that round trip.

## 6. What is deliberately not here

See `docs/BACKLOG.md`. Every dim entry in a menu has a line there.
