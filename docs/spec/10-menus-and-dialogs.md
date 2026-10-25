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
5. **Applying has an effect now, and only where it should.** `OK` writes the configuration *and*
   pushes it into the running panels (sort, view, hidden files) — but only the keys the dialog
   itself owns, so a panel's own sort order, chosen with `Ctrl+F3..F7`, survives a trip through
   an unrelated dialog. Nothing waits for a restart, and nothing is written to disk until
   `Shift+F9` (`Save setup`), so experiments are free.

## 2. The menu bar (`F9`)

Five menus, in FAR's order: **Left · Files · Commands · Options · Right**. The bar is drawn on
the top line in the key-bar colours; the open menu is pulled down under its title.

`Left` and `Right` are the same menu aimed at different panels: choosing an entry there acts on
*that* panel and moves the focus to it, whichever panel had the focus before. `F9` opens the
menu of the panel that has the focus.

Keys inside the bar: `←`/`→` move between menus, `↑`/`↓` move over the entries and step over
separators, `Home`/`End` jump to the first and last entry, a letter picks the first entry whose
(translated) label starts with it, `Enter` chooses, `Esc`/`F9`/`F10` close.

A `√` marks the entry that describes the state the panel is in now — the current view mode,
`Hidden and system files` when they are shown, and `Lock the tab to its directory` when it is.

### 2.1 Left / Right

| Entry | Does | State |
| --- | --- | --- |
| Brief / Full / Detailed / Wide | `panel.view.set(...)` | `√` on the current view |
| Info panel · Quick view · Tree panel | `panel.info` · `panel.quickview` · `panel.tree` | |
| New tab · Close tab · Next · Previous · Move to the other panel | `panel.tab.new|close|next|prev|move` | |
| Lock the tab to its directory | `panel.tab.lock` | `√` when locked |
| Name this tab | `panel.tab.name` | |
| Sort modes | opens the sort list (§4.5) | |
| Hidden and system files | `panel.hidden.toggle` | `√` when shown |
| File filter | `panel.filter.quick` | |
| Panel on/off | — (backlog) | |
| Re-read | `panel.reread` | |
| Change drive | opens the drive list (§4.3) | |

### 2.2 Files

View, Edit, Edit new file, Copy, Rename or move, Create link, Make folder, Delete, Delete
permanently, Wipe · Add to archive / Extract files / Archive commands / Test archive ·
Checksums (§3.9), Check against a list · File attributes (§3.8), Apply command · Select group,
Unselect group, Invert selection, Select all, Restore selection · Folder size.

### 2.3 Commands

Find file, Command history (backlog), View and edit history (backlog), Folders history (§4.4),
Folder shortcuts (§4.4b) ·
Swap panels, Sync other panel, Panels on/off, Compare folders, Compare by content · Connections,
Disconnect · User menu, File associations (backlog) · Plugin
commands (backlog), Screens list (backlog: our own viewers, editors and jobs), Task list
(backlog: the operating system's processes, as FAR's is — see `docs/BACKLOG.md`).

### 2.4 Options

System settings, Panel settings, Interface settings, Languages, Themes, Colours · Confirmations,
Viewer settings (backlog), Editor settings (backlog), Code pages (backlog), Files highlighting
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
| Mouse support | `ui.mouse` | **shipped**, off by default (SPEC 02 §12) |
| Always show the menu bar | — | dim (backlog) |

### 3.4 Confirmations

| Row | Key |
| --- | --- |
| Delete | `general.confirm_delete` |
| Overwrite | `general.confirm_overwrite` |
| Exit | `general.confirm_exit` |
| Leaving the editor with unsaved changes | — dim: the editor always asks |

### 3.6 Editor settings (`opt.editor`, Options → Editor settings)

The `[editor]` section, which this dialog is the front of:

