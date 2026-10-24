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

## 8. Tabs

Per panel, each tab holds cwd, view, sort, filter, selection, cursor. Lockable (path reverts
on close), nameable, reorderable, movable to the other side, duplicable. Persisted in
`session.toml`. Default keys: `Alt+1..9` jump (see `03-keymap.md` §9 for the conflict notes).

Tabs are not a FAR feature, and both keys this once asked for are FAR's own: `Ctrl+T` is the
tree panel as of 0.0.6, and `Ctrl+Tab` is the screens of §10 as of 0.1.0. Tabs will need keys
of their own when they land; `docs/BACKLOG.md` holds the question.

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
| the panel's own status line | a rule and a row above the bottom frame | `panel.status_line` | shipped, **off** by default: the screen-wide status bar already shows the same thing, and two rows saying it is a waste of a row. On, it costs two file rows, as it does in FAR |
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
