# Backlog

What is specified but not implemented, in the order the menus mention it. Every dim entry in the
menu bar (SPEC 10) has a line here. "Open question" means a decision is needed before coding,
not that the work is large.

The sections after the menu ones come from `docs/04-far-inventory.md` — FAR 3.x read from its own
repository and compared with what we have. That file says what each feature *is*; this one says
what we intend to do about it.

## Panels

| Item | Where | Notes |
| --- | --- | --- |
| ~~Info panel~~ | Left/Right menu (`Ctrl+L`) | **Shipped.** Where the other panel is, what kind of place that is (this machine, a server, inside an archive, a list), the free space, what the directory holds, what is selected, and the row under the cursor. The `Ctrl+L` question is settled: it is FAR's key for this, and the change-directory dialog moved to `Ctrl+Shift+L`. Still to come: mount options and a git summary. |
| ~~Quick view~~ | Left/Right menu (`Ctrl+Q`) | **Shipped.** The viewer's own rows inside a panel's frame, following the other panel's cursor, opened once per file. A directory says what it is rather than being measured, and a file whose bytes are not here says that `F3` brings them out. |
| ~~Tree panel~~ | Left/Right menu (`Ctrl+T`) | **Shipped.** Rooted where the other panel's path starts and opening its way down to where that panel is, with a directory cache that outlives the panel. `→`/`Enter` open, `←` closes or climbs, `*` opens everything under the cursor up to two thousand rows, `Ctrl+R` re-reads a branch. The tabs that wanted `Ctrl+T` took the `Ctrl+Alt` row instead. |
| ~~Panel on/off~~ | Left/Right menu (`Ctrl+F1`/`Ctrl+F2`) | **Shipped**, and the focus follows the screen. |
| ~~Tabs per panel~~ | Left/Right menu (`Ctrl+Alt+T`/`W`/`N`/`P`/`M`/`K`) | **Shipped.** A tab *is* a panel — everything a tab remembers is what a panel remembers — so a side's tabs are a list of panels with one of them on screen and deliberately not in the list. Named, locked to a directory, handed to the other side, and kept between sessions with the rule that the directory the shell was in wins for the tab one lands on unless that tab is locked. The row is drawn only by a side that has more than one. Still to come: reordering a tab within its row, and a list of them to pick from. |
| ~~Compare folders~~ | Commands menu | **Shipped.** FAR's own rule, and why it is instant: names, sizes and times, no subdirectories and no contents. Each panel keeps selected what the other does not have or has older, so what stays selected is what a copy would have to carry. **Compare by content** is shipped beside it (same menu): every pair that matches by name is read on both sides and selected where the bytes differ, with a pair of different sizes settled from the listing rather than opened. Subfolders are still not walked — that is the remaining half, and the one that needs a dialog of its own. |
| ~~Panelize a result set~~ | SPEC 06 §1 | **Shipped.** `F5` in the find results turns them into the panel, through the temporary panel below. |
| The view modes that need more data | `Ctrl+6..0` | FAR's descriptions and long descriptions (`descript.ion`, its own backlog line), file owners (a uid/gid lookup per row, cached), file links (the hard-link count), and the alternative full view (sizes with thousands separators). The five we have — brief, medium, full, wide, detailed — are FAR's first five, on FAR's own keys. |
| Custom columns | `Ctrl+Shift+1..0` | SPEC 02 §3's column grammar: the user's own column sets, which is what makes the modes above a setting rather than a list. |

## Selection and sorting

