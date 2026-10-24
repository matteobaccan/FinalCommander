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

## 8. Tabs

Per panel, each tab holds cwd, view, sort, filter, selection, cursor. Lockable (path reverts
on close), nameable, reorderable, movable to the other side, duplicable. Persisted in
`session.toml`. Default keys: `Ctrl+T` new, `Ctrl+W` close, `Ctrl+Tab` next, `Alt+1..9` jump
(see `03-keymap.md` §9 for the conflict notes).

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
