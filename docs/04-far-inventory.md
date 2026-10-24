# FAR 3.x, feature by feature, read from the source

`docs/02-features.md` is the union of many commanders, written from documentation and from use.
This file is narrower and harder: it is what **FAR Manager 3.x actually does**, read from its own
repository (`FarGroup/FarManager`, BSD-3-Clause, HEAD `60567ba`, fetched 2026-10-24), and compared
line by line with what SupremeCommander 0.0.2 does.

Sources used, in order of authority:

| Source | What it gives |
| --- | --- |
| `far/FarEng.hlf.m4` | the English help: 283 topics, every dialog, every key, every option |
| `far/config.cpp` (`RoamingData` / `LocalData` tables) | **311 settings keys** in 33 sections — the real configuration surface |
| `far/farlang.templ.m4` | every UI string in 13 languages: the exact menu and dialog wording |
| `far/Far.exe.example.ini` | the launcher-level settings (profile location, portability, read-only config) |
| `far/changelog` | 66 662 lines of history: when and why a behaviour is the way it is |
| `plugins/*` | what FAR ships as plugins rather than as core |

Status column: ✅ in scmd 0.0.2 · ◐ partly · ✗ missing · ⊘ Windows-specific, needs a different
answer on three platforms (not a straight port).

> Method note: the help text is FAR's expression and stays FAR's. What is taken here are facts —
> which features exist, which keys they use, which values an option accepts — described in our
> own words. Where a format has to be compatible to be useful (file masks, column types,
> metasymbols) the grammar is recorded, because a grammar is an interface, not prose.

## 1. The numbers

FAR 3.x is bigger than it looks from the screen:

- 283 help topics, of which 23 are settings dialogs and 41 are `far:config` parameter pages.
- 311 configuration keys. The largest sections: `System` 57, `Editor` 34, `Interface` 26,
  `Viewer` 23, `Panel.Tree` 17, `Panel` 16, `Confirmations` 15.
- 10 built-in panel view modes, each one fully re-definable through a column grammar.
- 14 independent macro areas.
- 15 confirmations, each separately switchable.
- 20 bundled plugins, including the archive layer and the FTP client — in FAR these are *not*
  core, which is why FAR's core looks smaller than it is.

For comparison, scmd 0.0.2 has ~45 configuration keys, 4 view modes, no macros, 3 confirmations
and 3 built-in providers. That gap is the roadmap.

## 2. Panels

| Feature | Keys | Status |
| --- | --- | --- |
| Two panels, active/passive | `Tab`, `Ctrl+U` swap | ✅ |
| Re-read | `Ctrl+R` | ✅ |
| Info panel | `Ctrl+L` | ✗ |
| Quick view panel | `Ctrl+Q` | ✗ |
| Tree panel, with an on-disk tree cache per drive | `Ctrl+T` | ✗ |
| Hide both panels / hide passive / hide left / hide right | `Ctrl+O`, `Ctrl+P`, `Ctrl+F1`, `Ctrl+F2` | ✗ |
| Temporarily uncover the screen while keys are held | `Ctrl+Alt+Shift` | ✗ |
| Resize panels: width, height, per-panel height, and reset | `Ctrl+←/→`, `Ctrl+↑/↓`, `Ctrl+Shift+↑/↓`, `Ctrl+Numpad5`, `Ctrl+Alt+Numpad5` | ✗ |
| Key bar on/off | `Ctrl+B` | ◐ config only (`ui.key_bar`) |
| Total/free size display mode | `Ctrl+Shift+S` | ✗ |
| 10 view modes: brief, medium, full, wide, detailed, descriptions, long descriptions, owners, links, alternative full | `LeftCtrl+1…0` | ◐ 4 of them (brief, full, detail, wide) |
| Hidden and system files | `Ctrl+H` | ✅ |
| Long/short (8.3) names | `Ctrl+N` | ⊘ |
| Sort by name, extension, write time, size, unsorted, creation time, access time, description, owner | `Ctrl+F3…F11` | ◐ 5 of 9 (no ctime/atime/description/owner) |
| Sort modes list, with `+`/`-`/`*` to set the direction and `F4` to fine-tune | `Ctrl+F12` | ✅ (list, and re-choosing reverses) |
| **Sort criteria chain**: per sort mode, an ordered set of tie-breakers, edited with `Ins`/`Del`/`F4`/`+`/`-`/`*`/`=`/`Ctrl+↑↓`/`Ctrl+R` | — | ✗ |
| Sort groups: named mask groups that sort before or after everything else | `Shift+F11` | ✗ |
| Selected files first | `Shift+F12` | ✅ (setting) |
| Folder shortcuts: 10 slots, set and jump | `Ctrl+Shift+0…9`, `RightCtrl+0…9` | ✗ |
| Scroll long names and descriptions sideways | `Alt+←/→`, `Alt+Home/End` | ✗ |
| Column titles, status line, file totals, free size, scrollbar, sort-mode letter, `..` in root, background-screen count — each a setting | — | ◐ column titles, the sort letter and the filter mark, file totals yes (`panel.column_titles`); free size, the per-panel status line, the scrollbar, `..` in root and the screen count no |
| Automatic update on filesystem change, with a size threshold that turns it off, and a separate switch for network drives | — | ✗ (no watch yet) |
| Detect volume mount points, distinguish junctions | — | ⊘ |
| Drag and drop between panels and with the OS | — | ⊘ |