| Item | Where | Notes |
| --- | --- | --- |
| ~~The selection matrix~~ | panel | **Shipped.** `Ctrl++`/`Ctrl+−` by the current file's extension, `Alt++`/`Alt+−` by name, `Alt+*` inverting folders too, and one restore undoing a whole sweep. Needs a terminal that reports modified printable keys; the keymap doc says which. |
| A `Select folders` setting | Panel settings | Decides whether the mask keys touch directories at all. One setting, three keys behave differently. |
| ~~`Shift+F5`, `Shift+F6`, `Shift+F8`~~ | panel | **Shipped**, and the list travels with the dialog so a later selection cannot widen the copy. |
| Sort by creation time, access time, owner, description | `Ctrl+F8…F11` | **Creation and access time shipped** (`Ctrl+F8`, `Ctrl+F9`), with the missing-field rule in SPEC 02 §4. Owner and description still cost a lookup per entry and a side file. |
| Sort criteria chains | sort modes list, `F4` | Per sort mode, an ordered set of tie-breakers with its own editor (`Ins`/`Del`/`F4`/`+`/`-`/`*`/`=`/`Ctrl+↑↓`/`Ctrl+R`). SPEC 07 already has `chain` in the config shape, so this is filling in a designed hole. |
| Sort groups | `Shift+F11` | Named mask groups that sort before or after everything else, ordered, with a separator marking "front" from "back". |
| ~~Scroll long names sideways~~ | `Ctrl+Shift+←/→` | **Shipped**, per panel, reset on a change of directory. `Alt+←/→` stays history here. |

## Masks, filters, groups

| Item | Where | Notes |
| --- | --- | --- |
| ~~Mask groups `<arc>`, `<temp>`, `<exec>`~~ | Options menu | **Shipped** as values (nestable, used by the filter bar and by file highlighting). The *editor* for them in the Options menu is still to come. |
| ~~The full mask grammar~~ | everywhere | **Shipped**: `[c-f]` ranges, `/regex/i`, `include|exclude`, lists with either separator. |
| ~~Filters as a shared object~~ | panel, find, copy | **Shipped** in `scmd-core::filter`: masks, size, four date fields (absolute or relative in days), attributes as three states, hard links — one value type, used by the panel's filter bar today. |
| Files highlighting | Options menu | Colour by mask and attribute group, with its own list and editor. Wants the filter object above. |

## Files

