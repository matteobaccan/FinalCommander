# FAR 3.x, feature by feature, read from the source

`docs/02-features.md` is the union of many commanders, written from documentation and from use.
This file is narrower and harder: it is what **FAR Manager 3.x actually does**, read from its own
repository (`FarGroup/FarManager`, BSD-3-Clause, HEAD `60567ba`, fetched 2026-10-24), and compared
line by line with what FinalCommander does. The FAR side of every row is fixed — it is read
from that revision — and the **status column is re-read at each release**; it is as of 0.10.0
here. `docs/BACKLOG.md` is the live list of what is deliberately not built yet, with reasons;
this file is the measuring stick it is measured against.

Sources used, in order of authority:

| Source | What it gives |
| --- | --- |
| `far/FarEng.hlf.m4` | the English help: 283 topics, every dialog, every key, every option |
| `far/config.cpp` (`RoamingData` / `LocalData` tables) | **311 settings keys** in 33 sections — the real configuration surface |
| `far/farlang.templ.m4` | every UI string in 13 languages: the exact menu and dialog wording |
| `far/Far.exe.example.ini` | the launcher-level settings (profile location, portability, read-only config) |
| `far/changelog` | 66 662 lines of history: when and why a behaviour is the way it is |
| `plugins/*` | what FAR ships as plugins rather than as core |

Status column: ✅ in fcmd 0.10.0 · ◐ partly, with what is missing spelled out · ✗ missing ·
⊘ Windows-specific, needs a different answer on three platforms (not a straight port).

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

For comparison, fcmd 0.10.0 has 86 configuration keys in 11 sections, all 10 view modes, all 9
sort keys, macros in 5 scopes, 3 confirmations, and providers for local files, archives
(zip, 7z, the tar family), SFTP, FTP, FTPS, WebDAV and object storage. What is left of the gap
is the roadmap, and most of what remains is Windows-specific or is a plugin in FAR too.

## 2. Panels

