# SPEC 02 — Panels

## 1. Panel state

```rust
pub struct PanelState {
    pub id: PanelId,                 // Left | Right (M4: arbitrary)
    pub cwd: VPath,
    pub entries: Arc<Listing>,       // immutable snapshot, swapped atomically
    pub cursor: usize,               // index into the *visible* order
    pub top: usize,                  // first visible row
    pub selection: Selection,        // stable ids, survives re-sort and re-read
    pub view: ViewMode,              // Brief | Medium | Full | Wide | Detail | Custom(n)
    pub sort: Sort,                  // key + reverse + dirs_first + selected_first + chain
    pub filter: Option<FilterSet>,
    pub show_hidden: bool,
    pub tabs: Vec<Tab>, pub active_tab: usize,
    pub history: History<VPath>,     // back/forward
    pub mode: PanelKind,             // Files | Tree | QuickView | Info | Temp | Search
}
```

`Listing` holds the entries plus derived indices:

```rust
pub struct Listing {
    pub entries: Vec<Entry>,         // arena-ish: names in one Box<[u8]>, metas in a Vec
    pub order: Vec<u32>,             // sorted + filtered view onto entries
    pub by_id: HashMap<EntryId, u32>,
    pub totals: Totals,              // files, dirs, bytes, selected
    pub generation: u64,
}
```

- `Entry` is 48 bytes or less: name offset/len, `EntryKind`, size, mtime, flags, optional
  `Box<ExtMeta>` for owner/links/xattr fetched lazily.
- Selection is stored as a set of `EntryId` (device+inode on Unix, file id on Windows, path
  hash on VFS) so it survives sorting, filtering and re-reads.

## 2. Reading a directory

1. Dispatch `panel.read(path, generation)` to the VFS task; cancel the previous generation.
2. The provider streams entries in chunks (default 2 048); the panel paints after the first
   chunk (progressive listing), with a spinner in the status line until complete.
3. Sorting runs on the rayon pool for listings above 10 000 entries, otherwise inline.
4. Expensive metadata (owner names, link targets, archive comments, git status) is fetched by a
   second pass and merged by `EntryId`, re-rendering only the affected rows.
5. Directory sizes are never computed implicitly; `file.dirsize` (`F3`/`Space` on a dir) starts
   a cancellable recursive scan whose partial result is shown live.
6. A `notify` watcher on the cwd triggers a debounced (200 ms) incremental re-read: only the
   changed names are re-stat'ed, cursor and selection are preserved.

Failure modes: permission denied → show the directory with an error banner and whatever was
readable; path vanished → walk up to the nearest existing ancestor; stalled provider → the
panel stays interactive, a cancel action appears after 2 s.

## 3. View modes and columns

A view mode is a list of columns; column types:

```
name[.short|.full]  ext  size[.bytes|.human|.dirs=count]  packed_size
mtime|ctime|atime|btime[.fmt]  attrs|perm  owner  group  links  target
descr  checksum  git  custom:<plugin-id>
```

Built-in presets (names kept close to FAR's so veterans recognise them): `brief` (names in
N columns), `medium`, `full` (name, size, mtime), `wide`, `detail` (name, size, mtime, attrs,
owner), `links`, `owners`, `descr`, plus `custom:0..9` from config:

**Shipped (0.0.6)**: the first five, in FAR's order and on FAR's keys — `Ctrl+1` brief (three
columns of names), `Ctrl+2` medium (two), `Ctrl+3` full, `Ctrl+4` wide (name and size, two
columns), `Ctrl+5` detailed. A mode says how many columns it draws and whether it shows a size,
so the layout follows from the mode rather than from a width guess, and a column too narrow for
what it holds falls back to fewer columns. `links`, `owners`, `descr` and the custom sets want
data we do not collect yet and are lines in `docs/BACKLOG.md`.

```toml
[[panel.view.custom]]
id = 0
columns = [
  { type = "name",  width = "*" },
  { type = "size",  width = 10, align = "right", fmt = "human" },
  { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
  { type = "git",   width = 2 },
]
```

Column widths: fixed, percentage, or `*` (fill). Horizontal scrolling for long names
(`Alt+←/→`), ellipsis in the middle for paths and at the end for names.

### 3b. Reading FAR's column strings

Our column definition is a TOML table; FAR's is a pair of strings (`Column types`,
`Column widths`) that every FAR user already has in their configuration, in their saved panel
modes and in their find-results layout. We parse that spelling too — on import (SPEC 07 §6) and
wherever a string is more convenient than a table. The grammar, taken from FAR 3.x
(`docs/04-far-inventory.md` §2):