**The column grammar** (same one for panel modes, their status lines, and find results) is the
piece worth copying exactly, because every user's saved layout is written in it:

```
N[M[D],O,R[F],N]  name    M selection marks, D dynamic, O no path, R right-align, F always, N no extension
X[R]              extension, R right-aligned
S / P / G [C,T,F,E]  size / allocated size / stream size   C group digits, T decimal units,
                                                           F fractional (1.00 K), E no space before unit
D  T              write date, write time
DM / DC / DA / DE [B,M]  write / creation / access / change date+time   B Unix-style, M month names
A                 attributes      Z  description     O[L] owner (L with domain)
LN                hard link count F  stream count
```

Widths are absolute, `0` for automatic, or a percentage of what is left; `+1` on a time column
switches to 12-hour, more adds seconds and milliseconds, `+2` on a date column gives a 4-digit
year. Repeating a group of columns turns the panel into newspaper-style stripes. Per mode there
are also: fullscreen, align file extensions, align folder extensions, folders uppercase, files
lowercase, uppercase names shown lowercase.

## 3. Selection

FAR has a selection *matrix*, not a selection key: the modifier picks the criterion.

| | select | deselect | invert |
| --- | --- | --- | --- |
| by mask | `Gray +` | `Gray -` | `Gray *` |
| same extension as the cursor | `Ctrl+Gray +` | `Ctrl+Gray -` | `Ctrl+Gray *` (incl. folders) |
| same name as the cursor | `Alt+Gray +` | `Alt+Gray -` | `Alt+Gray *` (files only, folders cleared) |
| everything | `Shift+Gray +` | `Shift+Gray -` | — |

Plus `Ins` and `Shift+arrows` (the action follows the state of the item you started on),
`Ctrl+M` to restore the previous selection, right-click and right-drag selection, and a
`Select folders` setting that decides whether the mask keys touch directories at all.

scmd: ✅ `Ins`, mask `+`/`-`/`*`, select all/none/invert, restore. ✗ the extension and name
variants, the folder rules, mouse selection.

## 4. File operations

| Feature | Keys | Status |
| --- | --- | --- |
| Copy, move/rename, make folder, delete | `F5`, `F6`, `F7`, `F8` | ✅ |
| Same four, but on the item under the cursor, ignoring the selection | `Shift+F5`, `Shift+F6`, `Shift+F8` | ✗ |
| Delete skipping the recycle bin | `Shift+Del` | ⊘ (we always delete) |
| Wipe: overwrite, truncate, rename, unlink | `Alt+Del` | ✗ |
| Create hard links and symlinks | `Alt+F6` | ✗ |
| Several destinations in one copy (`;`-separated, quoted if needed) | — | ✗ |
| Existing-file policy chosen up front: ask, overwrite, skip, **append**, only newer, plus "also ask for read-only" | — | ◐ ask/overwrite/skip/rename/all, no append, no only-newer-as-default |
| Access rights for new files: default, copy, inherit | — | ⊘ |
| Copy the contents of symbolic links instead of the links | — | ✗ |
| Copy files other programs hold open for writing | — | ⊘ |
| A **filter** applied to the copy, so only matching files travel | — | ✗ |
| Destination picked from a folder tree, prefilled from the active panel, the passive one, or the typed path | `F10`, `Alt+F10`, `Shift+F10` | ◐ prefilled from the passive panel only |
| Total progress bar, and average speed / elapsed / ETA | — | ◐ bars, no speed or ETA |
| File attributes dialog: the full NTFS attribute set, four timestamps, owner, hard links, streams | `Ctrl+A`, `F4` on a folder | ✗ |
| Apply a command line to each selected file | `Ctrl+G` | ✗ |
| Describe files (`descript.ion` / `files.bbs`) | `Ctrl+Z` | ✗ |
| Compare folders | Commands menu | ✗ |
| Copy names, full names, UNC names, or the files themselves, to the clipboard | `Ctrl+Ins`, `Alt+Shift+Ins`, `Ctrl+Alt+Ins`, `Ctrl+Shift+C/X` | ✗ |