| Row | Key | What it does |
| --- | --- | --- |
| Tabs | `editor.tabs` | Three policies, as FAR has: `keep` leaves every tab alone, `typed` turns a newly typed tab into spaces, `all` also expands the tabs already in the file when it is opened |
| Tab width | `editor.tab_width` | How wide a tab is drawn, and how far one indent moves a line |
| Keep the indent of the line above | `editor.auto_indent` | |
| Show spaces and tabs | `editor.show_whitespace` | A space is `·` and a tab is `→` followed by its padding, so trailing space and a mixed indent show |
| The cursor lands after the text a search found | `editor.cursor_after_match` | The match is selected either way; this is where the cursor goes — the start is for reading, the end for typing over it |
| The cursor may go past the end of a line | — | Dim: the setting does not exist. Every movement, every insert and the block arithmetic would have to agree about a column that is not in the text, which is a change through the whole editor rather than a row in a dialog |

`all` is the one with a consequence: expanding what somebody else wrote is a diff nobody asked
for, so it is not the default and the status line says how many lines it touched when it
happens — the next save will write that difference.

### 3.6b Viewer settings (`opt.viewer`, Options → Viewer settings)

The `[viewer]` section, and the same arrangement FAR has: three of these rows are also keys
inside the viewer, and the key writes nothing back. The setting is what a file *opens* with;
the key is what this reader wants of the file in front of them, and a row taken away to read
one long line is not a preference.

| Row | Key | What it does |
| --- | --- | --- |
| Show the top line (name and position) | `viewer.status_line` | Also `Ctrl+Shift+B` inside the viewer |
| Show a scrollbar | `viewer.scrollbar` | Also `Ctrl+S`. Off by default, as in FAR: the top line already says the position as a percentage |
| Show the key bar | `viewer.key_bar` | Also `Ctrl+B`. The interface setting `ui.key_bar` still wins: somebody who turned the bar off everywhere did not mean "except here" |
| Long lines | `viewer.wrap` | `off`, `char` or `word` — what `F2` and `Shift+F2` then change for this file |
| Tab width | `viewer.tab_width` | Eight by default, which is what a terminal and `cat` do: a file that lined something up with tabs was written against that number |
| Remember the position and the code page of each file | — | Dim: it wants the store the histories want |

What each row gives back is a row of content, which is the point of taking it away. The
scrollbar costs a column instead, and it says *where* in the file the reader is rather than how
much of it is on screen: a wrapped line, a hex dump and a rendered Markdown page each answer
"how many bytes fit" differently, so the thumb is one row and its position is the percentage the
top line already shows.

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

### 3.8 File attributes (`Ctrl+A`)

**Implemented** (0.3.0). What a file *is*, rather than what is in it. `Ctrl+A` is FAR's key for
it; select-all moved to FAR's own `Shift+Gray +`, with `Ctrl+Shift+A` beside it for terminals
that do not report `Shift` with a printable key.

The rows are what the platform has:

| Row | Where |
| --- | --- |
| the name, or `N selected items`, and what the file under the cursor is now (`rw-r--r-- (0644)`) | everywhere, as text and not as a box |
| twelve boxes: owner, group and world × read, write, run, then setuid, setgid and sticky | Unix |
| read-only | everywhere — the write bits on Unix, the attribute on Windows, the write bits again over SFTP |
| hidden, system, archive | Windows |
| *Changed* and *Last read*, as `YYYY-MM-DD HH:MM:SS` in local time | everywhere a time can be set |
| *and everything inside the folders* | when a directory is among what was selected |

A row left alone changes nothing: a blank time means "leave it", and the mode's boxes start
from what the file under the cursor has. The dialog is the tallest in the program, so a screen
too short for it scrolls to the cursor and the frame says there is more above or below.

The work is `scmd_jobs::attrs`, and it is deliberately *not* `JobFs::apply_meta`. That one is a
copy's best effort and swallows what it cannot do, which is right after a copy and wrong here:
somebody ticked a box, so a permission that could not be set is collected with its path and
shown. It reports and cancels through the same progress box as a copy ([`JobKind::Attrs`]),
and a directory is changed *after* its contents — taking the write bit off a directory first
would put its own contents out of reach.

Where there is nothing to set, that is said rather than silently done: an entry inside an
archive has the archive's permissions, and an object in a bucket has the time the service wrote
it and an access policy that is not a mode.

**Still to come:** the owner and the group (`chown`), which needs a user and group picker and
the privilege to use it; and Windows' advanced bits (compressed, encrypted, not indexed,
sparse, temporary, offline), which FAR offers behind a second page.