| Item | Where | Notes |
| --- | --- | --- |
| ~~Create link~~ | Files menu (`Alt+F6`) | **Shipped** as a relative symlink in the other panel. The engine does hard and absolute links too; the chooser is the remaining work. |
| ~~Wipe~~ | Files menu (`Alt+Del`) | **Shipped**, and the dialog says what overwriting cannot promise on an SSD, on copy-on-write or with snapshots. |
| ~~File attributes~~ | `Ctrl+A`, Files menu | **Shipped.** The mode as twelve boxes (owner, group, world × read, write, run, plus setuid, setgid and sticky), read-only, the modification and access times, and *and everything inside the folders* for a tree. Windows gets its four bits instead of the mode; a file on a server gets the mode and the modification time, which is what SFTP carries; an entry inside an archive and an object in a bucket have nothing to set and say so. `Ctrl+A` is FAR's key for it, so select-all moved to FAR's own `Shift+Gray +` with `Ctrl+Shift+A` beside it for terminals that do not report Shift with a printable key. |
| ~~Apply command~~ | Files menu (`Ctrl+G`) | **Shipped.** One command per selected file, in sequence, counted, stopping at the first failure; a template that asks for a value says why it cannot (SPEC 06 §7.3). |
| ~~Copy: several destinations at once~~ | copy dialog | **Shipped.** `;`-separated, quoted when a name contains one. They run one after the other rather than at once — two jobs writing together would fight over the progress box and the disk — and only the first of a *move* is a move: deleting the source after it would leave the second destination nothing to carry. A cancelled or failed destination drops the rest. |
| ~~Copy: append to the existing file, and "only newer" as a policy~~ | copy dialog | **Shipped**: the conflict row offers ask, overwrite, overwrite-if-newer, skip, give-it-a-free-name and append, and *only files newer than the ones there* is a box of its own beside it — the two are not the same, and FAR has both for the same reason: the box holds even when the policy says overwrite. |
| ~~Copy: contents of symbolic links instead of the links~~ | copy dialog | **Shipped** as a box, off by default: a copy that silently turned links into copies of what they point at would duplicate a tree the first time anybody tried it. |
| ~~Copy: a filter, so only matching files travel~~ | copy dialog | **Shipped** as a mask row (`JobOptions::filter`). It applies to files and never to directories, and a directory is planned only once something under it has matched — so `*.rs` out of a tree gives the directories that have Rust in them and no others. |
| Copy: destination from a folder tree | `F10`, `Alt+F10`, `Shift+F10` in the dialog | Wants the tree panel's cache. |
| ~~Copy: average speed, elapsed and estimated time~~ | progress dialog | **Shipped**, from the average rather than the last tick, and absent until there is enough to compute. |
| Clipboard: names, full names and the files themselves | `Ctrl+Ins`, `Alt+Shift+Ins`, `Ctrl+Shift+C/X` | Needs the OS clipboard (SPEC 06). Copying *files* to the clipboard so another application can paste them is platform work, not a one-liner. |
| Descriptions (`descript.ion`, `files.bbs`) | `Ctrl+Z`, description columns, info panel | A whole small feature: a side file per folder, kept in step through copy, move, rename and delete, shown in two view modes and in the info panel, with its own settings (names to look for, encoding, read-only update). |
| ~~Archives~~ | Files menu (`Shift+F1`/`F2`) | **Shipped** for zip and the tar family (plain, gzip, bzip2, xz, zstd), all with pure-Rust decoders: `Enter` opens an archive as a panel, `F3` reads an entry, `F5` copies out through the job engine, `Shift+F1` builds an archive in any of the six formats and `Shift+F2` extracts one into a directory of its own. `Shift+F3` opens the archive menu — test, extract here, extract to, delete, add — and testing reads every entry through the format's own checks. Writing into an archive landed next (the row below). Still owed: an archive inside an archive, an archive on a remote filesystem, 7z and rar. |
| ~~Write into an archive~~ | `F4`, `F6`, `F8` inside one | **Shipped** for zip and the tar family: a change rebuilds the container as one transactional unit — the old archive is streamed into a `.part` file beside it with the change applied, the result is read back and verified, and only then does it replace the original, so an interrupted change leaves the archive untouched. Entries nobody touched are copied as the compressed bytes they already are. `F4` edits an entry and saves it back, `F6` renames or moves one inside the archive, `F8` takes entries out with their trees, and `F5`/`F6` into an archive put files in, one file at a time. Still owed: an entry's permissions and times, and `F7` inside an archive. |
| An archive inside an archive | — | SPEC 04 §4's nesting. Needs the inner container copied to a temporary file first, with the depth limit (`[arc] max_depth`) already in place to stop the recursion. |
| An archive on a server | — | Same shape as nesting: the container is materialised locally first, under the size limit, with a progress bar. Today an `sftp://` archive says it can only be read from a local disk. |
| 7z and rar | — | `sevenz-rust2` reads 7z; rar needs `unrar`, whose licence is not free, so it is an optional feature off by default in distribution builds. |
| ~~Archive commands menu, test an archive~~ | `Shift+F3` | **Shipped**: a menu of what can be done to the archive under the cursor, and a verify pass that reads every entry through the format's own checks — a zip's CRC, a decompressor's framing — and writes nothing. It reports as a job, so it has a progress bar, Esc stops it, and the entries that failed are listed with the reason each one gave. |

## Command line and history

| Item | Where | Notes |
| --- | --- | --- |
| ~~The command line itself~~ | SPEC 06 §7 | **Shipped**: internal commands, prefixes, metasymbols, conditions, history, and `Ctrl+O` for the user screen. |
| ~~Command history~~ | Commands menu (`Alt+F8`) | **Shipped**, and kept between sessions. |
| View and edit history | Commands menu (`Alt+F11`) | Needs the "keep history between sessions" store below. |
| ~~Keep history between sessions~~ | System settings | **Shipped** for commands, folders and both panels' directories. View/edit history still needs the list. |

## Histories and screens