## 5. Masks, filters, groups, highlighting

The mask language is small and worth matching exactly:

- `*` and `?` as usual, `[c-f]` character sets and ranges.
- Several masks separated by `,` or `;`; a single mask may be quoted when it contains one.
- `/…/i` — the whole mask is a Perl regular expression instead.
- `include|exclude` — one `|`, exclusions after it.
- `<name>` — a **mask group**, editable in the Options menu, nestable, with `arc`, `temp` and
  `exec` predefined; `<arc>|*.rar` is "archives except rar".

On top of masks sit three features we have none of:

- **Filters**: named condition sets — mask, size range (`B K M G T P E`, binary; lowercase for
  decimal), date range on any of the four timestamps, absolute or **relative in days**,
  attributes as three-state include/exclude/ignore, "has more than one hard link". Each
  condition separately switchable. Filters are used for the panel display, for find, and for
  copy.
- **Sort groups**: masks that pull files to the front or the back of the sort, before or after
  the separator in their list.
- **Highlighting**: colour per mask/attribute group, with its own list and editor.

## 6. Find, histories, screens

| Feature | Keys | Status |
| --- | --- | --- |
| Find file: masks, text or hex, case, whole words, **fuzzy** (diacritics, ligatures, fancy numerals), "not containing", code page (one, all standard, or a chosen set), search in archives, include folders, follow symlinks, search alternate streams | `Alt+F7` | ✗ |
| Search scope: fixed drives, local drives, `%PATH%`, from here, this folder only, the selected folders | — | ✗ |
| Advanced: stop after the first N bytes (with size suffixes), result columns in the column grammar | — | ✗ |
| Results: new search, go to, view (search continues behind the viewer), **panelize**, stop | — | ✗ |
| Find folder in the tree | `Alt+F10` | ✗ |
| Four histories — commands, folders, view/edit, dialogs — each with a size and a lifetime in days, saved across sessions, with items that can be **locked** so new entries do not push them out | `Alt+F8`, `Alt+F12`, `Alt+F11`, `Ctrl+↑/↓` | ◐ folder history, in-session only |
| Task list, screens list, user screen, and the count of background screens | `Ctrl+W`, `F12`, `Ctrl+O` | ✗ |
| Screen grabber: stream or block selection, extend, move, append to the clipboard | `Alt+Ins` | ✗ |

## 7. Viewer and editor

Both are close to ours; these are the differences worth noting.

