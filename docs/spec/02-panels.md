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
6. A `notify` watcher on the cwd triggers a debounced (300 ms) re-read of the directory, so a
   file that arrives from somewhere else appears without `Ctrl+R`. One watch per panel, local
   paths only — a server reports nothing locally and an archive is a file — and the whole
   directory is re-read rather than the changed names re-stat'ed: a local listing is cheap and
   a partial update is a cache to get wrong. `set_listing` keeps the cursor on the name it was
   on and the selection on the names that are still there, so nothing moves under the user.
   `panel.auto_refresh` turns it off; a directory that has just been deleted is left on screen
   as it was rather than re-read into an error.

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
owner), `links`, `owners`, `descr` — each of the ten re-definable from the configuration
(§3a):

**Shipped (0.0.6)**: the first five, in FAR's order and on FAR's keys — `Ctrl+1` brief (three
columns of names), `Ctrl+2` medium (two), `Ctrl+3` full, `Ctrl+4` wide (name and size, two
columns), `Ctrl+5` detailed. The full view draws the extension in a column of its own, as FAR
does: the names run down the left of the name field and the extensions down its right, against
the size column, which is what makes a directory of one kind of file read as a list rather than
as a paragraph. Five columns of the field, four of them the extension — a directory (`docs.old`
is a folder, not a `.old` file), a dotfile, and an extension too long for the column keep their
name whole across both. A mode says how many columns it draws and whether it shows a size,
so the layout follows from the mode rather than from a width guess, and a column too narrow for
what it holds falls back to fewer columns. **The custom sets are shipped** (§3a): each of the
ten modes is re-definable from the configuration, on its own digit.

**The wide view** (`Ctrl+4`) is name and size, in as many columns as the *names* allow:
a directory of short names gets two, and a directory of download names —
`wetransfer_…_2026-10-06_1431.zip` — gets one, with the whole name readable. That is FAR's
own rule and it is the point of the mode; the brief and medium views divide the width as
they always did, because they are about seeing many names at once and a name too long for
its column is cut. The longest name is measured once per listing, in the pass that filters
it, in bytes rather than display columns — a directory of 1.7 million rows is a pass worth
keeping cheap, and a name with accents in it then errs towards fewer columns, which is the
safe direction.

**All ten of FAR's view modes are there**, on FAR's own numbers: `Ctrl+6` names with their
descriptions and `Ctrl+7` the same with the size and the time (§13), `Ctrl+8` the owners,
`Ctrl+9` the links, `Ctrl+0` the alternative full view. The last three need something a
listing does not carry, and each is fetched the same way — by a worker, while that view is
the one on screen, with a row the answer has not reached yet drawn blank rather than guessed
at. The font reset moved off `Ctrl+0` to `Ctrl+Alt+0` to make room: a number key in the
panels is a view, all ten of them.

**The links view** (`Ctrl+9`) puts how many names a file has between the name and the size.
One is an ordinary file; two or more is a file that is in two directories at once, and a
column of them is how a hundred copies that are really one file stop looking like a hundred
files' worth of space. The count costs a lookup per row — on Unix the `nlink` of a `stat`,
on Windows a file that has to be opened for `GetFileInformationByHandle` — so it is fetched
like the owners are, and a file nobody may open leaves the column blank rather than claiming
one name.

**The alternative full view** (`Ctrl+0`) is the full view with the sizes written out, digits
grouped: `1 234 567` where the full view says `1.18 M`. It is for when the difference between
1.2M and 1.9M is the thing one came to see. The totals in the bottom frame stay abbreviated,
because they are a summary and not a figure to compare.

