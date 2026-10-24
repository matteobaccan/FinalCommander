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
| Info panel | Left/Right menu | SPEC 02 §7. FAR puts it on `Ctrl+L`, which we currently use for "change directory". **Open question:** move `nav.goto.path` to `Ctrl+\` (FAR's key) when the info panel lands, or give the info panel another key. |
| Quick view | Left/Right menu (`Ctrl+Q`) | SPEC 02 §7. Needs the viewer to render into a panel-sized rect — the viewer already draws into a `Rect`, so this is mostly plumbing. |
| Tree panel | Left/Right menu (`Ctrl+T`) | SPEC 02 §8. Wants a cached directory tree per root; the cache is the work, not the drawing. |
| Panel on/off | Left/Right menu (`Ctrl+F1`/`Ctrl+F2`) | One panel full width. Touches the layout code that splits the screen. |
| Compare folders | Commands menu | SPEC 02 §9. FAR's version marks, in each panel, the files that the other panel does not have or has older — names, sizes and times only, no subfolders and no contents. Its `compare` plugin is the one that goes deeper (subfolders, contents), which is a good division for us too. |
| Panelize a result set | SPEC 06 §1 | Needs a virtual listing that is not a directory — the panel already takes a `Listing`, so the shape is there. |

## Selection and sorting

| Item | Where | Notes |
| --- | --- | --- |
| The selection matrix | panel | FAR's modifiers pick the criterion, not just the action: `Ctrl+Gray ±` for the extension under the cursor, `Alt+Gray ±` for the name, `Shift+Gray ±` for everything, `Ctrl+Gray *` inverting folders too and `Alt+Gray *` clearing them. We have the mask, all/none/invert and restore. The rest is an afternoon and it is muscle memory for a FAR user. |
| A `Select folders` setting | Panel settings | Decides whether the mask keys touch directories at all. One setting, three keys behave differently. |
| `Shift+F5`, `Shift+F6`, `Shift+F8` | panel | Copy, move and delete **the item under the cursor**, ignoring the selection. Cheap, and they prevent accidents. |
| Sort by creation time, access time, owner, description | `Ctrl+F8…F11` | `SortKey` needs the variants and the listing needs the fields; owner and description come with their own cost (a lookup per entry, a description file). |
| Sort criteria chains | sort modes list, `F4` | Per sort mode, an ordered set of tie-breakers with its own editor (`Ins`/`Del`/`F4`/`+`/`-`/`*`/`=`/`Ctrl+↑↓`/`Ctrl+R`). SPEC 07 already has `chain` in the config shape, so this is filling in a designed hole. |
| Sort groups | `Shift+F11` | Named mask groups that sort before or after everything else, ordered, with a separator marking "front" from "back". |
| Scroll long names sideways | `Alt+←/→`, `Alt+Home/End` | Needed as soon as the name column is narrower than the names. |

## Masks, filters, groups

| Item | Where | Notes |
| --- | --- | --- |
| Mask groups `<arc>`, `<temp>`, `<exec>` | Options menu | Named, nestable sets of masks, usable anywhere a mask is. The Options menu entry is specified (SPEC 10 §2.4) and the mask parser already needs extending for it. |
| The full mask grammar | everywhere | `[c-f]` ranges, `/regex/i` for a whole mask, `include|exclude` with one `|`, quoting a mask that contains a delimiter. Ours does `*`, `?` and lists. |
| Filters as a shared object | panel, find, copy | Named condition sets: mask, size range with binary and decimal suffixes, date range on any of four timestamps (absolute or relative in days), attributes as three-state include/exclude/ignore, "more than one hard link" — each condition separately switchable. FAR reuses one filter object for the panel, for find and for copy; building it once is the whole point. |
| Files highlighting | Options menu | Colour by mask and attribute group, with its own list and editor. Wants the filter object above. |

## Files

| Item | Where | Notes |
| --- | --- | --- |
| Create link | Files menu (`Alt+F6`) | Hard links and symlinks, local only to start with. |
| Wipe | Files menu (`Alt+Del`) | Overwrite before unlink. Honest naming matters: on SSDs and copy-on-write filesystems it cannot promise much, and the dialog should say so. |
| File attributes | Files menu | SPEC 03 §7. Dialog for mode bits, times, Windows attributes, and the SFTP equivalents. **Open question:** FAR uses `Ctrl+A`, which we gave to `sel.all`; probable answer is `Ctrl+A` for attributes and `Ctrl+Shift+A` for select-all, matching FAR more closely. |
| Apply command | Files menu (`Ctrl+G`) | Run a command line once per selected file, with `!.!`-style substitutions (SPEC 06 §5). Needs the command line first. |
| Copy: several destinations at once | copy dialog | `;`-separated, quoted when a name contains one. |
| Copy: append to the existing file, and "only newer" as a policy | copy dialog | We ask, overwrite, skip, rename and the `…All` variants; FAR also appends and can be told up front to copy only newer files. |
| Copy: contents of symbolic links instead of the links | copy dialog | A real decision with data-loss consequences either way, so it belongs in the dialog rather than in a default. |
| Copy: a filter, so only matching files travel | copy dialog | Wants the filter object above. An empty folder is not created when nothing in it matches. |
| Copy: destination from a folder tree | `F10`, `Alt+F10`, `Shift+F10` in the dialog | Wants the tree panel's cache. |
| Copy: average speed, elapsed and estimated time | progress dialog | We show the bars and the counters; the three numbers people actually watch are missing. |
| Clipboard: names, full names and the files themselves | `Ctrl+Ins`, `Alt+Shift+Ins`, `Ctrl+Shift+C/X` | Needs the OS clipboard (SPEC 06). Copying *files* to the clipboard so another application can paste them is platform work, not a one-liner. |
| Descriptions (`descript.ion`, `files.bbs`) | `Ctrl+Z`, description columns, info panel | A whole small feature: a side file per folder, kept in step through copy, move, rename and delete, shown in two view modes and in the info panel, with its own settings (names to look for, encoding, read-only update). |
| Archives | Files menu (`Shift+F1`/`F2`/`F3`) | SPEC 04 §4: the archive VFS provider (read first, write later) and the three dialogs. The biggest single item in this list. |

## Command line and history

| Item | Where | Notes |
| --- | --- | --- |
| The command line itself | SPEC 06 §5 | A shell line under the panels, with output capture (`Ctrl+O`). Everything below depends on it. |
| Command history | Commands menu (`Alt+F8`) | The list exists in the spec; the data does not until there is a command line. |
| View and edit history | Commands menu (`Alt+F11`) | Needs the "keep history between sessions" store below. |
| Keep history between sessions | System settings | A small on-disk store (folders, commands, viewed and edited files). Until it exists, folder history lives only in the running session. |

## Histories and screens

| Item | Where | Notes |
| --- | --- | --- |
| Four histories with a size and a lifetime | commands, folders, view/edit, dialogs | FAR keeps each with its own count and a lifetime in days, saves them between sessions, and lets an item be **locked** so new entries never push it out. Ours is one folder list, in memory. |
| Screens list and background screens | `F12` | Several viewers, editors and jobs open at once, a list to switch between them, and the count in the panel corner. The job engine already runs more than one job; the screen stack is what is missing. |
| Task list | `Ctrl+W` | Worth being precise, because the name misleads: FAR's task list is the *operating system's* processes (PID, window title, `Del` kills one), not FAR's own windows — those are the screens list above. On Linux and macOS this is a process panel, closer to FAR's `proclist` plugin than to the core, so it belongs in a plugin once there are plugins. |
| User screen | `Ctrl+O`, `Ctrl+Alt+Shift` to peek | Needs the command line first. |
| Screen grabber | `Alt+Ins` | Select any region of the screen, stream or block, extend, move, append to the clipboard. Small, self-contained, and oddly useful. |

## Viewer and editor gaps

| Item | Where | Notes |
| --- | --- | --- |
| Viewer ⇄ editor at the same position | `F6` | Both ways, keeping the byte offset. The two components already share a file source; this is bookkeeping. |
| Jump to the open file in the panel | `Ctrl+F10` | |
| Next and previous file of the panel from inside the viewer | `Gray +/-` | |
| Remember per file: position, code page, view mode, wrap, bookmarks | viewer and editor settings | Wants the persistent store the histories need. |
| Select and copy text in the viewer | mouse, `Ctrl+Ins` | Needs the OS clipboard. |
| Key bar, status line and scrollbar toggles | `Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S` | |
| Editor: append block to the clipboard, `Ctrl+Q` literal character code, `Ctrl+N`/`Ctrl+E` screen start and end, `Alt+U`/`Alt+I` indenting the current line when nothing is selected | editor | Small, and each one is somebody's habit. |
| Editor: show whitespace, cursor beyond end of line, select the found text, cursor at the end of it | editor settings | |
| Editor: three tab policies | editor settings | Don't expand, expand newly typed only, expand everything on open. We have the first two. |
| Code pages menu: favourites, a configurable `F8` cycle, custom names | `Shift+F8` | Ours cycles a fixed list. |

## Tools

| Item | Where | Notes |
| --- | --- | --- |
| Find file | Commands menu (`Alt+F7`) | SPEC 06 §1, and bigger than it sounds once FAR's dialog is the yardstick: masks, text or hex, case, whole words, fuzzy matching, "not containing", one code page or all or a chosen set, search inside archives, include folders, follow symlinks, alternate streams, six scopes, a filter, and an advanced page with a first-N-bytes limit and result columns in the column grammar. Results then need: new search, go to, view while the search continues behind it, panelize, stop. |
| A temporary panel | — | A panel made of an arbitrary file list, which is what "panelize" needs and what FAR ships as `tmppanel`. |
| Find folder in the tree | `Alt+F10` | Wants the tree cache. |
| User menu | Commands menu (`F2`) | SPEC 06 §4: `menu.toml` with conditions. |
| File associations | Commands menu | SPEC 06 §6: what `Enter` does to a `.md`, a `.zip`, a `.wasm`. |
| Folder shortcuts | Commands menu | FAR's `Ctrl+0..9`/`Right Ctrl+0..9` bookmarks. |

## Interface and configuration

| Item | Where | Notes |
| --- | --- | --- |
| A status line inside each panel | panel | FAR puts the item under the cursor in the panel's own bottom line, and keeps the screen's bottom rows for the command line and the key bar. We have one global status line instead. Worth settling together with the command line, since they compete for the same rows. |
| Free space in the panel frame, a scrollbar, the background-screen counter, a clock | panel | Small drawing jobs, except the clock, which needs a repaint on a timer. |
| The six view modes we do not have | `LeftCtrl+1…0` | Medium, descriptions, long descriptions, owners, links, alternative full. Three of them need data we do not read yet (descriptions, owners, link counts). |
| A settings browser | `far:config` equivalent | FAR has 311 settings and a searchable editor for all of them; we have ~45 and `--dump-config`. The browser matters more as the count grows. |
| Colour groups and a theme editor | Options menu | Per-element colour editing with 16-colour, 256-colour and RGB pickers, on top of the themes we already load. |
| Auto-save setup on exit | System settings | Including both panels' folders, so the next start is where the last one ended. |
| Sorting collation | System settings | FAR splits what we fold into one `natural_sort`: collation (ordinal / invariant / linguistic), digits-as-numbers, case sensitivity. On three platforms this is a real decision — ICU or not. |
| Menus: filter as you type, lock the filter, horizontal scrolling | `RAlt`, `Ctrl+Alt+L`, `Alt+←/→` | Our menus are short today; this becomes necessary with plugin and macro lists. |
| Dialogs: history in edit fields with lockable items, autocomplete, three-state checkboxes, move with `Ctrl+F5`, click outside to accept or cancel | dialog framework | The session editor gave us text rows (SPEC 10 §3.5); these are the next layers. |
| Mouse wheel everywhere, with a per-area line count, and horizontal wheel | — | Comes with mouse support. |
| Clock, screen saver, window title variables | Interface settings | Each needs a repaint on a timer, which the input-driven event loop does not do yet. |
| Macros | `Ctrl+.` to record, 14 areas | Record and play key sequences per area, hotkeys with double modifiers, two play modes, a list, and eventually the Lua engine (SPEC 08). The command vocabulary is already the right foundation: a macro is a list of commands, not a list of keystrokes. |
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
| FTP / FTPS / WebDAV / S3 | SPEC 04 §6 | The provider trait and the router are in place; each protocol is then its own crate behind a feature. |
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

* `Ctrl+A` — ours: select all; FAR: file attributes.
* `Ctrl+L` — ours: change directory; FAR: info panel.
* `Ctrl+F12` — ours: open the sort list (FAR's behaviour); reversing moved to `Ctrl+Shift+F12`.
* `Ctrl+M` — FAR's "restore selection" is `Ctrl+Shift+M` here, because a terminal delivers
  `Ctrl+M` as `Enter`. This one will not change.