```
N[M[D],O,R[F],N]      name: M selection marks (D dynamic), O without path,
                      R right-align what does not fit (F always), N without extension
X[R]                  extension (R right-aligned)
S / P / G [C,T,F,E]   size / allocated / stream size: C group digits, T decimal units,
                      F fraction with three significant digits, E no space before the unit
D   T                 write date, write time
DM / DC / DA / DE [B,M]   write / creation / access / change date+time: B brief, M month name
A    attributes    Z  description    O[L] owner (L with domain)
LN   hard links    F  streams
```

Widths: a number, `0` for automatic, or `n%` of the space left after the fixed columns; `+1` on a
time column means 12-hour, more gives seconds and milliseconds; `+2` on a date column gives a
four-digit year. A repeated group of columns means newspaper stripes. Our equivalents are named
(`{ type = "mtime", fmt = "…" }`), and the mapping is one to one for everything we support;
what we do not support (allocated size, streams, descriptions before they exist) is reported as
a warning on import, never dropped silently.

Per-mode flags from the same dialog: fullscreen, align file extensions, align folder extensions,
folders uppercase, files lowercase, uppercase names shown lowercase.

## 4. Sorting

Keys: name, ext, size, mtime, ctime, atime, btime, attrs, owner, descr, packed, type, none
(directory order). Modifiers: reverse, directories first (default on), selected first,
case-insensitive (default on Windows/macOS), **natural/version order** for digits, locale
collation via ICU-less `unicode-collation`-style folding. A **criteria chain** breaks ties
(FAR's idea): `sort.chain = ["size", "name"]`.

Pressing the same sort key twice toggles direction when `panel.allow_reverse_sort = true`; so
does choosing the current order again in the sort list (SPEC 10 §4.5).

**Sort groups** (FAR's `Shift+F11`) sit on top of the sort: an ordered list of named mask groups,
some pulling their files ahead of everything else and some pushing them behind, with the
boundary between "front" and "back" explicit in the list. Within a group the normal sort
applies. This is what makes `*.cpp` files appear before `*.o` ones however the panel is sorted.

## 5. Filters

- **Quick filter bar** (`Alt+I`): live substring/glob, applied to the visible order only.
- **Filter sets**: named rules combining masks (`*.rs,*.toml`), exclude masks, size range, age
  range, attributes, and entry kind; stored in `filters.toml`, selectable from a menu,
  lockable so a changed directory keeps the filter.
- One filter object, three users. A filter is not a panel feature that find and copy happen to
  copy: it is one object the panel display, the find tool and the copy engine all take. FAR
  proves the shape (its filter dialog is reached from all three) and it halves the work.
  A filter condition set is: masks and exclude masks · a size range, with binary suffixes
  (`B K M G T P E`) and lowercase for decimal · a date range on a *chosen* timestamp (write,
  creation, access, change), given either absolutely or **relative in days**, which is what
  makes a filter survive being saved · attributes as three states (must have, must not have,
  don't care) · "more than one hard link". Each condition switches off on its own, so a saved
  filter can be narrowed without being rewritten.
- **Highlight groups** use the same predicate language to colour rows:

```toml
[[highlight]]
mask = "*.rs"; fg = "syntax.rust"
[[highlight]]
when = "attrs.contains('x') && kind == 'file'"; fg = "exec"
[[highlight]]
when = "age < '1h'"; attrs = ["bold"]
```

The predicate language is a tiny expression evaluator (no Turing completeness, no I/O) shared
with selection-by-predicate and the find tool.

## 6. Fast find / quick search

Typing while the command line is empty starts fast find (configurable: FAR uses `Alt`+letters,
mc uses bare letters). Matching: prefix, then substring, then fuzzy, with `*`/`?` wildcards
accepted literally. `Ctrl+Enter` / `Ctrl+Shift+Enter` cycle matches forward/backward. The match
is highlighted inside the name. `Esc` restores the previous cursor position.

## 7. Special panel kinds

| Kind | Behaviour |
| --- | --- |
| **Tree** | directory tree of the current root, lazily expanded, cursor movement changes the *other* panel's path; `*` expands all under cursor (bounded) |
| **Quick view** | renders the viewer's output for the item under the cursor in the other panel, debounced 120 ms, size-capped, cancellable |
| **Info** | fs type, capacity/free, mount options, selected totals, VFS/archive details, git branch/status summary |
| **Temp** | a virtual collection of entries from anywhere; add with `Ins`-like command, operations act on the real paths (FAR's `tmppanel`) |
| **Search** | result list of `tool.find` (panelize); each row keeps its real path; operations work normally |

When the active panel is Tree/QuickView/Info, directory-changing commands apply to the other
panel (FAR's rule) — this keeps `Enter` meaningful everywhere.

### 7a. What shipped in 0.0.6, and the rule they share

**Quick view** (`Ctrl+Q`), **Info** (`Ctrl+L`) and **Tree** (`Ctrl+T`), all three in the
*passive* panel, each key turning its kind off again, and all three following the other
panel's cursor.

| | Quick view | Info | Tree |
| --- | --- | --- | --- |
| What it shows | the viewer's own rows for the file under the other panel's cursor | where that panel is, what kind of place that is, the free space, what the directory holds, what is selected, the row under the cursor | the directories under that panel's root |
| Where the focus goes | to the files: there is nothing to select in a preview | the same | **into the tree**, because moving its cursor is how it changes the other panel's directory |
| What it reads | the file, once, and again only when the cursor moves to another | nothing: everything shown is already known | one directory per expansion, through the worker, cached beyond the panel's life |
| What it refuses | a directory (`F3` measures one) and a file whose bytes are not local (`F3` brings them out) | — | nothing: a directory it cannot read is an empty branch |

The tree is rooted where the other panel's path starts — the drive, the archive, the server —
and opens its way down a level per read until the cursor is on the directory that panel is
showing, so it comes up where the user already is. `*` opens everything under the cursor up to
two thousand rows, so `*` at the root of a disk cannot hang the panel.

`Ctrl+L`'s divergence is settled with this: it is FAR's info panel, and our
change-directory dialog moved to `Ctrl+Shift+L`, which it can afford now that `cd` works on
the command line.

## 8. Tabs (shipped)

A tab **is** a panel. There is no tab object holding a panel beside a name and a lock,
because everything a tab has to remember — where it is, its view and sort, its filter, what
is selected, where the cursor is, where it has been, what it is called, whether it is
locked — is already what a panel remembers. So one side's tabs are a list of panels with one
of them on screen, and `crates/scmd/src/tabs.rs` is that list.

The one on screen is **not** in the list: it is `App::panels[side]`, and its slot in the row
is empty. A parked copy of it beside the live one would be a second answer to where the panel
is, and the two would drift apart within a keystroke. Switching is therefore a move out of
the slot and the live panel's move into it — no copy, whatever the directory holds.

What the keys do:

| | |
|---|---|
| `Ctrl+Alt+T` | another tab, on the directory under the cursor if the cursor is on one and otherwise on this one — a duplicate |
| `Ctrl+Alt+W` | close this tab; a side always has one, so the last one stays and says so |
| `Ctrl+Alt+N` / `Ctrl+Alt+P` | the next tab, the previous one; the row is a ring |
| `Ctrl+Alt+M` | hand this tab to the other side, which is a move: it leaves this one |
| `Ctrl+Alt+K` | lock this tab to its directory, or let it move again |
| Left/Right menu | the same six, and *Name this tab* |

A **locked** tab refuses to change directory, whichever way it is asked — `Enter`, the
history, the tree driving it from the other side, a plugin, a connection opening on it — and
says why rather than doing nothing, which would read as a key that had stopped working. The
one case that overrides it is a connection closing under the tab: a tab locked to a directory
on that connection cannot keep its promise, so the lock comes off rather than the panel
staying on a directory that is no longer reachable.

The row is drawn above the panel, in its own row, and **only by a side that has more than one
tab**: a bar that was always there would cost every panel a row of files to say that it has
one tab. The current tab is drawn as the path is — inverted, and in the header's inactive
colour on the panel without the focus — a locked one is marked, and the row scrolls to keep
the current tab whole, with guillemets where it goes on.

Tabs are **persisted** in the state file (`scmd-state.toml`), with their names and their
locks. Two rules on the way back in, both about what the user asked for most recently: the
directory this session was started in wins for the tab that is on screen, because that is
what running a program in a directory means — unless that tab is locked, since staying put is
the whole of what a lock promises. And a tab that was on a server or in a bucket is not
restored at all: the connection it needs is not open, and a row of tabs that each say "not
connected" is worse than a row that is not there.

Tabs are not a FAR feature, and both keys this once asked for are FAR's own: `Ctrl+T` is the
tree panel as of 0.0.6, and `Ctrl+Tab` is the screens of §10 as of 0.1.0. Hence the `Ctrl+Alt`
row, which is also where the terminal lives — and not `Ctrl+Alt` with an arrow, which most
desktops take for themselves before a terminal ever sees it, nor `Ctrl+Alt+L`, which locks
the screen on most of them.

Still owed: reordering a tab within its row, and a list of them to pick from.

## 11. The folders kept to hand (shipped)

Ten **folder shortcuts**, which is FAR's own number: `Ctrl+Shift+0..9` puts the panel's
directory on a digit, `Alt+0..9` goes to it. FAR goes to one with `RightCtrl+0..9`, and no
terminal can tell the two control keys apart (`03-keymap.md` §9), so `Alt` is ours.

A digit holds one folder. Setting it over a folder that had it replaces that folder and the
name with it, because the name belonged to the old one; a folder already kept without a digit
is given the digit rather than kept twice.

`Ctrl+D` opens the whole **hotlist**, Total Commander's key for it: one list of folders, the
ones with a digit first and in order, then the rest as they were added. It is the one list
one edits from inside it — `Ins` keeps the panel's directory with no digit of its own, `Del`
gives a folder up, `F4` names it — because a folder is read, kept and named in the same
place. A row's digit is its own key, so one press takes it.

A shortcut may be on a server or in a bucket, in which case going to it is connecting first:
the hotlist takes the same route the drive list does (`04-vfs.md` §6).

They are kept in `scmd-state.toml` rather than in `scmd.toml`, because a *key* sets them: a
shortcut that waited for *Save setup* and vanished without it would be a bug, not a setting.

Still owed: FAR's habit of inserting the path instead of going there when an edit line has
the keys, and reordering the list.

## 10. Screens (`F12`)

A viewer or an editor is a **screen**, not a dialog: one steps away from it and comes back,
and it keeps its position, its search, and its unsaved changes while the panels, or another
file, are worked on. The panels are screen zero — always there, never closed.

`F12` lists them: the panels first, then each screen with its kind and its file, and a `*` in
front of one holding changes nobody has written. Choosing a row brings that screen forward and
puts the one that was in front behind it. The number of screens behind the panels is drawn in
the corner, `[2]`, and only when there is one.

`Ctrl+Tab` and `Ctrl+Shift+Tab` round the ring without the list. The panels are its first
member and the screen in front its last, so `Ctrl+Tab` from a screen steps out to the panels
and from the panels steps in to the next screen: the panels are never more than one key away,
and every screen is reached by pressing it again. Going straight from one screen to another is
what the list is for — a ring that walked screen to screen would have to own the application's
mode rather than shadow it, and the backlog carries that.

A dialog is not a screen. A question asked over a viewer belongs to that viewer, and stepping
away puts it back on the screen it was asked about rather than leaving it in front of the
panels. Nothing in the list is ever closed by switching: a screen leaves only through its own
`F10`, which is what asks about unsaved changes — and `F10` from the panels asks too, naming
every background screen that holds some, because the panels cannot otherwise know.

## 9. Layout

```
┌ menu bar (hidden unless F9 / always, configurable) ────────────┐
│ ┌ left panel ──────────────┬ right panel ───────────────────┐ │
│ │ header: path + tabs      │ header                         │ │
│ │ rows                     │ rows                           │ │
│ │ footer: totals/status    │ footer                          │ │
│ └──────────────────────────┴────────────────────────────────┘ │
│ user screen / terminal pane (optional, resizable)             │
│ command line: user@host cwd$ _                                │
│ key bar: 1 Help 2 Menu 3 View 4 Edit 5 Copy …                 │
└────────────────────────────────────────────────────────────────┘
```

- Split: vertical (default) or horizontal; ratio draggable and settable (`Ctrl+←/→`), reset
  with `Ctrl+Alt+=`.
- Height: `Ctrl+↑` brings the panels' bottom edge **up** — shorter panels, and the rows they
  give up go to the user screen underneath (`06-tools.md` §7a); `Ctrl+↓` gives them back. The
  command is `panel.height(N)` and its sign is the panels' *height*, which is what the keys
  read as; the setting it moves (`panel.height_cut`) is what they give up, so the two have
  opposite signs and exactly one place says so.
- Any element can be hidden; hiding both panels gives the full-screen shell (FAR `Ctrl+O`).
- Minimum usable size 60x15; below that the panels stack and the key bar collapses to one row.

### 9a. A panel that is a list rather than a directory (shipped)

Find results, `panelize`, and later an archive's contents and a plugin's own list are all the
same thing: a panel whose rows are files wherever they live. It comes down to one method —
`Panel::path_of` — which is the single place that decides whether a row's path is the panel's
own path with the name on the end, or something the panel was told. Every operation, the
viewer, the editor and the metasymbols go through it, so none of them needs to know which kind
of panel it is looking at.

| Question | Answer |
| --- | --- |
| What is a row's path? | The list's own, kept in a map keyed by entry id — *not* by row, because the listing re-sorts underneath it |
| What does the frame say? | What the list is of, rather than a directory the rows are not in |
| What does `Enter` do on a file? | Goes to the directory the file lives in, with the cursor on it: that is what a result is for |
| What does a re-read do? | Asks about each of its own paths again, dropping what is no longer there — a results panel that still shows a deleted file is a liar |
| What does navigating away do? | Turns the panel back into a directory; a list is never half a directory |
| What is `{dir}` on one? | The directory of the file under the cursor, because the panel's own has nothing to do with the rows |

### 9b. The panel's own frame (shipped)

What FAR draws in and around a panel's frame, and what we do about it:

| Element | Where | Setting | State |
| --- | --- | --- | --- |
| path | top frame, centred, elided in the middle | — | shipped; inverted on the active panel |
| sort letter and filter mark | left end of the titles row, or of the top frame without it | `panel.column_titles` | shipped |
| column titles | row under the top frame | `panel.column_titles` | shipped, on by default |
| totals, or the selected count, or the filter | bottom frame, left | — | shipped |
| free space on the device | bottom frame, right | `panel.free_space` | shipped; `statvfs` on Unix, nothing on a server or on Windows, and it says nothing rather than guessing |
| scrollbar | right frame | `panel.scrollbar` | shipped; drawn only when there is more than fits |
| the panel's own status line | a rule and a row above the bottom frame | `panel.status_line` | shipped, **on** by default, as in FAR: the rule carries the selected totals and the row under it names the file the cursor is on. It costs two file rows |
| one panel full width | `Ctrl+F1` / `Ctrl+F2` | — | shipped; the focus follows the screen |
| long names scrolled sideways | `Ctrl+Shift+←/→` | — | shipped, per panel, reset on a change of directory |
| free space, screen counter, clock in the corner | — | — | the counter and the clock want a repaint on a timer; see the backlog |

### 9c. Comparing the two panels (shipped)

FAR's own rule, which is why it is instant: names, sizes and times — no subdirectories and no
contents. In each panel it selects the files the other does not have and the ones it has in an
older version, so what stays selected is exactly what a copy would have to carry, and a file
that is the same on both sides is selected in neither. Case is ignored when matching names,
because the two panels may be on filesystems that disagree about it; a pair that differs only
in case is then a difference the user sees rather than a pair we quietly match up. With no
timestamp on either side the size is all there is to go on. Directories are left alone.

## 12. The mouse (shipped, off by default)

`ui.mouse` turns it on; it is **off** until somebody asks, and that is a decision rather than
an oversight. While a terminal is reporting the mouse it stops acting on it: selecting text
with the mouse and the middle-click paste go away, and those are the user's own tools, which
this program cannot replace. Somebody who wants the mouse says so once in *Interface settings*
and keeps it; everybody else keeps their terminal.

What it does, and why it is this and not more:

| Action | What happens |
| --- | --- |
| left click on a row | that panel becomes the active one and the cursor goes to the row |
| left click inside a panel, past the last row | the panel becomes active and the cursor stays where it was |
| double click on a row | the same as `Enter` on it: into a directory, into an archive, or open the file |
| right click on a row | the cursor goes there and the row's selection is toggled — FAR's and mc's button, and nothing here wants it for a context menu |
| wheel | scrolls what is **under the pointer**, three rows a notch, without taking the focus: looking at the other panel is not working in it |
| left click on the key bar | presses the key written in the cell, with whatever modifiers are held — so `Shift` and a click is `Shift+F5` |
| left click on the menu bar | opens that menu; one click on a dropdown row runs it |
| left click on a dialog's list | moves its cursor; a double click answers the dialog |
| left click outside an open dialog | nothing at all, and in particular not the panel underneath: a click must not move a cursor the user cannot see |
| left click in the editor | puts the caret there |

Every one of them ends up in the same `run` the keyboard uses, so the macro recorder, the
confirmations and the error reporting are the ones already there — the mouse does what a key
does rather than having powers of its own.

Still to come: dragging to extend a selection, dragging a column rule to resize it, dragging
between the panels, and the sideways wheel. Each wants state the click handling does not keep
yet, and none of them is the difference between having a mouse and not.