**The owners view** (`Ctrl+8`, FAR's own number for it) draws `name │ size │ owner  group`:
the two names take the room the date had, because somebody asking who a file belongs to is
not asking when it was written. On a panel too narrow to keep a readable name as well, the
group goes and the owner stays — the owner is what was asked for.

Who a file belongs to is **not part of a listing**, and that is the whole design of this mode.
It costs a lookup per file: a `getpwuid` through NSS on a Unix, which may mean a directory
server, and on Windows a security descriptor read plus a SID lookup that may mean a domain
controller. So nothing is paid for it unless this is the view on screen: choosing the mode
sends a request, a worker answers with a map from name to the two names, and the panel — which
has been on screen all along — fills the two columns in when it arrives. A row the map does
not mention yet is blank rather than a guess. One cache serves a whole directory, because a
listing is nearly always one or two accounts repeated. Reading the directory again drops the
map and asks afresh, since the names it was about are gone. A panel that is a list, one on a
remote filesystem and one inside an archive ask nothing at all: there is no owner there to
report. A number with no account behind it — an NFS mount with no matching users — is shown
as the number, which is more useful than a blank and is what `ls` does.

The detailed view draws FAR's own row — name, size, when it was written, when it was made,
when it was last read, attributes — and drops the two extra dates, the last one first, as soon
as keeping them would leave the name unreadable: a half-width panel therefore shows what the
full view shows, with the attributes after it. A filesystem that does not keep one of the
dates leaves that column blank rather than inventing an epoch. The allocated size, FAR's
seventh column, is a backlog line: it needs a platform call on Windows.

### 3a. The user's own column sets

FAR has no separate keys for custom modes: its ten modes **are** the editable ones, each one a
list of columns the user may rewrite. Ours work the same way. A `[[column_set]]` block whose
`id` is a view's own digit is what that view draws, and the built-in list above is the default
for a slot nobody has written — so `id = 3` is what `Ctrl+3` shows, and `id = 0` is `Ctrl+0`.
(`Ctrl+Shift+1..0` is *not* where these live: those are the folder shortcuts, in FAR and
here.)

```toml
[[column_set]]
id = 3
columns = [
  { type = "name",  width = "*" },
  { type = "size",  width = 10, align = "right", fmt = "bytes" },
  { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
]
```

Column types: `name`, `ext`, `size`, `allocated`, `mtime`, `btime`, `atime`, `attrs`, `owner`,
`group`, `links`, `descr` — the same fields the built-in modes draw, and a set that asks for
one of the four a listing does not carry (the owner, the group, the link count, the
description) puts the same worker to work as the view that shows it, with a row the answer has
not reached yet drawn blank.

Column widths: a number, a percentage of the panel, `*` to take what is left, or `0` for the
column's own natural width — FAR's `0`, which for a name or a description means "what is left
over" because neither has a width of its own. One blank column between each pair, as the
built-in modes have. A column the panel has no room for is dropped from the right, and a set
whose name column would be left unreadable drops another, which is the rule the detailed view
already follows with its extra dates. `align` is `left` or `right` where the column's own
default is not wanted; `fmt` is `human` or `bytes` on a size (`1.18 M` against `1 234 567`)
and a `strftime` string on a date.

A set may also be written in **FAR's own pair of strings** instead of the list, for a mode
somebody already has in their FAR configuration (§3b):

```toml
[[column_set]]
id = 4
far_types  = "N,SC,DM"
far_widths = "0,9,16"
```

The two spellings are the same thing, so a block that writes both is a block whose author has
not decided: the strings win and the block says so on the startup screen. What we have no
column for — the stream size, the change time, the stream count — is reported there too rather
than dropped in silence, and a set left with no column that reads at all leaves the built-in
mode in place, because a panel that drew nothing would be a worse answer than a panel that
looks as it always did.

A column that costs a lookup per file is fetched by the same worker the built-in view uses, so
`owner`, `group`, `links` and `descr` in a set of one's own behave exactly as `Ctrl+8`,
`Ctrl+9` and `Ctrl+6` do — and a view whose set does *not* ask for them pays nothing, even
when the mode it replaced would have.

Horizontal scrolling for long names (`Ctrl+Shift+←/→`, because `Alt+←/→` is the folder history
here), ellipsis in the middle for paths and at the end for names.

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
(`{ type = "mtime", fmt = "…" }`), and the mapping is one to one for everything we support —
**the precision suffix included**: FAR counts precision where we spell a format, so `T` with
`+1` arrives as `%I:%M %p`, with `+2` as `%H:%M:%S` and with `+3` as `%H:%M:%S%.3f`, a `DM`
keeps its date and lengthens only the clock, and `+2` on a date on its own is already true of
ours and is passed over in silence. A precision on a column that has none — a size, a name —
is reported;
what we do not support (allocated size, streams, descriptions before they exist) is reported as
a warning on import, never dropped silently.

Per-mode flags from the same dialog, written beside the columns in the block they belong to:

```toml
[[column_set]]
id = 4
columns = [{ type = "name", width = "*" }, { type = "size", width = 10 }]
fullscreen             = true   # the panel takes the screen while this mode is on
align_extensions       = true   # a file's extension down the right of the name field
align_folder_extensions = false # and a directory's, which is off: its dot is part of its name
folders_upper          = false  # directory names in capitals
files_lower            = false  # file names in small letters
upper_to_lower         = false  # a name that is *all* capitals drawn in small letters
```

Each is what the panel drew before the flags existed when the block does not say: the
extensions of files lined up, and nothing else. `fullscreen` is `Ctrl+F1` applied by the mode
rather than by a key, so choosing the mode hides the other panel and leaving it gives the
screen back — but a panel hidden by the key itself is left alone, because a mode undoing
somebody's own keypress would be the mode reaching outside itself. `upper_to_lower` is FAR's
switch for the 8.3 names a FAT volume hands back, which are all capitals and were never meant
to be read that way; a name with no letters in it at all is not "all capitals", so `README`
changes and `2026` does not.

### 3c. The size of a folder

A directory's own size is four kilobytes of bookkeeping on most filesystems, so the panel
writes the word `Folder` in the size column rather than that figure. `Space` measures what one
actually holds — and with a selection, **every selected folder**, because the question behind
the key is which of several is the big one.

What comes back becomes that directory's size, with a flag saying it was measured. Everything
that reads a size then reads this one: the column shows the figure in place of the word, the
selected-totals bar adds it up (`Bytes: 69 082 073, files: 0, folders: 2`), and sorting by
size puts the measured tree where it belongs. A re-read drops it, because the figure was about
the directory as it was.

## 4. Sorting

Keys: name, ext, size, mtime, ctime, atime, btime, attrs, owner, descr, packed, type, none
(directory order). Modifiers: reverse, directories first (default on), selected first,
case-insensitive (default on Windows/macOS), **natural/version order** for digits, locale
collation via ICU-less `unicode-collation`-style folding. A **criteria chain** breaks ties
(FAR's idea): `panel.sort_chain = ["size", "name"]`.

**Shipped in 0.7.2.** The chain is up to three keys, asked in order when the sort key leaves
two rows equal, and the *name* has the last word after them whatever was asked — so the
comparison is total and the order does not wander between two reads of the same directory. A
key the chain repeats is skipped, since asking it twice cannot say anything new, and `reverse`
turns the whole comparison round rather than only its first step. Three slots because `Sort`
is `Copy` — every panel, tab and saved session holds one — and because a fourth tie-breaker
after three that all came out equal has nothing left to decide. A word that is not a sort key
is said on the startup screen rather than quietly sorting by name.

Pressing the same sort key twice toggles direction when `panel.allow_reverse_sort = true`; so
does choosing the current order again in the sort list (SPEC 10 §4.5).

**The two keys whose field is not in the listing** — the description (`Ctrl+F10`) and the owner
(`Ctrl+F11`), FAR's own two keys for them — work the way the views that show those fields work.
Neither is in an `Entry`, because both cost something to find out: the description comes out of
the directory's own `descript.ion` and the owner out of a lookup per file, and an `Entry` is a
thing a million of exist. So pressing the key asks a worker for the field, exactly as choosing
the view does, the panel sorts without it in the meantime, and the answer sorts it again —
keeping the cursor on the file it was on, because a row is a different file once the order has
changed. A file nothing has said anything about sorts **after** every file that has: a
directory of a hundred where three are described reads as those three and then the rest, which
is what the key is for. The lookup happens once per rebuild rather than inside the comparison,
which is asked some thirty-five million times on a panel of a million rows.

**Sort groups** (FAR's `Shift+F11`, and `Group` is what FAR writes above that key) sit on top
of the sort: an ordered list of named mask groups, some pulling their files ahead of everything
else and some pushing them behind, with the boundary between "front" and "back" explicit in the
list. Within a group the normal sort applies. This is what makes `*.cpp` files appear before
`*.o` ones however the panel is sorted.

**Shipped in 0.7.2.** The bands are `[[sort_group]]` blocks in the configuration — a `name`, a
`mask` in the filter's own grammar, and `place = "front"` (the default) or `"back"` — read in
the order they are written: the front ones keep that order among themselves, so do the ones
behind, and the files that match nothing sit between them. Writing the blocks turns them on;
`Shift+F11` turns them off and on again per panel, and says how many there are, because a
toggle over an empty list is otherwise a key that does nothing. **The bands are edited from
the program too**, at `F9 → Options → Sort groups` and in the shape SPEC 10 §3.11 describes
for the colour groups: the list in the order it cuts the listing, `Ins` beside the row the
cursor is on, `Del`, `F4`, `Ctrl+↑↓` to move one past its neighbour. A change is in force on
both panels at once — writing the first band turns the grouping on and taking the last one
away turns it off — and *Save setup* writes the blocks.

Two things the bands deliberately do *not* touch. **Directories are not grouped**: they have
their place in the order already, and a folder that happened to match `*.o` landing in the
middle of the object files is not what the mask was written for. And **`reverse` does not move
a band**: it turns the order round inside each one, because a band that swapped ends with it
would make the list mean the opposite of what it says.

### 4b. Selecting with a mask, and whether folders count

The keys that name a mask — `+` and `-`, and the two that take the name under the cursor apart
— leave directories alone unless `panel.select_folders` says otherwise, which is FAR's
*Select folders* and FAR's default. One types `*.rs` to select the sources, not the
directories whose names happen to end that way.

*Select all* and *invert all* include directories whatever the setting says: they name no
mask, so they cannot be meaning files alone.

A mask in a **filter** or in a **highlight group** applies to directories regardless, and that
is deliberate rather than an oversight: both carry a *kind* condition of their own — files
only, directories only, either — so there one says it instead of inferring it. The selection
keys have no such condition, which is why they need a default at all.

## 5. Filters

- **Quick filter bar** (`Ctrl+Shift+I`): live substring/glob, applied to the visible order
  only. It was `Alt+I` and moved: `Alt+<letter>` is the quick search, and a binding always
  wins over it, so that one key cost the letter `i` in every listing.
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
- **Highlight groups** colour rows with the same condition set, which is the point of having
  one: what a filter can say, a group can say.

```toml
[[highlight]]
name = "Rust"
mask = "*.rs"
fg = "#d7875f"

[[highlight]]
name = "Fresh"
age = "<=1"            # days, as the filter dialog spells it
date_field = "create"  # write (the default), create, access, change
fg = "file.archive"    # a theme key, so the group follows the theme
bg = "idx:235"

[[highlight]]
name = "Big and hidden"
size = "1G-"           # 1M-2G, -4K, 1G-; a lone 1M is a lower bound
attrs = "h-s"          # hidden, and not system
fg = "red"
```

**Shipped**: the groups above, asked in the order they are written — the first that matches a
row decides — with FAR's own six after them, so a file nothing is said about looks as it
always did. A group with no condition is refused (it would paint every row) and a group that
does not parse is a warning on the startup screen and is left out, because a colour is not
worth refusing to start over. The ink is a colour (`#d7875f`, `red`, `idx:235`) or the name of
a theme key, and a key is resolved every time it is drawn rather than once, so a group follows
`Ctrl+Shift+T` from one theme to the next.

The dialog that edits these is `F9 → Options → Files highlighting` (SPEC 10 §3.11): the
groups as a list in the order they are tried, each row drawn in its own colours, with a dialog
per group that takes exactly the words the file takes. A predicate language for the conditions
that are not conditions on one field (`attrs.contains('x') && kind == 'file'`) is still a
backlog line: the condition set covers what a person writes in a file.

### 5b. The named mask groups (`[[mask_group]]`)

Every field in this program that takes a mask takes the same mask list, and a mask list may
name a group: `<arc>` is every archive, `<temp>` every file somebody meant to throw away,
`<exec>` everything that runs. Three come with the program and are not written down anywhere;
a `[[mask_group]]` block adds one, and a block with the name of a built-in one replaces it —
which is how somebody who keeps `.cbz` files teaches `<arc>` about them.

```toml
[[mask_group]]
name = "images"
mask = "*.png,*.jpg,*.jpeg,*.gif,*.webp,*.svg"
```

Groups nest: a group may name another. A group that would name itself, directly or round a
circle, is refused where it is written rather than where it is used.

**The editor** is `F9 → Options → File masks`: the groups as a list, each row showing what it
stands for, with a `·` on the three that are built in. `Ins` writes a new one, `F4` and
`Enter` edit the row — including a built-in one, which then becomes a block that replaces it —
and `Del` takes a block away, saying so rather than pretending when the row is one of the
three nobody has rewritten. A name with a space, a comma or an angle bracket in it is refused,
because it could never be written back between brackets; so is a group that stands for
nothing.

Everything that was compiled from a mask is compiled again the moment a group changes: the
colour groups, the sort bands and each panel's own filter. *Save setup* writes the blocks.

## 6. Fast find / quick search

Typing while the command line is empty starts fast find (configurable: FAR uses `Alt`+letters,
mc uses bare letters). Matching: prefix, then substring, then fuzzy, with `*`/`?` wildcards
accepted literally. `Ctrl+Enter` / `Ctrl+Shift+Enter` cycle matches forward/backward. The match
is highlighted inside the name. `Esc` restores the previous cursor position.

**Where the box is drawn.** In the panel it is searching, standing on that panel's own bottom
frame, two columns in from its left edge — never in the middle of the screen. The search is
about the rows, so the box must not cover the ones being read; the rows it does cover are the
last of the list, which are the ones already walked past. It is FAR's own placement and FAR's
own reason. A panel too narrow or too short to hold the box falls back to the middle of the
screen, which is at least somewhere it fits.

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

**The tree's cache survives the session.** What each directory held is written into the state
file (capped at twenty thousand directories) and read back at startup, so `Ctrl+T` opens
filled rather than reading its way down again — and so the folder list below has something to
show the first time it is opened. It is a cache and says so: a row that is out of date costs a
re-read and nothing else.

**`Alt+F10` is that cache as a list one types into.** Every folder known under this panel's
root, narrowed by what is typed — the frame shows it, because three rows out of four thousand
need a reason — with `Enter` to go there. Opening it also starts one bounded walk that fills
the cache in behind the dialog, breadth first, so a walk stopped by its limit has read the
directories near the top, which are the ones people look for; the list refreshes when it ends
and says how many folders it found. A second `Alt+F10` does not start a second walk. The copy
dialog's own `Alt+F10` opens the same list and puts the row chosen into the destination —
FAR's *Tree* button, under another key because `F10` here is `OK`. A root that is not this
machine is refused with its reason: a tree of a server is a listing per directory over the
wire, and the folder history and the hotlist are what answer that question there.

`Ctrl+L`'s divergence is settled with this: it is FAR's info panel, and our
change-directory dialog moved to `Ctrl+Shift+L`, which it can afford now that `cd` works on
the command line.

**The quick view is the viewer, in a panel's frame.** It is not a preview of its own: it is a
real reader with its chrome turned off, so everything the viewer does it does. While the files
have the focus it only follows the cursor, as FAR's does. Give it the focus with `Tab` and its
own keys are the viewer's: `F2` wraps, `F4` walks text → dump → hex → document → table,
`Shift+F4` is hex and back, `F5` disassembles, `F6` decompiles, `F7` searches, `F8` changes
the code page, `F11` opens the structure pane, and the key bar says so. The ones that would
leave — closing a file it did not open, going to the next file, crossing to the editor — stay
with the panel, and so does every letter: `n` and `/` belong to the command line, whatever the
viewer binds them to.

**A row that is not a file previews its card.** Beside a process list, a socket list or the
list of installed programs, the quick view holds the same page `F3` opens over there (§7b),
built when the cursor lands on the row. The card is longer than the panel, which is exactly
why it is a reader: it scrolls.

## 8. Tabs (shipped)

A tab **is** a panel. There is no tab object holding a panel beside a name and a lock,
because everything a tab has to remember — where it is, its view and sort, its filter, what
is selected, where the cursor is, where it has been, what it is called, whether it is
locked — is already what a panel remembers. So one side's tabs are a list of panels with one
of them on screen, and `crates/fcmd/src/tabs.rs` is that list.

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

Tabs are **persisted** in the state file (`fcmd-state.toml`), with their names and their
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

They are kept in `fcmd-state.toml` rather than in `fcmd.toml`, because a *key* sets them: a
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
| free space on the device | bottom frame, right | `panel.free_space` | shipped on all three platforms through `fcmd-platform` (`statvfs`, `GetDiskFreeSpaceEx`); a server has none to report and the frame says nothing rather than guessing |
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
| wheel | scrolls what is **under the pointer**, without taking the focus: looking at the other panel is not working in it. How many rows a notch moves is a setting per area, as it is in FAR — `ui.wheel_lines` for the panels and the lists, `ui.wheel_lines_viewer`, `ui.wheel_lines_editor` — three by default and never more than twenty |
| sideways wheel | scrolls what has anything to scroll sideways, by as many columns as the wheel moves rows: a long line in the viewer, a long line in the editor. A panel has nothing of the kind — its columns are a layout, not a window onto something wider — so there it does nothing rather than moving the cursor about |
| left click on the key bar | presses the key written in the cell, with whatever modifiers are held — so `Shift` and a click is `Shift+F5` |
| left click on the menu bar | opens that menu; one click on a dropdown row runs it |
| left click on a dialog's list | moves its cursor; a double click answers the dialog |
| left click outside an open dialog | nothing at all, and in particular not the panel underneath: a click must not move a cursor the user cannot see |
| left click in the editor | puts the caret there |

Every one of them ends up in the same `run` the keyboard uses, so the macro recorder, the
confirmations and the error reporting are the ones already there — the mouse does what a key
does rather than having powers of its own.

Two of the three drags are there as well. Which one a drag *is* was decided when the button
went down, so that one gesture cannot turn into the other halfway:

| gesture | what it does |
| --- | --- |
| drag from a row | selects every row it sweeps, in the panel the press began in, and leaves the cursor where it stopped. It only ever selects: a sweep that overshoots and comes back leaves what it passed over selected, which is the right way round for a gesture nobody aims precisely — the right button takes one row out again and `Gray *` turns the lot round. A drag that wanders into the other panel selects nothing there, and carries on where it left off if it comes back |
| drag the frame where the panels meet | moves the rule between them, within the same limits `Ctrl+←` and `Ctrl+→` keep, so the two ways of moving it cannot disagree about how narrow a panel may get. The press alone does nothing: a click on a frame never did, and a press that moved the split would be a trap |
| drop on the other panel | opens the **copy dialog**, with the destination the pointer chose: the directory under it, or that panel's own when the row is a file — a file is not somewhere to copy into. What it acts on is what the sweep on the way over selected, or the row the press landed on, so it is `F5` with the destination filled in, down to the error when there is nothing to copy |

A drop copies nothing by itself, and that is the decision rather than a gap. The gesture is
imprecise by nature and starts by accident, so what answers for it is the dialog that was
already there — the conflict policy, the filter, what travels with the file — and the way out
is the `Esc` every dialog has. In the Explorer a drag within one drive moves and across drives
copies: the same gesture doing two different things depending on where the files are, one of
which deletes the original. That rule has surprised people for thirty years; FAR asks instead,
and so does this.

## 13. Descriptions (`descript.ion`, shipped in 0.7.3)

A 4DOS habit that FAR kept and nothing else has: a plain file in the directory, one line per
name, holding what a file *is* where the name has no room to say it.

```text
readme.md          what to read first
"two words.txt"    a name with a space in it is quoted
```

The whole format is a name and the rest of the line. A name with a space is quoted; a line with
nothing after the name describes nothing and is dropped, because an empty description and no
description are the same thing. A description with more than one line in it is held on one line
with FAR's own two-byte break (`04 C2`), and a panel shows it on the single line a row has. The
file is read in the code page it is in — one written under DOS is in a DOS code page, one
written today is UTF-8 — and written back in UTF-8. Both of FAR's names are read,
`descript.ion` first and then `files.bbs`; the first is what is written.

Two settings of FAR's say otherwise, for a tree somebody already keeps:

```toml
[panel]
description_names = ["files.bbs", "descript.ion"]   # which, and in which order
description_utf8  = false                           # keep the page it was read in
```

`description_names` is the list looked for, in order, and its first entry is what a new file
is written under; an empty list is the two above. `description_utf8 = false` writes the file
back **in the code page it was read in** — there is nothing else to go on, and guessing the
machine's own page would turn a Russian `descript.ion` into a Latin one on a Latin machine.
A file that is not there yet, and one that was already UTF-8 or UTF-16, is written in UTF-8
either way. So is one whose page cannot hold what was typed: an encoder writes a `?` for what
it has no room for, and a file a DOS tool cannot read is better than a line that lost its
words.

**Where it shows.** Two view modes, on FAR's own numbers: `Ctrl+6` is name and description,
`Ctrl+7` is name, size, time and description. The name keeps two fifths of the row and the
description takes the rest, with a floor under the name — a description in eight columns is no
use, and a name that cannot be read is no use at all. The information panel (`Ctrl+L`) spells
out the description of the row under the cursor, and the panel's own status line shows it
beside the size and the time wherever the panel has read the file.

**When it is read.** Only when something on screen shows one: either description view, or an
information panel beside the panel. A side file nobody is looking at is one round trip per
directory on a network share, and the two columns are not worth that to somebody who keeps no
descriptions. `panel.descriptions = false` turns the whole habit off. Wherever the panel
stands: on this disk, inside a container — where the index is already in hand and the lookup
costs nothing — and on a server or in a bucket, where it is one small file fetched whole. A
list panel reads none, because its rows come from several directories at once.

**Writing one.** `Ctrl+Z` opens the dialog on what the directory already says about the file
under the cursor, so changing a description is a key and a few characters rather than retyping
it. An empty answer takes the line away — there is no second key for removing one. The panel
shows the new text at once and the file is written behind it; a directory left with nothing to
say loses the file rather than keeping an empty one. The file goes where the panel stands, a
server and a bucket included. Inside a container it is refused: the side file is an entry like
any other and writing one is a rebuild of the whole archive, which is a job and not a
keystroke.

**Carrying one.** A description follows what it describes: a copy leaves the line where it was
and writes one at the other end, a move takes it with it, a rename takes it to the new name,
and a delete takes it away — a description that outlived its file would land on the next file
to be given that name. Only the sources the user chose are carried, because a directory taken
whole carries its own `descript.ion` along with everything inside it; what needs carrying is
the line about the directory itself, which lives in its parent's file. It happens when the job
ends and only if the job really did something: a cancelled copy must not leave a description
pointing at a file that never arrived. A job whose sources came from several directories at
once — a list gathered from a search — carries each line out of the directory it was in: the
sources are grouped by their parent and each side file is read, changed and written on its
own. A container at either end carries nothing, for the reason *Writing one* gives.

**What is still owed**: writing one inside an archive, and the settings FAR has for which
names to look for and whether to write UTF-8.

## 14. The panels that are not drives (shipped in 0.8.0)

`Alt+F1` and `Alt+F2` offer more than drives, as FAR's change-drive dialog does: under the
mounted filesystems and the open connections come three panels that are not filesystems at
all, each numbered so a key reaches it.

**NetBox — the stored sessions.** Every `[[site]]` and `[[s3]]` block, plus whatever is open
now, as a panel of rows named `sftp://user@host`. A panel and not a dialog, because that is
what makes it useful: `Enter` on a row connects and the panel becomes that server's directory,
and the quick search, the keys and the other panel beside it work the way they work on files.
A session that is already open is the row worth having, so it hides the stored one it came
from.

**Process list.** What is running, as a panel: the name with its pid beside it, and the *size*
column holding the memory it has resident — which means `Ctrl+F6` sorts the list by memory,
which is what anybody opening it came to see. Linux reads `/proc/<pid>/status`, which answers
the name, the thread count and the resident memory in one file and needs no call into the
kernel; Windows takes a `CreateToolhelp32Snapshot` and asks each process for its working set,
and one that refuses keeps its row with the column empty. macOS and the BSDs keep the list
behind a `sysctl` whose structure would have to be mirrored by hand, field for field, per
release — a page of `unsafe` for a panel, where a wrong offset reads somebody else's memory —
so it waits for a tested way to do it.

`F3` on a row is not a file to read: it is the card for that process, asked for afresh when the
key is pressed, because the memory it had when the panel was built is not the memory it has
now. **The card is a page and is shown in the viewer**, over text the program made rather than
a file it opened: the environment and the list of loaded libraries are each longer than any
screen, and the viewer already scrolls, searches and wraps. Its frame says *Process: name
[pid]* rather than a path, so nothing claims a file was opened.

It is laid out in three blocks and two lists, in the order FAR's own process plugin writes
them. What it is: the name, the pid, what started it, whose it is and which group, the SID
where the system keeps one, the session, what it is doing, its priority, the thread count,
32 or 64 bits, the version and description the executable carries where it carries them, when
it started and how long it has been up. What it has taken and spent: resident, private,
mapped and swapped-out memory, page faults, processor time in its own code and in the kernel,
how many handles it holds, and how much it has read and written. Where it is: the executable,
the working directory and the whole command line. Then its environment, in the order the
system keeps it, and every library it has loaded. Every row only where the platform and this
user's rights answer it, and no row that they will not — a card with blanks on it would say
the program failed where the system simply refused.

Linux has all of it in files anyone may read: `status` for the names and the memory, `stat`
for the session, the priority, the faults and the two processor times (in clock ticks, so
`sysconf(_SC_CLK_TCK)` is the one number that is not in the file), `io` for the counters,
`fd/` counted for the handles, `environ` and `maps` for the two lists, and the first bytes of
the executable's own ELF header for the bitness rather than a call. `io` and `environ` are
readable only by the process's owner, so somebody else's process has those rows missing
rather than wrong.

Windows asks for it a call at a time, each left out when it is refused: the snapshot for the
name, the parent and the threads, `GetProcessMemoryInfo` with the extended counters for the
memory and the faults, `GetProcessTimes` for the start and the two processor times,
`GetProcessHandleCount`, `GetProcessIoCounters`, `GetPriorityClass`, `ProcessIdToSessionId`,
the process token with `LookupAccountSidW` for the owner and the SID, and
`EnumProcessModules` — through a second handle, because reading another process's module list
wants rights the rest of the card does not — for the libraries. The command line, the working
directory and the environment it keeps inside the process itself, behind
`NtQueryInformationProcess`, which is not a documented structure to depend on: those rows are
left out there rather than paid for with a page of `unsafe` a service pack can invalidate.

**Nothing here kills anything**: a file manager that
could end a process from a listing with the key that deletes a file is one that ends a process
by accident, and ending one wants a confirmation of its own.

**Service list (§7d).** What the machine runs in the background, as a panel: the name, the
state in one word, and what the service says about itself. `F3` is its card, and the three
keys that matter are on the bar — `F5` starts, `F6` restarts, `F8` stops — because a file
manager is where one already is when a service has to be looked at, with its log in one panel
and its configuration file in the other. Stopping and restarting ask first; starting does not,
because nothing is taken away by it.

Each platform is asked the way its own tools ask. Linux has systemd, and systemd answers
`systemctl`: the list is `systemctl list-units --type=service --all` in its machine-readable
form, and the three actions are `systemctl start|stop|restart`. There is no D-Bus dependency
here, for the reason there is no GUI toolkit. Windows has the Service Control Manager, which
is an API and is called as one — `EnumServicesStatusExW` for the list, `StartServiceW` and
`ControlService` for the actions, and a restart is a stop, a wait for it to be down and a
start, because the manager has no single call for it. macOS has `launchctl`, whose `list` is
three columns and whose `kickstart -k` is a restart.

**Nothing here runs as anybody else.** Stopping a system service wants root, and this program
does not reach for `sudo`, `pkexec` or any other way of asking: it runs as the user running it
and, when the system refuses, says *this needs root — the program will not ask for it on your
behalf* rather than appearing to have worked. A file manager that silently escalated would be
one nobody could audit. The action goes through a worker, because a manager takes seconds over
a restart and the panels must keep answering; when it comes back the list is read again, so
the panel shows the state the service is in now rather than the one it was in.

A service name is checked before it goes anywhere near a command line — letters, digits and a
handful of punctuation — so that a name with a semicolon in it is refused here rather than
interpreted there.

**Socket list.** What is listening and what is connected, as a panel: the protocol, this end,
the other end, the state, and the process holding it — in columns, as the drive dialog is. The
other end is left empty for a socket that is only listening, because `0.0.0.0:0` says nothing.
`F3` on a row opens the card for the process that holds it (§7b), which is the question a socket
list is opened to ask; a socket this user may not be told the owner of says so rather than
showing a card about nothing.

Linux reads `/proc/net/tcp`, `tcp6`, `udp` and `udp6` and then maps each row's inode back to a
process by reading every process's open files once — a socket shows up as `socket:[12345]` among
them — so a process somebody else owns keeps its rows and loses only its name. Windows answers
this better than it answers anything else about another process: `GetExtendedTcpTable` and
`GetExtendedUdpTable` give the whole table *with* the owning pid, one call per protocol and
nothing to be refused. macOS and the BSDs wait for the same tested way to read a `sysctl` as the
process list does. **Nothing here closes a socket**, for the reason the process panel kills
nothing.

**Installed programs.** What is installed, as a panel, with the *size* column holding what each
program takes on disk — so `Ctrl+F6` sorts the list by size, which is the question that makes
anybody open such a list at all. FAR has this as a plugin (`UnInstall`); the list is somewhere
the system keeps it and nobody remembers where.

Windows reads the three registry keys the Control Panel's own list reads — the machine's, the
machine's 32-bit view and this user's — and hides what the Control Panel hides: an entry with no
display name, one marked as a system component, one whose release type says it is an update.
Linux reads whichever package database is a plain file: dpkg's `status`, or pacman's directory
of descriptions. An rpm database lives behind `librpm` and waits.

`F3` on a row is the program's card: name, version, publisher, size, when it was installed,
where it lives, and the command that would remove it — which is the one thing in the list that
cannot be found any other way. `F4` puts that command on the **command line, unrun**, for the
person at the keyboard to read before pressing `Enter`. **Nothing here uninstalls anything by
itself**: a list that removes a program on a keypress is a list one removes a program from by
accident.

**Temporary panel.** A panel that holds paths rather than a directory, so what is in it can
come from anywhere — several directories, two drives, a search — and `F5` out of it copies the
real files. It opens empty from the dialog; `Ctrl+Shift+Ins` puts what is selected in the
other panel into it, adding to what is there and never adding the same path twice. The rows
are whole paths, because that is the only thing that tells two files of one name apart.

**Network** is the one of FAR's four this does not have: browsing a Windows network needs SMB
discovery, which is a protocol and not a panel, and a line in `docs/BACKLOG.md`. The sessions
panel is where a server one has already named lives.