| Feature | Keys | Status |
| --- | --- | --- |
| Two panels, active/passive | `Tab`, `Ctrl+U` swap | ✅ |
| Re-read | `Ctrl+R` | ✅ |
| Info panel | `Ctrl+L` | ✅ device, cluster size, what the directory costs, memory, the row under the cursor |
| Quick view panel | `Ctrl+Q` | ✅ (a folder shows the same device and directory figures) |
| Tree panel, with an on-disk tree cache per drive | `Ctrl+T` | ◐ the panel, read live; no on-disk cache per drive, which is what `Alt+F10` and the copy dialog's tree are waiting for |
| Hide both panels / hide passive / hide left / hide right | `Ctrl+O`, `Ctrl+P`, `Ctrl+F1`, `Ctrl+F2` | ✅ all four, and with the panels off the command line, the menus and the keys all still work |
| Temporarily uncover the screen while keys are held | `Ctrl+Alt+Shift` | ✗ — a terminal reports a key, not a modifier being held, so there is nothing to act on; `Ctrl+O` is the answer here |
| Resize panels: width, height, per-panel height, and reset | `Ctrl+←/→`, `Ctrl+↑/↓`, `Ctrl+Shift+↑/↓`, `Ctrl+Numpad5`, `Ctrl+Alt+Numpad5` | ◐ width (`Ctrl+←/→`) and the shared bottom edge (`Ctrl+↑/↓`), both settings `Shift+F9` keeps; no per-panel height and no numpad reset |
| Key bar on/off | `Ctrl+B` | ◐ a setting in the panels (`ui.key_bar`, and the Interface dialog); a key in the viewer, where `Ctrl+B`, `Ctrl+Shift+B` and `Ctrl+S` turn the bar, the status line and the scrollbar |
| Total/free size display mode | `Ctrl+Shift+S` | ◐ the free space is drawn in the frame and is a setting (`panel.free_space`); there is no key to change what the figure counts |
| 10 view modes: brief, medium, full, wide, detailed, descriptions, long descriptions, owners, links, alternative full | `LeftCtrl+1…0` | ✅ all ten, on FAR's own numbers (`Ctrl+1…0`); the three that need what a listing does not carry are each filled by a worker while that view is on screen. What is still owed is the column grammar that makes each mode re-definable — ours are ten fixed lists |
| Hidden and system files | `Ctrl+H` | ✅ |
| Long/short (8.3) names | `Ctrl+N` | ⊘ |
| Sort by name, extension, write time, size, unsorted, creation time, access time, description, owner | `Ctrl+F3…F11` | ✅ all nine; the last two are fetched by a worker and sort again when the answer lands |
| Sort modes list, with `+`/`-`/`*` to set the direction and `F4` to fine-tune | `Ctrl+F12` | ✅ (list, and re-choosing reverses) |
| **Sort criteria chain**: per sort mode, an ordered set of tie-breakers, edited with `Ins`/`Del`/`F4`/`+`/`-`/`*`/`=`/`Ctrl+↑↓`/`Ctrl+R` | — | ◐ `panel.sort_chain` in the configuration, up to three tie-breakers, with the name having the last word; no dialog to edit it and not per sort mode |
| Sort groups: named mask groups that sort before or after everything else | `Shift+F11` | ◐ the bands work, from `[[sort_group]]` blocks, and `Shift+F11` turns them on and off; the editor for them is a backlog line |
| Selected files first | `Shift+F12` | ✅ (setting) |
| Folder shortcuts: 10 slots, set and jump | `Ctrl+Shift+0…9`, `RightCtrl+0…9` | ✅ `Ctrl+Shift+0…9` sets, `Alt+0…9` goes — no terminal can tell the two control keys apart — and `Ctrl+D` is the whole list, where one can be named |
| Scroll long names and descriptions sideways | `Alt+←/→`, `Alt+Home/End` | ◐ `Ctrl+Shift+←/→` scrolls the name column, because `Alt+←/→` is the folder history here and `Alt+<letter>` belongs to the quick search; no jump to either end |
| Column titles, status line, file totals, free size, scrollbar, sort-mode letter, `..` in root, background-screen count — each a setting | — | ◐ column titles, the sort letter and the filter mark, file totals yes (`panel.column_titles`); free size, the per-panel status line, the scrollbar, `..` in root and the screen count no |
| Automatic update on filesystem change, with a size threshold that turns it off, and a separate switch for network drives | — | ◐ a local panel watches its directory (`panel.auto_refresh`) and keeps the cursor on its file; no size threshold and no separate switch for network drives — a remote panel simply does not watch |
| Detect volume mount points, distinguish junctions | — | ◐ a junction is recognised, drawn as a link and sorted with the folders; FAR's mount-point detection is Windows-specific and not ported |
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

fcmd: ✅ `Ins` and `Shift+arrows`, mask `+`/`-`/`*`, select all/none/invert, restore
(`Ctrl+Shift+M`), the extension variants on `Ctrl+Shift++`/`Ctrl+Shift+-` and the name
variants on `Alt++`/`Alt+-`/`Alt+*` — `Ctrl+Gray+` is FAR's own font key here and `Alt+<letter>`
belongs to the quick search, which is why those two moved. Right-click selects a row with the
mouse on. ✗ right-drag, and the `Select folders` setting that decides whether the mask keys
touch directories at all (a backlog line).

## 4. File operations