### 3.9 Checksums (Files menu)

Three rows: which hash (SHA-256, SHA-1, MD5), whether one list holds them all, and what the
list is called. The hash the last dialog was answered with is what the next one opens on, as
the copy dialog does, and the name follows it — `<directory>.sha256` becoming
`<directory>.md5`.

The name is a text row rather than a fixed name because the two conventions disagree and both
are right: a commander writes `project.sha256` beside what it is about, a Linux release writes
`SHA256SUMS`, and `sha256sum -c` reads either. With *one list for all of them* off, each file
gets `<file>.<hash>` beside it, which is what a download page offers next to each file.

*Check against a list* takes no dialog: the cursor is on a list, and what it says is read off
the digests in it (`03-file-operations.md` §9). The result is a report naming what does not
match, and those files are left selected in the panel.

Both read the bytes and write nothing but the list, so `Esc` stops a run the way it stops a
comparison by content, and a path this filesystem cannot read a chunk at a time is refused
before the dialog opens rather than halfway through.

### 3.10 Synchronise (Commands menu, `Ctrl+Shift+Y`)

Six rows: into the subdirectories, both ways, offer to take out the extras, compare the bytes,
and a mask — and a line at the top naming the two directories, because which side is "left" is
the whole of what the arrows in the table mean. What the dialog was last answered with is what
the next one opens on, as the copy dialog does.

The defaults are the two that cannot lose anything — walk the trees, leave extras alone — and
the two that can are off until somebody ticks them.

It is not the question, though: the *table* is (`03-file-operations.md` §10a), and this only
says how to work it out.

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

### 4.4b Folder shortcuts (`Ctrl+D`)

The folders kept to hand (`02-panels.md` §11): the ones on a digit first and in order, then
the rest as they were added, each shown as its name and its path or, with no name, its path
alone. The digit is the row's own key. It is the one list that is edited from inside it —
`Ins` keeps the panel's directory, `Del` gives a folder up, `F4` names it — and its hint line
says so. `Enter` goes to the folder, connecting first where it is on a server.

### 4.5 Sort modes (`Ctrl+F12`)

Name · Extension · Size · Modified · Unsorted, `√` on the one in force. Choosing the order that
is already in force turns it round, the way FAR does it — unless `panel.allow_reverse_sort` is
off, in which case nothing changes. `Ctrl+Shift+F12` reverses without opening the list.

## 4b. The palette editor (`opt.colors`, Options → Colours)

Every colour the interface asks for, one per row, in the order they are declared — which groups
them by what they colour. Each row shows the key, the colour as a theme file writes it
(`#rrggbb`, a name, or one of the 256), and the colour twice: as ink on the panel background and
as a background behind the panel's ink. Both, because a colour is only ever used as one or the
other and a single swatch would be half an answer.

`Enter` asks for a colour, with the field opened on what the row is set to, so a shade is nudged
rather than retyped. It takes everything a theme file takes: `red`, `brightcyan`, `default`,
`idx:208`, `ansi:4`, `#ff8000`, `#f80`. Anything else is refused by name and changes nothing.

The change is applied to the **live** theme at once, because a colour one cannot see is a
colour one cannot choose. It is therefore on the screen before it is in a file: `F2` writes the
theme into `scmd.toml` as a `[[theme]]` block under its own id — which the loader already
treats as an override of the built-in of that name, so `Shift+F12` still cycles the themes —
and `Esc` without `F2` says plainly that the colours changed on the screen only.

Still to come: a grid of the sixteen and the two hundred and fifty-six to pick from rather than
typing a name, and the colour *groups* FAR has (one change for every "selected" colour at
once).

## 5. Save setup (`Shift+F9`)

Writes the configuration as it stands now to `paths.config_file` (`scmd.toml`), creating the
directory if needed, and appends any theme that came from the configuration file so nothing is
lost on the way through. The status line names the file it wrote. What is written parses back
into the same configuration — there is a test for exactly that round trip.

## 6. What is deliberately not here

See `docs/BACKLOG.md`. Every dim entry in a menu has a line there.