| Feature | Keys | Status |
| --- | --- | --- |
| Viewer ⇄ editor at the same position | `F6` | ✗ |
| Jump to the viewed/edited file in the panel | `Ctrl+F10` | ✗ |
| Next / previous file of the panel without leaving the viewer | `Gray +/-` | ✗ |
| Select text with the mouse, copy to clipboard | — | ✗ |
| Key bar, status line, scrollbar toggles | `Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S` | ✗ |
| Remember per file: position, code page, view mode, wrap mode, bookmarks | — | ✗ |
| Max line width setting (100…100 000) | — | ✗ |
| Show a printable character for `\0` | — | ✗ |
| Editor: append block to the clipboard | `Ctrl+Gray +` | ✗ |
| Editor: next keypress is a character code | `Ctrl+Q` | ✗ |
| Editor: start/end of screen | `Ctrl+N`, `Ctrl+E` | ✗ |
| Editor: indent the current line when nothing is selected | `Alt+U`, `Alt+I` | ◐ `Tab`/`Shift+Tab` on blocks |
| Editor: read-only lock, warn on opening read-only, allow editing files held open | `Ctrl+L` | ◐ lock only |
| Editor: show whitespace, cursor beyond end of line, select found, cursor at end of found | — | ✗ |
| Editor: three tab policies (don't expand, expand new only, expand all on open) | — | ◐ two |
| Code pages menu with favourites, auto-detection, a configurable `F8` cycle, custom names | `Shift+F8` | ◐ fixed cycle |

## 8. Command line (the one thing we have nothing of)

This is the largest single hole in scmd, and several other features hang off it.

- Full line editing: word movement, `Ctrl+K` to end of line, `Ctrl+BS`/`Ctrl+Del` by word,
  history with `Ctrl+E`/`Ctrl+X`, `Ctrl+Y` to clear.
- Insertion keys: file name from the active or passive panel, full name, UNC name, path from the
  left, right, active or passive panel — eleven combinations in all.
- Autocomplete, with four separately switchable sources (filesystem, history, `%PATH%`,
  environment), each three-state (yes / no / only when asked with `Ctrl+Space`), shown as a list
  that can be modal, optionally appending the first match as you type.
- A prompt format language (`$p$g` by default).
- Commands FAR runs itself, without a shell: `cls`, `drive:`, `cd`/`chdir` (including `cd ~`
  with a configurable home), `chcp`, `set`, `if [not] exist`, `if [not] defined` (nestable),
  `pushd`/`popd`/`clrd`, `title`, `exit`.
- Command **prefixes** (from the bundled `farcmds` plugin): `view:`, `edit:`, `goto:`, `clip:`,
  `whereis:`, `link:`, `run:`, `load:`, `unload:`, with output redirection modifiers.
- `Ctrl+O` shows the user screen (the output of the last command) and `Ctrl+Alt+Shift` peeks at
  it while held.

**The metasymbols** are the glue between the command line, the user menu, file associations and
"apply command". Any of them would need this grammar to be useful:

```
!!  literal !      !      name without extension     !.!  name with extension
!`  extension      !@!    file containing the list of selected names
!&  the list of selected names inline              !\   current path
!=\ current path resolved through symlinks         !?!  description of the current file
!?<title>?<init>!  ask the user, with an optional $<history>$ prefix
! !^ ![ !]         switch all following symbols to the passive / active / left / right panel
modifiers on !@!   Q quote, S forward slashes, F full paths, A ANSI, U UTF-8, W UTF-16
```

## 9. Configuration, interface, system

| Feature | Status |
| --- | --- |
| 311 settings, every one reachable from a `far:config` browser with search, and a `Default.farconfig` template | ◐ ~45 keys, `--dump-config`, no browser |
| Settings dialogs: system, panel, tree, interface, dialogs, menus, command line, autocomplete, viewer, editor, confirmations (15), plugin manager, mask groups, code pages, colours, panel modes, descriptions | ◐ 4 dialogs |
| Save setup, plus an **auto-save on exit** option that also stores both panels' folders | ◐ `Shift+F9` only |
| Portable: profile location, template profile, global user menu folder, read-only configuration — all from `Far.exe.ini` | ◐ portable mode, one config file |
| Colour **groups** and colour **themes**, with 16-colour, 256-colour and RGB pickers | ◐ 6 themes, no per-group editor |
| Virtual-terminal rendering (8- and 24-bit colour), fullwidth-aware rendering, ClearType-friendly redraw | ◐ 24-bit with degradation; width handled always |
| Clock, screen saver with a timeout, mouse, always-show-menu-bar, console icon, window-title variables (`%Ver`, `%Platform`, `%Admin`, `%PID`) | ✗ |
| Sorting collation: ordinal / invariant / linguistic, digits-as-numbers, case sensitivity — as three separate settings | ◐ natural sort only |
| Elevation: ask for administrator rights per operation class (modify / read / extra privileges) | ⊘ |
| XLat: transliterate a typed line between keyboard layouts, with tables, rules and flags | ✗ |
| Mouse wheel everywhere, with a configurable line count per area, and horizontal wheel support | ✗ |
| Menus: filter-as-you-type (`RAlt`), lock the filter, horizontal scrolling and alignment of items, hide fixed columns | ✗ |
| Dialogs: history in edit controls (with lockable items), autocomplete, three-state checkboxes, move with `Ctrl+F5`, click outside to accept or cancel, insert a file name with `Shift+Enter` | ◐ plain prompts and dialogs |
| Macros: 14 areas, double-modifier hotkeys, record and play, two play modes, per-macro settings, a list, and a Lua engine (`luamacro`) | ✗ |

## 10. Drives, network, plugins

FAR's drive menu (`Alt+F1`/`Alt+F2`) is a small application: nine information columns toggled
with `Ctrl+1…9` (type, network/SUBST/VHD path, label, filesystem, size and free space,
removable parameters, plugins, CD type, network parameters), `Del` to disconnect a network
drive, delete a SUBST drive, detach a VHD or eject a disc, `Shift+Del` to stop a USB device
safely, `Shift+F6` to change a volume label, `Ctrl+H` for unmapped volumes, and per-plugin
hotkeys. Ours lists roots and open connections and connects: ◐.

What FAR ships as **plugins**, and what it tells us about scope:

| Plugin | What it is | Our answer |
| --- | --- | --- |
| `arclite` | archives through the 7-Zip engine | planned (SPEC 04 §4) |
| `multiarc` | archives through external archivers | planned |
| `ftp` | FTP/FTPS client as a panel | planned (SPEC 04 §6) |
| `network` | network browser | ⊘ |
| `tmppanel` | a panel made of an arbitrary file list | needed for find-panelize |
| `proclist` | the process list as a panel | an idea, not a need |
| `compare` | folder compare with contents and subfolders | planned |
| `emenu` | the OS context menu | ⊘ |
| `farcmds` | the command prefixes above | wanted, cheap |
| `luamacro` | the Lua macro engine | planned (SPEC 08) |
| `align`, `autowrap`, `brackets`, `drawline`, `editcase`, `filecase`, `samefolder`, `hlfviewer`, `macroview` | small editor and panel helpers | the kind of thing our plugin API should make easy |

The lesson is architectural, not feature-by-feature: FAR's core is a panel engine, a dialog
engine, a macro engine and a plugin API — and *everything else*, archives included, is a plugin.
Our SPEC 08 says the same thing; this confirms the shape rather than changing it.

## 10b. How close the screen is

The question "can it look like FAR?" has a short answer: yes, because it is all character cells
and sixteen colours, and nothing in FAR's screen needs anything a terminal cannot do. What is
already identical:

- **The palette.** The `far` theme is FAR's own default table from `far/palette.cpp`, element for
  element: light cyan on blue panels, yellow for marked files and column titles, a cyan bar on
  the active panel's path, cyan menus with yellow shortcuts, light grey dialogs with cyan input
  fields, red warning dialogs, light grey numbers on black in the key bar.
- **The panel chrome.** The column titles row under the frame, with the sort-mode letter in the
  top-left corner — lower case for ascending, upper case for descending — and a `*` beside it
  while a filter is on. The active panel's path is drawn inverted.
- **Menus are menus.** A pulled-down menu is cyan with yellow shortcuts and a black selection
  bar, not a grey dialog; the bar across the top is black on cyan with a black selection.
- **Shadows.** A dialog, a menu, a list or a progress window darkens two columns to its right
  and one row below it, which is `DropShadow`'s geometry, by halving what is there rather than
  painting a colour — so it works in any theme and degrades to 16 colours.
- **The key bar wording**, taken from FAR's own catalogues in each language we share with it
  (`UserMn`, `MkFold`, `ConfMn`, and their Italian, German, Spanish and Russian forms).

What is still different, and why:

| Difference | Why |
| --- | --- |
| One status line at the bottom of the screen, not one inside each panel | FAR shows the item under the cursor in the panel's own bottom line. Ours is global, which is a real design choice to revisit when the command line lands, because that is what competes for the same rows. |
| No free space in the panel frame, no scrollbar, no screens counter, no clock | Each is a small drawing job; the clock also needs a repaint on a timer, which the input-driven loop does not do yet. |
| Four view modes, not ten | The missing six are descriptions, long descriptions, owners, links, medium and alternative full — and most of them need data we do not read yet (descriptions, owners). |
| No dialog shadow *under* a dropped-down list inside a dialog | We have no nested floating windows yet. |

## 11. What this changes for us

Nothing in the specs turned out wrong, but the gap list grew and sharpened. In rough order of
how much each one unlocks:

1. **The command line** (`docs/BACKLOG.md`): with it come history, `Ctrl+O`, apply-command,
   prefixes, metasymbols and the user menu. Without it, six menu entries stay dim.
2. **Filters as a shared object**: used by the panel, by find and by copy. Building it once, as
   FAR does, is cheaper than three times.
3. **The column grammar**: it is what makes 10 view modes one feature instead of ten. Worth
   adopting verbatim, including the status-line columns.
4. **Histories with lifetimes and locked items**, persisted — small, and felt every day.
5. **The selection matrix** and the under-cursor variants of `F5`/`F6`/`F8`: an afternoon each,
   and they are the keys a FAR user hits without thinking.
6. **Sort: ctime/atime/owner/description, criteria chains, sort groups** — our `Sort` already has
   the shape (`chain` is in SPEC 07), so this is filling in rather than designing.
7. **Find file**, which is a project of its own, and the temporary panel that holds its results.

Everything in this file that we intend to build has a line in `docs/BACKLOG.md`; everything we
intend *not* to build (`⊘`) is marked there as Windows-specific, with what we do instead.