| Item | Where | Notes |
| --- | --- | --- |
| Four histories with a size and a lifetime | commands, folders, view/edit, dialogs | FAR keeps each with its own count and a lifetime in days, saves them between sessions, and lets an item be **locked** so new entries never push it out. Ours is one folder list, in memory. |
| Screens list and background screens | `F12` | Several viewers, editors and jobs open at once, a list to switch between them, and the count in the panel corner. The job engine already runs more than one job; the screen stack is what is missing. |
| Task list | `Ctrl+W` | Worth being precise, because the name misleads: FAR's task list is the *operating system's* processes (PID, window title, `Del` kills one), not FAR's own windows — those are the screens list above. On Linux and macOS this is a process panel, closer to FAR's `proclist` plugin than to the core, so it belongs in a plugin once there are plugins. |
| ~~User screen~~ | `Ctrl+O` | **Shipped**, scrollable and capped, with each command as its heading. Peeking while a command runs is still to come. |
| Screen grabber | `Alt+Ins` | Select any region of the screen, stream or block, extend, move, append to the clipboard. Small, self-contained, and oddly useful. |

| Screen to screen without the panels | `Ctrl+Tab` | SPEC 02 §10. The ring steps out to the panels between screens, because the screens shadow the application's mode rather than owning it. A ring that walked screen to screen wants `screens: Vec<Mode>` with the front swapped in and out, which means every `self.mode = Mode::Normal` that closes a viewer has to say which screen it is closing. Worth doing when something else needs that list anyway — the task list (`Ctrl+W`) probably will. |

## Binary viewer and hex editor

What a dedicated binary tool does and we do not (SPEC 05 §1.8–§1.10 and §2.5–§2.8). The order
here is the order to build it in: each item needs only the ones above it.

| Item | Where | Notes |
| --- | --- | --- |
| ~~Structure pane~~ | `F11` in the viewer | **Shipped.** ELF, PE, Mach-O (thin and universal) and `ar`: identity, the header field by field with its raw bytes, sections, segments, load commands, archive members, interpreter and build id, with `Enter` taking the bytes to an offset. Written here rather than taken from `object`, because the pane needs the raw bytes beside the meaning, has to survive a hostile or truncated file, and must read only the offsets that matter. Imports and exports (ELF `DT_NEEDED` and `.dynsym` with bindings and types, the PE import and export directories including forwarders and exports with no name, Mach-O `LC_LOAD_DYLIB` and `LC_SYMTAB`), the symbol table feeding names into the disassembly, and the .NET CLI header with the metadata root all shipped too. |
| ~~Following a branch~~ | `Enter` in the disassembly | **Shipped**, with `Backspace` back and the jumped-from stack on screen. |
| ~~Hex mode of the editor~~ | `F4` on a binary | **Shipped (0.0.8).** SPEC 05 §2.5. Deliberately in the editor, not the viewer: the viewer is read-only by contract and that is what lets it seek through a 40 GB file. Overwrite and insert, a patch list, the editor's own undo and an atomic save. |
| ~~Blocks~~ | `F3` marks, `F12` operates | **Shipped (0.0.8).** SPEC 05 §2.6: write out, read in, fill, XOR, copy and move from elsewhere in the file, insert, delete — each with the resulting length shown before it happens. |
| ~~Assembler~~ | `F9` | **Shipped (0.0.8).** SPEC 05 §2.8: one instruction at a time into the patch list, decoded again before the dialog shows it. Registers, numbers and branches written as the address to arrive at. Memory operands and the vector sets are refused by name — a full ModRM and SIB parser is its own piece of work, and is the line below. |
| Memory operands in the assembler | `F9` | SPEC 05 §2.8. `mov [rbx+rcx*4+8], eax`: a ModRM and SIB parser, with the addressing-mode mistakes that come with it. Worth doing only with a test corpus to assemble against, because a wrong address written into a binary is the one failure this tool must not have. |
| ~~A calculator~~ | `Alt+F12` | **Shipped (0.0.8).** One line answered in decimal, hex, octal, binary and as characters at once, with the bitwise and shift operators and literals in every base the tools around it use. |
| Crypt / decrypt a block | `F12` on a block | As a block operation (XOR, and a named cipher with a key), never as a promise of security — the dialog has to say what it is and what it is not. Follows the block operations above. |
| ~~Search and replace across files~~ | find tool | **Shipped.** Its own job, over the files selected in the panel or the one under the cursor — a find result panelised with `F5` included, which is how it joins up with the find tool. The dry run is not optional: the confirmation counts the files and the matches by running the same engine that then writes, so the two cannot disagree. Hiew does this across multiple files, and it was the one thing on its list that belongs to the *panel* rather than to the viewer. |
| ARM / AArch64 disassembly | `F5` | A decoder crate, not new design — and on an Apple Silicon machine a viewer that only decodes x86 is half a tool. |
| Raw devices on Windows | hex pane | SPEC 05 §1.10. **Linux and macOS shipped (0.0.8)**: a block device is a file, its length is measured with a seek rather than read from `stat`, and it opens read-only. Windows needs `\\.\PhysicalDrive0` handling, where neither the open nor the seek behaves the same way. |
| More executable formats | structure pane | NE, LE, LX, TE/TE64 — the DOS and EFI families. Rainy-day additions to the same pane now that ELF, PE and Mach-O are solid. Netware's NLM/DSK/LAN, which Hiew also reads, are *not* planned: there is no living platform behind them, and if anyone asks they belong in a plugin. |

