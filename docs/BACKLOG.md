# Backlog

What is specified but not implemented, in the order the menus mention it. Every dim entry in the
menu bar (SPEC 10) has a line here. "Open question" means a decision is needed before coding,
not that the work is large.

## Panels

| Item | Where | Notes |
| --- | --- | --- |
| Info panel | Left/Right menu | SPEC 02 §7. FAR puts it on `Ctrl+L`, which we currently use for "change directory". **Open question:** move `nav.goto.path` to `Ctrl+\` (FAR's key) when the info panel lands, or give the info panel another key. |
| Quick view | Left/Right menu (`Ctrl+Q`) | SPEC 02 §7. Needs the viewer to render into a panel-sized rect — the viewer already draws into a `Rect`, so this is mostly plumbing. |
| Tree panel | Left/Right menu (`Ctrl+T`) | SPEC 02 §8. Wants a cached directory tree per root; the cache is the work, not the drawing. |
| Panel on/off | Left/Right menu (`Ctrl+F1`/`Ctrl+F2`) | One panel full width. Touches the layout code that splits the screen. |
| Compare folders | Commands menu | SPEC 02 §9: mark what differs between the two panels. |
| Panelize a result set | SPEC 06 §1 | Needs a virtual listing that is not a directory — the panel already takes a `Listing`, so the shape is there. |

## Files

| Item | Where | Notes |
| --- | --- | --- |
| Create link | Files menu (`Alt+F6`) | Hard links and symlinks, local only to start with. |
| Wipe | Files menu (`Alt+Del`) | Overwrite before unlink. Honest naming matters: on SSDs and copy-on-write filesystems it cannot promise much, and the dialog should say so. |
| File attributes | Files menu | SPEC 03 §7. Dialog for mode bits, times, Windows attributes, and the SFTP equivalents. **Open question:** FAR uses `Ctrl+A`, which we gave to `sel.all`; probable answer is `Ctrl+A` for attributes and `Ctrl+Shift+A` for select-all, matching FAR more closely. |
| Apply command | Files menu (`Ctrl+G`) | Run a command line once per selected file, with `!.!`-style substitutions (SPEC 06 §5). Needs the command line first. |
| Archives | Files menu (`Shift+F1`/`F2`/`F3`) | SPEC 04 §4: the archive VFS provider (read first, write later) and the three dialogs. The biggest single item in this list. |

## Command line and history

| Item | Where | Notes |
| --- | --- | --- |
| The command line itself | SPEC 06 §5 | A shell line under the panels, with output capture (`Ctrl+O`). Everything below depends on it. |
| Command history | Commands menu (`Alt+F8`) | The list exists in the spec; the data does not until there is a command line. |
| View and edit history | Commands menu (`Alt+F11`) | Needs the "keep history between sessions" store below. |
| Keep history between sessions | System settings | A small on-disk store (folders, commands, viewed and edited files). Until it exists, folder history lives only in the running session. |

## Tools

| Item | Where | Notes |
| --- | --- | --- |
| Find file | Commands menu (`Alt+F7`) | SPEC 06 §1. The command exists and reports "not implemented"; the walker and the dialog do not. |
| User menu | Commands menu (`F2`) | SPEC 06 §4: `menu.toml` with conditions. |
| File associations | Commands menu | SPEC 06 §6: what `Enter` does to a `.md`, a `.zip`, a `.wasm`. |
| Folder shortcuts | Commands menu | FAR's `Ctrl+0..9`/`Right Ctrl+0..9` bookmarks. |
| Screens list, Task list | Commands menu (`F12`, `Ctrl+W`) | Several viewers/editors/jobs open at once, and a list to switch between them. The job engine already runs more than one job; what is missing is the screen stack. |

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

## Keymap divergences to settle

* `Ctrl+A` — ours: select all; FAR: file attributes.
* `Ctrl+L` — ours: change directory; FAR: info panel.
* `Ctrl+F12` — ours: open the sort list (FAR's behaviour); reversing moved to `Ctrl+Shift+F12`.
* `Ctrl+M` — FAR's "restore selection" is `Ctrl+Shift+M` here, because a terminal delivers
  `Ctrl+M` as `Enter`. This one will not change.