| Feature | Keys | Status |
| --- | --- | --- |
| Copy, move/rename, make folder, delete | `F5`, `F6`, `F7`, `F8` | ✅ |
| Same four, but on the item under the cursor, ignoring the selection | `Shift+F5`, `Shift+F6`, `Shift+F8` | ✅ |
| Delete skipping the recycle bin | `Shift+Del` | ✅ `F8` goes to the system's own wastebasket — the one the desktop's file manager shows — and `Shift+Del` deletes outright; `general.use_trash` decides which `F8` means |
| Wipe: overwrite, truncate, rename, unlink | `Alt+Del` | ✅ overwrite, truncate, rename, unlink, in that order, with what it cannot promise on a copy-on-write filesystem or an SSD said in the dialog rather than implied |
| Create hard links and symlinks | `Alt+F6` | ✅ hard link, symbolic link, and a junction on Windows |
| Several destinations in one copy (`;`-separated, quoted if needed) | — | ✅ FAR's own convention, quotes included, so a directory called `a;b` can still be named |
| Existing-file policy chosen up front: ask, overwrite, skip, **append**, only newer, plus "also ask for read-only" | — | ✅ ask, overwrite, newer, skip, rename and append, chosen in the dialog before anything is written, with the same choices offered again on the conflict itself |
| Access rights for new files: default, copy, inherit | — | ◐ the mode is carried over or left to the umask (`copy.mode`); *inherit* is a Windows ACL idea with no Unix meaning |
| Copy the contents of symbolic links instead of the links | — | ✅ `copy.follow_links`, off by default as it is in FAR: a copy that silently turned links into copies of their targets would duplicate a tree the first time anybody tried it |
| Copy files other programs hold open for writing | — | ⊘ |
| A **filter** applied to the copy, so only matching files travel | — | ✅ a mask in the copy dialog |
| Destination picked from a folder tree, prefilled from the active panel, the passive one, or the typed path | `F10`, `Alt+F10`, `Shift+F10` | ◐ prefilled from the passive panel only |
| Total progress bar, and average speed / elapsed / ETA | — | ✅ a per-file and a total bar, with the speed and the time left beside them (and no time left shown while there is nothing to compute it from) |
| File attributes dialog: the full NTFS attribute set, four timestamps, owner, hard links, streams | `Ctrl+A`, `F4` on a folder | ◐ the Unix mode bit by bit (owner/group/other, setuid, setgid, sticky), the read-only, hidden and system flags where the platform has them, the write and read times, and recursion into a folder; ✗ the NTFS streams and the owner as something to *change* |
| Apply a command line to each selected file | `Ctrl+G` | ✅ with FAR's metasymbols for the parts of the name |
| Describe files (`descript.ion` / `files.bbs`) | `Ctrl+Z` | ✅ both names read, written back in UTF-8, shown by `Ctrl+6`/`Ctrl+7`, in the information panel and the status line, sorted by (`Ctrl+F10`), and carried by a copy, a move, a rename and a delete; ✗ inside an archive or on a server |
| Compare folders | Commands menu | ✅ beyond FAR's own: by name, size and time; by content; and a dialog of what counts as a difference — case, a time tolerance, contents only when the time differs, a first-fragment read, line-endings-only, selected only, stop at the first, newer only — which is what `AdvCmpEx` is for in FAR. Recursive through the subdirectories, where the answer is a panel of marked rows, and `Find duplicates` by content under one tree |
| Two files side by side with what differs marked | — (`Ctrl+Shift+D` here) | ✅ beyond FAR, which has no diff of its own and leaves it to a plugin: the file under each panel's cursor, two columns, the part of a changed line that changed marked inside it, `Alt+↑↓` from hunk to hunk, and blanks, case and line endings each a keystroke. Patience diff in `fcmd-core`, local files, 16 MiB a side |
| Copy names, full names, UNC names, or the files themselves, to the clipboard | `Ctrl+Ins`, `Alt+Shift+Ins`, `Ctrl+Alt+Ins`, `Ctrl+Shift+C/X` | ◐ the names (`Ctrl+Ins`) and the whole paths (`Alt+Shift+Ins`), through the terminal's own `OSC 52`, so it works over `ssh` as well; ✗ the files themselves as clipboard objects, which is a desktop protocol and not a terminal one |

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
| Find file: masks, text or hex, case, whole words, **fuzzy** (diacritics, ligatures, fancy numerals), "not containing", code page (one, all standard, or a chosen set), search in archives, include folders, follow symlinks, search alternate streams | `Alt+F7` | ◐ masks, a size condition, text or hex (and a regular expression, which FAR has not), case, whole words, not-containing, folders, symlinks; ✗ fuzzy matching, the code-page set, searching inside archives (dim in the dialog, which says so), and alternate streams |
| Search scope: fixed drives, local drives, `%PATH%`, from here, this folder only, the selected folders | — | ◐ from here, this folder only, the selected folders, the root, and the other panel; ✗ all fixed drives and `%PATH%`, which on three platforms mean three different things |
| Advanced: stop after the first N bytes (with size suffixes), result columns in the column grammar | — | ✗ both, and the second waits on the column grammar itself (§2) |
| Results: new search, go to, view (search continues behind the viewer), **panelize**, stop | — | ✅ the list fills while the walk is still running, `Enter` goes to a row, `F3` views one, `F5` puts the results in the panel, and `Esc` stops the walk |
| Find folder in the tree | `Alt+F10` | ✗ — wants the tree cache (§2) |
| Four histories — commands, folders, view/edit, dialogs — each with a size and a lifetime in days, saved across sessions, with items that can be **locked** so new entries do not push them out | `Alt+F8`, `Alt+F12`, `Alt+F11`, `Ctrl+↑/↓` | ✓ all four, each with a size and all persisted in the state file: the commands (`Alt+F8`, and `↑`/`↓` on the line), the folders per panel (`Alt+F12`), the files read or edited (`Alt+F11`); ✓ a lifetime in days, lines pinned with `Ins`, filtering a history as one types, and the dialogs' own fields (`Ctrl+↓`) |
| Task list, screens list, user screen, and the count of background screens | `Ctrl+W`, `F12`, `Ctrl+O` | ◐ the screens list (`F12`), the user screen (`Ctrl+O`, with `PgUp`/`PgDn` through what the commands printed) and the task list — the processes, on FAR's own `Ctrl+W` and in the Commands menu, as the panel `Alt+F1` also opens, with `F3` on a row showing the process's card, and the count of what is behind the panels in the status line's corner rather than the panel's own; ✗ killing a process with `Del` |
| Screen grabber: stream or block selection, extend, move, append to the clipboard | `Alt+Ins` | ◐ a rectangle of the screen, marked with `Shift+arrows` from where the cursor already was, `Ctrl+A` for the lot, `Enter` or `Ctrl+Ins` to copy, and the row under the cursor when nothing is marked (SPEC 06 §11b); ✗ a stream selection, and appending to the clipboard — which wants the clipboard to be readable, and over `OSC 52` it is not |