## Viewer and editor gaps

| Item | Where | Notes |
| --- | --- | --- |
| ~~Viewer ⇄ editor at the same position~~ | `F6` | **Shipped** both ways; the editor asks you to save first, because the viewer reads the file on disk. |
| ~~Jump to the open file in the panel~~ | `Ctrl+F10` | **Shipped.** |
| ~~Next and previous file of the panel from inside the viewer~~ | `+` / `−` | **Shipped**, skipping directories and keeping the mode. |
| Remember per file: position, code page, view mode, wrap, bookmarks | viewer and editor settings | Wants the persistent store the histories need. |
| Select and copy text in the viewer | mouse, `Ctrl+Ins` | Needs the OS clipboard. |
| Key bar, status line and scrollbar toggles | `Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S` | The viewer carries the three flags; the drawing and the keys are not wired. `Ctrl+B` is page-up in our viewer (a vi habit), so that one needs a decision rather than a binding. |
| Editor: append block to the clipboard, `Ctrl+Q` literal character code, `Ctrl+N`/`Ctrl+E` screen start and end, `Alt+U`/`Alt+I` indenting the current line when nothing is selected | editor | Small, and each one is somebody's habit. |
| Editor: show whitespace, cursor beyond end of line, select the found text, cursor at the end of it | editor settings | |
| Editor: three tab policies | editor settings | Don't expand, expand newly typed only, expand everything on open. We have the first two. |
| Code pages menu: favourites, a configurable `F8` cycle, custom names | `Shift+F8` | Ours cycles a fixed list. |

## Tools

| Item | Where | Notes |
| --- | --- | --- |
| ~~Find file~~ | Commands menu (`Alt+F7`) | **Shipped** in the shape SPEC 06 §1b describes: masks in the filter grammar, text or a hex pattern, case, whole words, "not containing", five scopes, follow symlinks, folders in the results, and results that stream into a list usable while the walk runs — go to, view, panelize, stop without losing what was found. Still owed, from FAR's dialog: a sixth scope, one code page or a chosen set, fuzzy matching, searching inside archives, alternate streams, the advanced page (a first-N-bytes limit, result columns in the column grammar), saving and loading a search, and a new search from the results. |
| ~~A temporary panel~~ | — | **Shipped.** One method decides a row's real path, so every operation, the viewer, the editor and the metasymbols work on a list panel; it re-reads by asking about its own paths again, and navigating away turns it back into a directory. |
| Find folder in the tree | `Alt+F10` | Wants the tree cache. |
| User menu | Commands menu (`F2`) | SPEC 06 §4: `menu.toml` with conditions. |
| File associations | Commands menu | SPEC 06 §6: what `Enter` does to a `.md`, a `.zip`, a `.wasm`. |
| ~~Folder shortcuts~~ | Commands menu (`Ctrl+D`) | **Shipped.** Ten of them, FAR's own number: `Ctrl+Shift+0..9` puts this panel's directory on a digit and `Alt+0..9` goes to it, since no terminal can tell FAR's two control keys apart. `Ctrl+D` opens the whole hotlist, where `Ins` keeps a folder with no digit, `Del` gives one up, `F4` names it and the digit is the row's own key. Kept in the state file rather than in `scmd.toml`, because a key sets them and a shortcut that waited for *Save setup* would be a bug. A shortcut on a server is a connection first, the same route the drive list takes. |

## Interface and configuration

| Item | Where | Notes |
| --- | --- | --- |
| ~~A status line inside each panel~~ | panel | **Shipped** behind `panel.status_line`, off by default because the screen-wide status bar says the same thing. |
| Free space in the panel frame, a scrollbar, the background-screen counter, a clock | panel | **Free space and the scrollbar shipped**, each behind a setting. The counter needs the screens list; the clock needs a repaint on a timer. |
| The six view modes we do not have | `LeftCtrl+1…0` | Medium, descriptions, long descriptions, owners, links, alternative full. Three of them need data we do not read yet (descriptions, owners, link counts). |
| A settings browser | `far:config` equivalent | FAR has 311 settings and a searchable editor for all of them; we have ~45 and `--dump-config`. The browser matters more as the count grows. |
| Colour groups and a theme editor | Options menu | Per-element colour editing with 16-colour, 256-colour and RGB pickers, on top of the themes we already load. |
| Auto-save setup on exit | System settings | Including both panels' folders, so the next start is where the last one ended. |
| Sorting collation | System settings | FAR splits what we fold into one `natural_sort`: collation (ordinal / invariant / linguistic), digits-as-numbers, case sensitivity. On three platforms this is a real decision — ICU or not. |
| Menus: filter as you type, lock the filter, horizontal scrolling | `RAlt`, `Ctrl+Alt+L`, `Alt+←/→` | Our menus are short today; this becomes necessary with plugin and macro lists. |
| Dialogs: history in edit fields with lockable items, autocomplete, three-state checkboxes, move with `Ctrl+F5`, click outside to accept or cancel | dialog framework | The session editor gave us text rows (SPEC 10 §3.5); these are the next layers. |
| Mouse wheel everywhere, with a per-area line count, and horizontal wheel | — | Comes with mouse support. |
| Clock, screen saver, window title variables | Interface settings | Each needs a repaint on a timer, which the input-driven event loop does not do yet. |
| ~~Macros~~ | `Ctrl+.` to record | **Shipped (0.1.0).** SPEC 08 §5: a macro is a list of commands in a readable TOML file, recorded by pressing keys and played by the key one assigns it to, with `once` and `each-selected` play modes, a list on `Ctrl+Shift+.`, and a question before anything destructive unless the file trusts it. |
| Macro conditions, and the Lua engine | `Ctrl+.` | SPEC 08 §5 and §2. FAR's macros carry a condition (play only when the panel is a file panel, only when something is selected) and far2m's are Lua. Both want the same thing underneath — a macro step that can ask the application a question — which is the plugin host API's job, so they come together with it. FAR's fourteen areas are also finer than our five scopes; worth splitting when something needs it. |
| XLat | — | Transliterate a typed line between keyboard layouts, with tables, rules and flags. Niche outside Cyrillic keyboards, cheap once the tables exist. |

## Settings not yet in the configuration