## 7. Viewer and editor

Both are close to ours; these are the differences worth noting.

| Feature | Keys | Status |
| --- | --- | --- |
| Viewer ⇄ editor at the same position | `F6` | ✅ both ways |
| Jump to the viewed/edited file in the panel | `Ctrl+F10` | ✅ |
| Next / previous file of the panel without leaving the viewer | `Gray +/-` | ✅ |
| Select text with the mouse, copy to clipboard | — | ✗ — with `ui.mouse` off, which is the default, the terminal's own selection does this, which is why it is not urgent |
| Key bar, status line, scrollbar toggles | `Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S` | ✅ all three, on FAR's own keys, and each is a setting as well |
| Remember per file: position, code page, view mode, wrap mode, bookmarks | — | ✗ — a dim row in the viewer's own dialog says so, so that it is visible as owed rather than forgotten |
| Max line width setting (100…100 000) | — | ✗ |
| Show a printable character for `\0` | — | ✗ |
| Editor: append block to the clipboard | `Ctrl+Gray +` | ✅ on `Ctrl+Shift+Ins`, because `Ctrl++` is the font key here |
| Editor: next keypress is a character code | `Ctrl+Q` | ✅ |
| Editor: start/end of screen | `Ctrl+N`, `Ctrl+E` | ✅ |
| Editor: indent the current line when nothing is selected | `Alt+U`, `Alt+I` | ✅ on FAR's own two keys, with `Tab`/`Shift+Tab` on a block |
| Editor: read-only lock, warn on opening read-only, allow editing files held open | `Ctrl+L` | ◐ the lock (`Ctrl+Alt+L`, since `Ctrl+L` is the information panel); ✗ the warning and the override |
| Editor: show whitespace, cursor beyond end of line, select found, cursor at end of found | — | ◐ show whitespace and the cursor after a match, both settings; ✗ the cursor beyond the end of a line and leaving a match selected |
| Editor: three tab policies (don't expand, expand new only, expand all on open) | — | ✅ `editor.tabs` is `keep`, `typed` or `all` |
| Code pages menu with favourites, auto-detection, a configurable `F8` cycle, custom names | `Shift+F8` | ◐ detection and a fixed `F8` cycle; ✗ the menu, favourites and custom names |

## 8. Command line

This was the largest single hole in fcmd; most of it is filled, and what is left is listed
here with the rest.

- Full line editing: word movement, `Ctrl+K` to end of line, `Ctrl+BS`/`Ctrl+Del` by word,
  history with `Ctrl+E`/`Ctrl+X`, `Ctrl+Y` to clear. — ✅, with `↑`/`↓` through the history
  when something is typed and `Alt+F8` for the whole list, and `Shift`+arrows marking text out
  so that `Ctrl+Ins` copies the selection rather than the line.
- Insertion keys: file name from the active or passive panel, full name, UNC name, path from the
  left, right, active or passive panel — eleven combinations in all. — ◐ three of them:
  `Ctrl+Enter` the name under the cursor, `Ctrl+Shift+Enter` the other panel's, `Ctrl+[` the
  path. The rest are a backlog line; the metasymbols below are the general answer.
- Autocomplete, with four separately switchable sources (filesystem, history, `%PATH%`,
  environment), each three-state (yes / no / only when asked with `Ctrl+Space`), shown as a list
  that can be modal, optionally appending the first match as you type. — ✗, and the largest
  single thing still missing here.
- A prompt format language (`$p$g` by default). — ✅ `cmdline.prompt`, FAR's own letters, plus
  `$r` for the git branch (`%GITBRANCH%` as well) and `$+` for the `pushd` depth.
- Commands FAR runs itself, without a shell: `cls`, `drive:`, `cd`/`chdir` (including `cd ~`
  with a configurable home), `chcp`, `set`, `if [not] exist`, `if [not] defined` (nestable),
  `pushd`/`popd`/`clrd`, `title`, `exit`. — ◐ `cls`, `cd` (`cd ~` included), `set`, `if exist`
  and `if defined` with their negations, `pushd`/`popd`/`clrd`, `exit`; `title` says it is not
  implemented rather than doing nothing, and `drive:`/`chcp` are Windows ideas.
- Command **prefixes** (from the bundled `farcmds` plugin): `view:`, `edit:`, `goto:`, `clip:`,
  `whereis:`, `link:`, `run:`, `load:`, `unload:`, with output redirection modifiers. — ◐
  `view:`, `edit:`, `goto:`, `whereis:` and `run:` (with its output going where you say);
  ✗ `clip:`, `link:`, `load:`, `unload:`.
- `Ctrl+O` shows the user screen (the output of the last command) and `Ctrl+Alt+Shift` peeks at
  it while held. — ✅ for `Ctrl+O`, with `PgUp`/`PgDn` through the output and the panels going
  off by themselves while a command runs so it can be watched arriving; the peek-while-held
  cannot be done in a terminal (§2).

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
| 311 settings, every one reachable from a `far:config` browser with search, and a `Default.farconfig` template | ◐ 86 keys in 11 sections, `--dump-config` to see them all with their values, no browser |
| Settings dialogs: system, panel, tree, interface, dialogs, menus, command line, autocomplete, viewer, editor, confirmations (15), plugin manager, mask groups, code pages, colours, panel modes, descriptions | ◐ system, panel, interface, viewer, editor, the highlighting groups, the comparison, the drive dialog's own columns, and the lists for themes, languages, colours and code pages; ✗ the tree, the dialogs' and menus' own settings, autocomplete, the mask groups, the sort groups and the panel modes |
| Save setup, plus an **auto-save on exit** option that also stores both panels' folders | ◐ `Shift+F9` saves on demand, and the tabs, the histories and the hotlist come back by themselves; ✗ the auto-save switch and starting where the last session ended |
| Portable: profile location, template profile, global user menu folder, read-only configuration — all from `Far.exe.ini` | ◐ portable mode, one config file |
| Colour **groups** and colour **themes**, with 16-colour, 256-colour and RGB pickers | ◐ 6 themes, no per-group editor |
| Virtual-terminal rendering (8- and 24-bit colour), fullwidth-aware rendering, ClearType-friendly redraw | ◐ 24-bit with degradation; width handled always |
| Clock, screen saver with a timeout, mouse, always-show-menu-bar, console icon, window-title variables (`%Ver`, `%Platform`, `%Admin`, `%PID`) | ◐ the mouse (off by default, and the reason is in the help); ✗ the rest — the clock and the screen saver both want a repaint on a timer, which the input-driven loop does not do yet |
| Sorting collation: ordinal / invariant / linguistic, digits-as-numbers, case sensitivity — as three separate settings | ◐ natural sort only |
| Elevation: ask for administrator rights per operation class (modify / read / extra privileges) | ⊘ |
| XLat: transliterate a typed line between keyboard layouts, with tables, rules and flags | ✗ |
| Mouse wheel everywhere, with a configurable line count per area, and horizontal wheel support | ◐ the wheel scrolls whatever is under the pointer, three rows a notch, without taking the focus — in the panels, the lists, the viewer and the editor; ✗ a configurable count per area and the horizontal wheel. The mouse as a whole is off until asked for (`ui.mouse`), because a terminal that is reporting it stops acting on it, and the text selection and middle-click paste are the terminal's own |
| Menus: filter-as-you-type (`RAlt`), lock the filter, horizontal scrolling and alignment of items, hide fixed columns | ✗ |
| Dialogs: history in edit controls (with lockable items), autocomplete, three-state checkboxes, move with `Ctrl+F5`, click outside to accept or cancel, insert a file name with `Shift+Enter` | ◐ dialogs with pages, text rows, choices, toggles and dim rows that say what is not implemented, and a click outside does nothing at all (deliberately); ✗ the rest |
| Macros: 14 areas, double-modifier hotkeys, record and play, two play modes, per-macro settings, a list, and a Lua engine (`luamacro`) | ◐ record and play (`Ctrl+.`), a list (`Ctrl+Shift+.`), five scopes rather than fourteen areas, persisted in their own file, and a macro that records *commands* rather than keystrokes — so it survives a keymap change and can run once for every selected file; ✗ the conditions and the Lua engine, which both want the plugin host's question-asking API |

## 10. Drives, network, plugins

FAR's drive menu (`Alt+F1`/`Alt+F2`) is a small application: nine information columns toggled
with `Ctrl+1…9` (type, network/SUBST/VHD path, label, filesystem, size and free space,
removable parameters, plugins, CD type, network parameters), `Del` to disconnect a network
drive, delete a SUBST drive, detach a VHD or eject a disc, `Shift+Del` to stop a USB device
safely, `Shift+F6` to change a volume label, `Ctrl+H` for unmapped volumes, and per-plugin
hotkeys.

Ours is now most of that: ◐. It opens over the panel it is about, draws the type, the device,
the label, the filesystem, the size and the free space — each column a switch, and `F9` inside
the dialog is the dialog of switches, as it is in FAR — reads the drives again on `Ctrl+R`, and
puts under a rule, on `Ctrl+1…9`, the things that are not filesystems: the stored connections,
the object stores, the processes, the sockets, the installed programs and the temporary panel.
What is missing is the part that acts on a volume rather than listing it — `Del` to disconnect
or eject, `Shift+Del` to stop a device, `Shift+F6` to relabel — and `Ctrl+H` for the volumes
with no mount point, which wants `FindFirstVolumeW` and is a backlog line.

What FAR ships as **plugins**, and what it tells us about scope:

| Plugin | What it is | Our answer |
| --- | --- | --- |
| `arclite` | archives through the 7-Zip engine | planned (SPEC 04 §4) |
| `multiarc` | archives through external archivers | planned |
| `ftp` | FTP/FTPS client as a panel | planned (SPEC 04 §6) |
| `network` | network browser | ⊘ |
| `tmppanel` | a panel made of an arbitrary file list | needed for find-panelize |
| `proclist` | the process list as a panel | an idea, not a need |
| `compare` | folder compare with contents and subfolders | ✅ shipped, with the dialog, the subfolders, and a side-by-side diff of two files beside it (`Ctrl+Shift+D`), which FAR has in no plugin of its own |
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