| Item | Where | Notes |
| --- | --- | --- |
| Drive enumeration through the platform | Change drive (`Alt+F1`/`Alt+F2`) | The list is built with `std` today: `/` and `$HOME` on Unix, and on Windows the letters `A:`–`Z:` that answer `Path::exists`. `GetLogicalDrives` would be one call instead of twenty-six, and would also give the volume label and the free space FAR shows. |
| Use the system recycle bin | System settings | Needs a platform crate (`SHFileOperation`, `~/.local/share/Trash`, `NSFileManager`). |
| Use the system copy routine | System settings | `copy_file_range`, `CopyFileEx`, `fcopyfile` — a speed-up with its own failure modes, so it stays behind a setting. |
| Clock in the corner | Interface settings | Wants a repaint on a timer; the event loop is input-driven today. |
| Mouse support | Interface settings | SPEC 02 §10: click to focus, click to select, wheel to scroll, drag to size the columns. |
| Always show the menu bar | Interface settings | Costs one screen row; trivial once there is a setting for it. |
| Viewer settings, Editor settings | Options menu | The settings exist in `scmd.toml`; the dialogs do not. |
| Code pages | Options menu | The viewer already cycles encodings with `F8`; this is the list-and-pick version. |
| Files highlighting | Options menu | Colour by mask/attribute, FAR's `[Colors]` groups. |
| Plugins configuration | Options menu | SPEC 08: there is nothing to configure until plugins load from disk. |

## Netbox

| Item | Where | Notes |
| --- | --- | --- |
| Proxy / `ProxyJump` in the session editor | Connections (`Alt+N` → `F4`) | The session editor stores host, port, user, authentication, remote folder, group, keep-alive and compression. FAR's NetBox also has proxy settings; `ProxyJump` is not implemented in the transport yet, so the dialog does not offer it. |
| FTP / FTPS / WebDAV | SPEC 04 §6 | The provider trait and the router are in place; each protocol is then its own crate behind a feature. S3 shipped as `scmd-s3`, read and write. |
| Multipart upload | — | One `PUT` is one object, which AWS caps at five gigabytes; above that an upload has to be cut into parts and completed with a manifest. Until it exists the service's own refusal is what says so. The same mechanism is what a resumable upload would need. |
| Known-hosts editing | — | Accepting and changing a host key from inside the program, instead of editing the file. |

## Dialog framework

| Item | Notes |
| --- | --- |
| A field that browses for a path | The key-file row in the session editor is typed by hand. A "pick a file" button wants the panel to open inside a dialog. |
| Radio groups, dialog-wide `Tab` order | Needed as soon as a dialog has more than one kind of control. |
| A different key bar inside dialogs | FAR changes the bottom row in a dialog. Cheap, cosmetic, not done. |

## Windows-specific: what we do instead

FAR is a Windows program and some of its features have no cross-platform meaning. These are not
backlog items; they are decisions already taken.

| FAR | Our answer |
| --- | --- |
| Short (8.3) names, `Ctrl+N` | Dropped. No equivalent on Unix, and dying on Windows. |
| NTFS alternate data streams (columns, search, copy warnings) | Dropped from the core; a provider may expose them as entries if it wants to. |
| SUBST drives, VHD attach/detach, USB safe removal, CD eject, volume labels | The drive list shows what the platform can tell us (roots, mount points, connections). The rest belongs in a platform plugin, if ever. |
| Access-rights policy on copy (default / copy / inherit) | Unix permissions and ACLs are a different model; `operations.preserve` lists what we carry over, per platform. |
| Elevation ("request administrator rights") | A privileged helper is a security design, not a setting. Out of scope until someone needs it. |
| The OS context menu (`emenu`), drag and drop with the shell | A plugin's job on each platform, not the core's. |
| OEM (FAR 1.x) plugin support, console icon, ClearType redraw | Not applicable. |
| 12-hour time, `%USERDOMAIN%`-style title variables | The date format follows the locale; the title shows path and version. |

## Keymap divergences to settle

* `Ctrl+L` — ours: change directory; FAR: info panel.
* `Ctrl+F12` — ours: open the sort list (FAR's behaviour); reversing moved to `Ctrl+Shift+F12`.
* `Ctrl+M` — FAR's "restore selection" is `Ctrl+Shift+M` here, because a terminal delivers
  `Ctrl+M` as `Enter`. This one will not change.
