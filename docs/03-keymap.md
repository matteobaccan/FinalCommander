# Keyboard reference and default keymap

Sources: FAR Manager's own help (`far/FarEng.hlf.m4`, BSD-3), the `mc(1)` man page (GPL-3
project, facts only), Krusader's handbook (`doc/handbook/keyboard-commands.docbook`), MTN2's
README, and third-party cheat sheets for Total Commander (proprietary — **secondary sources,
marked `?` where unverified**). All descriptions below are written from scratch as factual
key→action pairs; no documentation text is reproduced. See `01-licensing.md` §3.

Column `FC` = FinalCommander default (profile `far`, our baseline).

---

## 1. Function keys — plain

| Key | FAR | MC | TC | Krusader | FC |
| --- | --- | --- | --- | --- | --- |
| F1 | help (context) | help | help | help | help (context) |
| F2 | user menu | user menu | reread panel? | rename | user menu |
| F3 | view (internal/assoc.); on a dir: compute size | view | view | view | **always our viewer** / dir size; an association is `Alt+F3` |
| F4 | edit; on a dir: attributes dialog | edit | edit | edit | **always our editor**; an association is `Alt+F4` |
| F5 | copy | copy | copy | copy | copy |
| F6 | rename / move | rename/move | move/rename | move | move / rename |
| F7 | make directory | make directory | new folder | new folder | make directory |
| F8 | delete (to trash per settings) | delete | delete | delete/trash | **delete, to the system's wastebasket when `general.use_trash` says so (it does by default)** |
| F9 | menu bar | pull-down menus | left menu? | terminal | menu bar |
| F10 | quit | quit | exit? | quit | quit |
| F11 | plugin commands | — | — | — | plugin/command palette |
| F12 | — | screen list | custom filter? | — | panel/tab list |

## 2. Shift + F-keys

| Key | FAR | MC | TC `?` | Krusader | FC |
| --- | --- | --- | --- | --- | --- |
| Shift+F1 | add to archive | — | — | what's this | **add to archive** (zip, tar, tar.gz, tar.bz2, tar.xz, tar.zst) |
| Shift+F2 | extract from archive | — | compare by content | multi-rename | **extract archive** into a directory of its own; inside an archive it is `F5` |
| Shift+F3 | archive commands menu | — | — | view URL | **archive commands**: test, extract here, extract to, delete, add |
| Shift+F4 | edit a new file | edit new file | create new text file | edit new file | edit new file |
| Shift+F5 | copy the item under the cursor (same panel) | — | copy in same panel | copy by queue | copy here |
| Shift+F6 | rename/move the item under the cursor | — | rename in place | move by queue | rename in place |
| Shift+F8 | delete only the item under the cursor | — | delete permanently | — | delete under cursor |
| Shift+F9 | save configuration | — | — | — | save configuration |
| Shift+F10 | select the last used item of the user menu | — | — | view all files | the user menu is there (`F2`); remembering which row was taken last is not, so this stays unbound |
| Shift+F11 | sort groups on/off (named mask groups that sort ahead of or behind everything else — not the same as "directories first") | — | — | — | **directories first on/off**, which is the part of FAR's `Group` we have; named sort groups are still to come |
| Shift+F12 | selected files first | — | — | custom view | **selected files first on/off** |
| Shift+Del | delete skipping trash | delete | delete permanently | delete permanently | **delete permanently**, past the wastebasket |
| Shift+Enter | run in a separate window / OS open | — | run in own window | — | OS-open item |
| Shift+arrows | extend selection | extend selection | extend selection | extend selection | extend selection |

## 3. Alt + F-keys

| Key | FAR | TC `?` | Krusader | FC |
| --- | --- | --- | --- | --- |
| Alt+F1 | drive menu, left panel | drive left | left bookmarks | drive/mount menu, left |
| Alt+F2 | drive menu, right panel | drive right | right bookmarks | drive/mount menu, right |
| Alt+F3 | alternative (external/internal) viewer | alternative viewer | — | **what an association's `view` names, else `$PAGER`** |
| Alt+F4 | alternative editor | exit | — | **what an association's `edit` names, else `$VISUAL`/`$EDITOR`** |
| Alt+F5 | print | pack | — | print / export list |
| Alt+F6 | create hard/symlink | — | — | create link |
| Alt+F7 | find files | search | — | find files |
| Alt+F8 | command history | command-line history | — | command history |
| Alt+F9 | toggle window size (console) | — | — | toggle maximise/zoom panel |
| Alt+F10 | find folder | directory tree | — | find folder / tree |
| Alt+F11 | view & edit history | — | — | **the files read or edited**, newest first: `Enter` reads one, `F4` edits it (SPEC 06 §6) |
| Alt+F12 | folder history | — | — | folder history |
| Alt+Shift+F9 | plugin configuration | — | — | plugin settings |
| Alt+Del | wipe (secure delete) | — | — | wipe |
| Alt+Ins | screen grabber | — | — | (unbound) |
| Alt+Enter | — | properties | properties | properties |
| Alt+F1..F12 (KDE) | — | — | reserved by the desktop | — |

## 4. Ctrl + F-keys (panel modes and sorting)

| Key | FAR | TC `?` | FC |
| --- | --- | --- | --- |
| Ctrl+F1 | hide/show left panel | brief view | hide/show left panel |
| Ctrl+F2 | hide/show right panel | detailed view | hide/show right panel |
| Ctrl+F3 | sort by name | sort by name | sort by name |
| Ctrl+F4 | sort by extension | sort by extension | sort by extension |
| Ctrl+F5 | sort by modification time | sort by date | sort by mtime |
| Ctrl+F6 | sort by size | sort by size | sort by size |
| Ctrl+F7 | unsorted | unsorted | unsorted |
| Ctrl+F8 | sort by creation time | — | sort by ctime/birth |
| Ctrl+F9 | sort by access time | — | sort by atime |
| Ctrl+F10 | sort by description | show all files | sort by description |
| Ctrl+F11 | sort by owner | — | sort by owner |
| Ctrl+F12 | sort modes menu | custom filter | **sort modes list** (re-choosing the current order reverses it); `Ctrl+Shift+F12` reverses directly |
| Ctrl+Shift+F3 / F4 | always internal viewer / editor | — | force internal viewer / editor |

## 5. Panel control (FAR as baseline, verified)

| Action | FAR | MC | Krusader | FC |
| --- | --- | --- | --- | --- |
| switch panel | Tab | Tab, Ctrl+I, ←/→ | Tab | Tab |
| swap panels | Ctrl+U | Ctrl+U | Ctrl+U | Ctrl+U |
| re-read panel | Ctrl+R | Ctrl+R | Ctrl+R | Ctrl+R |
| info panel | Ctrl+L | Ctrl+x i | Alt+↓ (sidebar) | Ctrl+L |
| quick view panel | Ctrl+Q | Ctrl+x q | — | Ctrl+Q |
| tree panel | Ctrl+T | — | — | Ctrl+T |
| hide both panels | Ctrl+O | Ctrl+O | — | Ctrl+O |
| hide inactive panel | Ctrl+P | — | — | Ctrl+P (the focus stays where it is) |
| panel height / width | Ctrl+↑/↓, Ctrl+←/→ | — | — | Ctrl+↑/↓, Ctrl+←/→ — `Ctrl+↑` brings the panels' bottom edge **up**, so they get shorter and the command output below them gets the rows |
| toggle hidden files | Ctrl+H | Alt+. (Shift+Alt+H in some builds) | Alt+. | Ctrl+H |
| short/long names | Ctrl+N | Alt+T (cycle formats) | Alt+Shift+B/D | Ctrl+N |
| key bar on/off | Ctrl+B | — | — | Ctrl+B |
| font larger / smaller / as it was | — | — | — | **Ctrl++ / Ctrl+− / Ctrl+0**, as every graphical program does it; §9 says what a terminal program can actually do about its font |
| view modes 1..0 | LeftCtrl+1..0 | Alt+T cycles | Alt+Shift+B/D | **Ctrl+1..5** in FAR's own order: brief, medium, full, wide, detailed, and **Ctrl+8** for the file owners — name, size, owner, group (SPEC 02 §3). The three FAR has in between — descriptions, long descriptions, file links — and its alternative full view want data we do not collect yet; `docs/BACKLOG.md` says so |
| parent directory | Ctrl+PgUp | Ctrl+PgUp | Ctrl+PgUp | Ctrl+PgUp, Backspace |
| enter dir / archive | Ctrl+PgDn | Ctrl+PgDn | Enter | Ctrl+PgDn |
| root directory | Ctrl+\ | — | Ctrl+Backspace | Ctrl+\ |
| same dir in other panel | — | Alt+I | Ctrl+= | Ctrl+= |
| other panel follows cursor | — | Alt+O | Ctrl+←/→ | Left/Right menu, no default key — `Alt+<letter>` is the quick search here |
| directory hotlist/bookmarks | folder shortcuts: Ctrl+Shift+0..9 set, RightCtrl+0..9 go (and insert the path when in an edit line), plus a list in the Commands menu | Ctrl+\ | Ctrl+D / Ctrl+B add | **Ctrl+D** the list; **Ctrl+Shift+0..9** set; **Alt+0..9** go |
| directory history | Alt+F12 | Alt+Shift+H; Alt+Y / Alt+U back/forward | Ctrl+H; Alt+←/→ | Alt+F12; Alt+←/→ |
| quick cd / path entry | — | Alt+C | Ctrl+L | Ctrl+L |
| new tab / close tab | — | — | Ctrl+Alt+N / Ctrl+W | **Ctrl+Alt+T / Ctrl+Alt+W** |
| next/prev tab | — | — | Alt+←/→ (tabs: Shift+←/→) | **Ctrl+Alt+N / Ctrl+Alt+P** |
| move the tab to the other panel | — | — | — | **Ctrl+Alt+M** |
| lock the tab to its directory | — | — | — | **Ctrl+Alt+K** |
| name the tab | — | — | — | Left/Right menu |
| find file | Alt+F7 | Alt+? | Ctrl+S | Alt+F7 |
| disk usage | — | Ctrl+Space on a dir | Alt+Shift+S | `F3` on a directory computes its size; a report over a whole tree is on the backlog |
| compare directories | Commands menu, no default key (marks what is missing or newer in the other panel; names, sizes and times only, no subfolders) | — | Alt+Shift+C | Commands menu, no default key, as in FAR |
| synchronise directories | — | — | Ctrl+Y | **Ctrl+Shift+Y** — Total Commander's `Ctrl+Y` is FAR's *clear the command line*, and `--check-keymap` is what said so |
| pack / unpack | Shift+F1 / Shift+F2 | — | Alt+Shift+P / Alt+Shift+U | Shift+F1 / Shift+F2 |
| test archive | — | — | Alt+Shift+E | Commands menu, no default key |
| multi-rename | — | — | Shift+F2 | not built yet — `docs/BACKLOG.md` (and FAR's own `Ctrl+M` restores the previous selection, which is what it does here too, on `Ctrl+Shift+M`) |
| select group / unselect group | Gray + / Gray − | + / \ | Ctrl++ / Ctrl+− | + / − |
| invert selection | Gray * | — | Alt+* | `*` (files), `Alt+*` (folders too, FAR's `Ctrl+Gray *`) |
| select all / none | Shift+Gray + / Shift+Gray − | — | Alt++ / Alt+− | **Shift++ / Shift+−**, and `Ctrl+Shift+A` for *all* where a terminal does not report Shift with a printable key |
| by extension of the current file | Ctrl+Gray + / − | — | — | **Ctrl+Shift++ / Ctrl+Shift+−** (FAR's own `Ctrl++` is the font, below), and the Select menu |
| by name, any extension | Alt+Gray + / − | — | — | **Alt++ / Alt+−** |
| scroll long names sideways | Alt+←/→, Alt+Home/End | — | — | **Ctrl+Shift+←/→** (Alt+←/→ is history here) |
| restore previous selection | Ctrl+M | — | — | Ctrl+Shift+M |
| copy names to clipboard | Ctrl+Ins / Ctrl+Shift+Ins names, Alt+Shift+Ins full names, Ctrl+Alt+Ins real/UNC names, Ctrl+Shift+C / Ctrl+Shift+X copy/cut the files themselves | — | — | **`Ctrl+Ins` names, `Alt+Shift+Ins` full paths**, through the terminal with `OSC 52` (SPEC 06 §11) — so it also works over `ssh`. `Ctrl+Ins` copies the command line instead while there is something on it. Copying the files themselves to a clipboard another program can paste from is not a thing a terminal can be asked to do: that stays a backlog line |
| attributes / permissions | Ctrl+A | Ctrl+x c (chmod), Ctrl+x o (chown) | Alt+Enter | Ctrl+A |
| apply command to selection | Ctrl+G | — | — | Ctrl+G |
| describe files | Ctrl+Z | — | — | Ctrl+Shift+D |
| toggle terminal pane | — | Ctrl+O | Ctrl+Alt+E / Ctrl+Shift+↑ | Ctrl+Alt+E |
| command palette | — | — | — | **Ctrl+Shift+P** (new) |

Notes on FAR specifics we cannot reproduce verbatim in a terminal: `LeftCtrl` vs `RightCtrl`
(FAR uses them for *set* vs *go to* bookmarks) and numpad "gray" keys are generally
indistinguishable in a TTY — see §9.

## 6. Command line

| Action | FAR | MC | FC |
| --- | --- | --- | --- |
| char left/right | ←/→, Ctrl+S / Ctrl+D | ←/→ | ←/→ |
| word left/right | Ctrl+←/→ | Alt+B / Alt+F | Ctrl+←/→ |
| start/end of line | Ctrl+Home / Ctrl+End | Ctrl+A / Ctrl+E | Home / End |
| delete word left/right | Ctrl+Backspace / Ctrl+Del | Alt+Backspace / Alt+D | Ctrl+Backspace / Ctrl+Del, and `Ctrl+W` for the left one as every readline has it |
| delete to end of line | Ctrl+K | Ctrl+K | Ctrl+K |
| clear line | Ctrl+Y | Ctrl+U | `Esc` — `Ctrl+U` is FAR's swap the panels, and it is that here |
| history prev/next | Ctrl+E / Ctrl+X | Alt+P / Alt+N | ↑/↓ (when non-empty), Alt+F8 list |
| insert current file name | Ctrl+J, Ctrl+Enter | Alt+Enter | Ctrl+Enter |
| insert name from other panel | Ctrl+Shift+Enter | — | Ctrl+Shift+Enter |
| insert the panel's path (this one / the other) | Ctrl+F / Ctrl+; | — | Ctrl+[ / Ctrl+] |
| completion | — | Tab (also Alt+Tab list) | Tab |
| run keeping output | — | Ctrl+O after run | Ctrl+O |

## 7. Viewer

| Action | FAR | MC | FC |
| --- | --- | --- | --- |
| line / page / file bounds | ↑↓, PgUp/PgDn, Home/End | same | same |
| horizontal scroll | ←/→, Ctrl+←/→ (20 cols) | ←/→ | ←/→, Ctrl+←/→ |
| wrap toggle | F2 (wrap), Shift+F2 (word wrap) | F2 | F2 / Shift+F2 |
| hex mode | F4; Shift+F4 mode menu (text/hex/dump) | F4 | F4 / Shift+F4 |
| a tabular file as a table | — | — | **Shift+F3** for a `.csv`, `.tsv` or `.jsonl`: a heading that stays put, columns, one record a row (SPEC 05 §1.11). The same key goes back to the source, and `Shift+F4` has it in the cycle |
| switch to editor | F6 | — | F6 |
| search / next / previous | F7 / Shift+F7 or Space / Alt+F7 | F7 / Ctrl+S, Ctrl+R | F7 / n / N (and Shift+F7/Alt+F7) |
| codepage | F8 toggle, Shift+F8 menu | Alt+E | Shift+F8 |
| go to position | Alt+F8 | Alt+G / m | Alt+F8, `:`|
| bookmarks set / go | Ctrl+Shift+0..9 / LeftCtrl+0..9 | — | Ctrl+Shift+0..9 / Alt+0..9 |
| copy selection | Ctrl+Ins, Ctrl+C | — | Ctrl+C |
| next / previous file in panel | Gray + / Gray − | — | + / − |
| key bar / top line / scrollbar | Ctrl+B / Ctrl+Shift+B / Ctrl+S | — | **the same three.** `Ctrl+B` was one of three spellings of page-up here (a vi habit); `PgUp` and `Ctrl+Up` still page up, so FAR's key went to FAR's feature. A key changes the file being read; `[viewer]` says what every file opens with |
| quit | Esc, F3, F10, Numpad5 | Esc, F10, q | Esc, F3, F10, q |
| follow (tail) | End then auto-append | — | Shift+F | 

## 8. Editor

| Action | FAR | MC (mcedit) | FC |
| --- | --- | --- | --- |
| save / save as | F2 / Shift+F2 | F2 / — | F2 / Shift+F2 |
| quit / save+quit | F10, F4, Esc / Shift+F10 | F10 | F10, Esc / Shift+F10 |
| switch to viewer | F6 | — | F6 |
| search / replace | F7 / Ctrl+F7 | F7 / F4 | F7 / Ctrl+F7 |
| search next / previous | Shift+F7 / Alt+F7 | F17 (Shift+F7) | n / N, Shift+F7 |
| go to line:col | Alt+F8 | Alt+L | Alt+F8, `:` |
| undo / redo | Alt+Backspace, Ctrl+Z / Ctrl+Shift+Z | Ctrl+U | Ctrl+Z / Ctrl+Y, Ctrl+Shift+Z |
| select block | Shift+arrows, Ctrl+Shift+arrows | F3 marks | Shift+arrows |
| **column (vertical) block** | Alt+arrows, Alt+Shift+arrows | — | Alt+Shift+arrows |
| select all / deselect | Ctrl+A / Ctrl+U | — | Ctrl+A / Ctrl+U |
| cut / copy / paste | Shift+Del, Ctrl+X / Ctrl+Ins, Ctrl+C / Shift+Ins, Ctrl+V | F6/F5/F7-ish | Ctrl+X / Ctrl+C / Ctrl+V |
| append to clipboard | Ctrl+Gray + | — | **Ctrl+Shift+Ins** |
| copy / move block to cursor | Ctrl+P / Ctrl+M (persistent blocks) | F5 / F6 | Ctrl+Shift+P / Ctrl+Shift+M |
| delete block / line / to EOL | Ctrl+D / Ctrl+Y / Ctrl+K | F8 / Ctrl+Y / Ctrl+K | Ctrl+D / Ctrl+Y / Ctrl+K |
| delete word left / right | Ctrl+Backspace / Ctrl+T, Ctrl+Del | Alt+Backspace | Ctrl+Backspace / Ctrl+Del, Ctrl+T |
| indent / outdent block | Alt+I / Alt+U | — | **Alt+I / Alt+U**, which move the block — or the line the cursor is on when nothing is selected. `Tab` types a tab here and does not indent a block: it is the key one presses while writing, and a key that did both would do the wrong one half the time |
| start/end of file | Ctrl+Home, Ctrl+PgUp / Ctrl+End, Ctrl+PgDn | Ctrl+Home / Ctrl+End | Ctrl+Home / Ctrl+End |
| start/end of screen | Ctrl+N / Ctrl+E | — | **Ctrl+N / Ctrl+E** |
| scroll screen | Ctrl+↑ / Ctrl+↓ | — | Ctrl+↑ / Ctrl+↓ |
| bookmarks set / go | Ctrl+Shift+0..9 / LeftCtrl+0..9 | — | Ctrl+Shift+0..9 / Alt+0..9 |
| codepage toggle / menu | F8 / Shift+F8 | — | Shift+F8 |
| read-only lock | Ctrl+L | — | Ctrl+Alt+L |
| literal next key (char code) | Ctrl+Q | — | **Ctrl+Q** — a control character, or a tab where tabs are expanded |
| insert name of current file | Shift+Enter / Ctrl+Shift+Enter (other panel) | — | same |
| duplicate line | Ctrl+Shift+D (via block) | — | Ctrl+Shift+D |
| move line up / down | Alt+↑ / Alt+↓ (via block) | — | Alt+↑ / Alt+↓ |
| reload from disk | Ctrl+Shift+R | — | Ctrl+R |
| new file (from the panel) | Shift+F4 | Shift+F4 | Shift+F4 |

**Shipped today** in the `far` profile's `editor` scope (`fcmd --check-keymap` prints the live
list): F2 save, Shift+F2 save as, F4 / F10 / Esc quit, Ctrl+R reload, F7 search, Shift+F7 next,
Alt+F7 previous, Ctrl+F7 replace, Alt+F8 go to line, Ctrl+Z undo, Ctrl+Shift+Z redo, Ctrl+A
select all, Ctrl+C / Ctrl+Ins copy, Ctrl+X / Shift+Del cut, Ctrl+V / Shift+Ins paste, Ctrl+Y
delete line, Ctrl+Shift+D duplicate line, Alt+↑ / Alt+↓ move line, Alt+I / Alt+U indent and
outdent, Ctrl+K delete to end of line, Ctrl+Backspace delete word left, Ctrl+Alt+L read-only
toggle, Ctrl+Q literal next key, Ctrl+Shift+Ins append the block to the clipboard, Ctrl+N /
Ctrl+E the first and last line on the screen. Movement, typing, Shift+arrows (stream block) and
Alt+Shift+arrows (column block) are handled by the editor itself and are not bindable yet.

## 8b. Netbox (connections)

| Action | FAR | MC | FC |
| --- | --- | --- | --- |
| connection list | F11 → NetBox | — | **Ctrl+Shift+N** |
| connect by URL | inside the plugin | `cd sftp://host/dir` | **Ctrl+L** with `sftp://user@host:port/dir` |
| disconnect the active panel | Shift+F11-ish, per plugin | `cd /` | **Alt+Shift+N** |
| inside the list | — | — | Enter connects · `n` a new URL · `Ins`/`F7` stores a session · `F4` edits one · `d` disconnects · Esc closes |
| session editor | NetBox session dialog | — | **`Ctrl+Shift+N` → `Ins`/`F7`/`F4`**: name, host, port, user, authentication, key file, remote folder, group, keep-alive, compression (SPEC 10 §3.5) |

## 7b. Viewer: the structure pane (shipped, SPEC 05 §1.8)

| Action | FC |
| --- | --- |
| open or close the pane | **F11** |
| move the keys between the bytes and the pane | **Tab** |
| inside the pane | ↑↓ PgUp PgDn Home End |
| take the bytes to the row's offset | **Enter** (a row that points nowhere says so) |

FAR has no equivalent: its `F11` is the plugin menu, which in the viewer we leave free because
there are no plugins yet. When they arrive the pane moves rather than the plugin menu.

## 8c. The menu bar and the dialogs (shipped, SPEC 10)

| Action | FAR | FC |
| --- | --- | --- |
| open the menu bar | F9 | **F9** (the menu of the panel that has the focus) |
| inside the bar | ←→ menus, ↑↓ entries, letter, Enter, Esc | the same |
| drive / connection list | Alt+F1 left, Alt+F2 right | **Alt+F1** / **Alt+F2**, as a table: type, label, filesystem, size, free — then the connections and object stores, numbered |
| folder history | Alt+F12 | **Alt+F12** |
| sort modes list | Ctrl+F12 | **Ctrl+F12** (choosing the current order again reverses it) |
| reverse the order directly | — | **Ctrl+Shift+F12** |
| themes | Options → Colors | **Ctrl+Shift+T** cycles · Options → Themes lists them |
| languages | Options → Languages | Options → Languages, applied at once |
| save the configuration | Shift+F9 | **Shift+F9** |
| inside a settings dialog | ↑↓ rows, Space toggles, Tab to the buttons, Enter, Esc | the same, plus **F10** for OK |
| inside a choose-one list | ↑↓ PgUp PgDn Home End, Enter, Esc | the same |

One of FAR's keys is not ours yet, and the menu says nothing rather than something untrue:
`Ctrl+L` (FAR: info panel) is "change directory" here. It is in `docs/BACKLOG.md` as a
decision to settle when that feature lands. `Ctrl+A` was the other one, and it is settled: it
is the attributes dialog, as it is in FAR, and select-all moved to FAR's own `Shift+Gray +`
with `Ctrl+Shift+A` beside it.

## 8d. Selection, the keys that ignore it, and the panel chrome (shipped)

| Action | FAR | FC |
| --- | --- | --- |
| select / deselect by mask | Gray + / Gray − | **`+` / `−`** (the whole mask grammar: lists, `[c-f]`, `/regex/i`, `include\|exclude`, `<arc>`) |
| by the current file's extension | Ctrl+Gray + / − | **Ctrl++ / Ctrl+−** |
| by name, whatever the extension | Alt+Gray + / − | **Alt++ / Alt+−** |
| invert, folders included | Ctrl+Gray * | **Alt+*** |
| undo the last sweep | Ctrl+M | **Ctrl+Shift+M** (one press undoes a whole sweep, not one row) |
| copy / move / delete the item under the cursor | Shift+F5 / Shift+F6 / Shift+F8 | the same, and the list travels with the dialog |
| run a command once per selected file | Ctrl+G | **Ctrl+G** (in sequence, counted, stopping at the first failure) |
| give the screen to one panel | Ctrl+F1 / Ctrl+F2 | the same, and the focus goes with it |
| scroll long names | Alt+←/→ | **Ctrl+Shift+←/→** |
| the panel's own status line, scrollbar, free space | settings | three settings in the Panel dialog; the status line is off by default because we also have a screen-wide status bar |

## 8e. Find file (shipped in part)

| Action | FAR | FC |
| --- | --- | --- |
| open the dialog | Alt+F7 | **Alt+F7**, filled in with the last search |
| inside the dialog | ↑↓ rows, Space toggles, Tab to the buttons, Enter, Esc | the same, plus **F10** for OK |
| in the results | ↑↓ PgUp PgDn Home End | the same |
| go to the file | Enter | **Enter** — the panel changes directory and the cursor lands on it |
| read it without leaving | F3 | **F3** |
| turn the results into a panel | Panelize | **F5** |
| stop the search, keep the results | Esc | **Esc**; a second Esc closes the list |

## 9. Terminal reality check (design constraints)

A TUI cannot receive every combination the Windows console gives FAR. Decisions:

1. **`LeftCtrl` vs `RightCtrl` is unavailable** in classic terminals. FAR's "set bookmark with
   Ctrl+Shift+N, jump with LeftCtrl+N" becomes `Ctrl+Shift+0..9` = set, `Alt+0..9` = jump.
2. **Numpad / "gray" keys** are not distinguishable from the main block in most terminals.
   `+`, `-`, `*` are therefore bound as plain characters when the command line is empty.
   The *modified* ones — `Ctrl++`, `Ctrl+−`, `Alt++`, `Alt+−`, `Alt+*`, which carry FAR's
   selection matrix — need a terminal that reports modified printable keys: kitty, foot,
   WezTerm, Windows Terminal, or xterm with `modifyOtherKeys`. Where the terminal does not,
   the same work is a menu entry away, and `Ctrl+A` / `Ctrl+Shift+A` still select everything
   and nothing. FAR's `Ctrl+Gray *` (invert including folders) is `Alt+*` here, because that
   one a terminal does deliver.
3. **`Shift+F1..F12`, `Ctrl+F1..F12`, `Alt+F1..F12`** require a capable terminfo; we also detect
   and enable the **kitty keyboard protocol** (`CSI > 1 u`) and the **Win32 input mode** on
   Windows terminals, which restore full modifier reporting including key releases. Without
   them we fall back to a published **two-key chord** scheme: `F12` prefix (e.g. `F12 5` =
   `Shift+F5`) and the `Esc`-prefix for Alt.
4. **`Ctrl+arrows`, `Ctrl+Shift+arrows`, `Alt+arrows`** are detected at startup; a built-in
   "learn keys" dialog (mc's idea) records what the terminal actually sends and stores it in
   `keys.local.toml`.
5. **Conflicts with the terminal emulator and the desktop** (`Ctrl+Shift+C/V`, `Alt+F4`,
   `Ctrl+W`, `F10` on GNOME, `Ctrl+Q`) are listed in the keymap docs; every one of them has an
   alternative binding, listed next to it in the help, and a `keys.local.toml` override.
   **The font is not ours to set.** A terminal program does not own its font: the emulator
   does, and there is exactly one control sequence for asking — xterm's `OSC 50`, where a `#`
   and a signed number step through the font menu. So `Ctrl++`, `Ctrl+−` and `Ctrl+0` send
   that, and what happens next is the terminal's business: xterm and the terminals that copy
   it change font, the rest ignore it. On most emulators (VTE, Konsole, Windows Terminal,
   alacritty, foot) `Ctrl` with plus or minus is the emulator's *own* zoom and never reaches a
   program at all — the same outcome by a shorter road, which is why taking those keys costs
   nothing there. The status line says the request was made, not that it was granted, because
   there is no reply to wait for.
   The **tabs** sit on the `Ctrl+Alt` row for this reason and no other: `Ctrl+T` is the tree
   panel and `Ctrl+Tab` steps between the open screens, both of them FAR's own, so there was
   no key of FAR's left to keep. `Ctrl+Alt` with an *arrow* is avoided because most desktops
   switch workspaces with it before a terminal ever sees it, and `Ctrl+Alt+L` because most of
   them lock the screen — hence `Ctrl+Alt+K` for locking a tab.
6. **`Ctrl+I` is `Tab`, `Ctrl+M` is `Enter`, `Ctrl+J` is a newline** in every terminal, so
   they are not bindable: "restore selection" is `Ctrl+Shift+M`, and the help page states the
   rule instead of letting a user wonder why nothing happens. The quick filter went to
   `Ctrl+Shift+I` for the second reason below.
8. **`Alt+<letter>` belongs to the quick search**, which is what FAR does with it. A binding
   always wins over the search, so a plain `Alt+<letter>` anywhere a panel can see it is a
   letter one can never search a listing for. Three of ours used to take one — the site list
   (`Alt+N`), the quick filter (`Alt+I`) and the theme (`Alt+T`) — and they are now
   `Ctrl+Shift+N`, `Ctrl+Shift+I` and `Ctrl+Shift+T`. The editor's `Alt+I` and `Alt+U` stay
   where FAR has them: there is no quick search in the editor to collide with.
7. **`Esc`** is both "cancel" and the Alt prefix: we use a 25 ms disambiguation timeout
   (configurable), and `Esc Esc` always means cancel.

## 10. Command vocabulary (what keys actually bind to)

Keys never hard-code behaviour; they bind to named commands, like Double Commander's `cm_*`.
Naming: `<area>.<object>.<verb>`.

```
panel.focus.toggle          panel.swap                 panel.reread
panel.view.set(brief|medium|full|wide|detail|custom:N)
panel.sort.by(name|ext|size|mtime|ctime|atime|owner|none)   panel.sort.reverse
panel.filter.quick          panel.filter.set            panel.hidden.toggle
panel.tab.new|close|next|prev|move|lock|name     panel.layout.toggle
nav.enter  nav.parent  nav.root  nav.history.back|forward  nav.bookmark.set(N)|go(N)  nav.bookmarks
nav.goto.path  nav.goto.fuzzy  nav.drives  nav.other.sync
sel.toggle  sel.all  sel.none  sel.invert  sel.mask(+|-)  sel.ext  sel.restore
file.view  file.view.internal  file.edit  file.edit.internal  file.edit.new
file.copy  file.move  file.rename.inplace  file.mkdir  file.mkfile
file.delete(trash|permanent|wipe)  file.link(hard|sym|rel)  file.attrs
file.multirename  file.checksum.create|verify  file.split  file.combine
file.pack  file.unpack  file.archive.test  file.archive.commands
tool.find  tool.grep  tool.diskusage  tool.compare.dirs  tool.compare.files
tool.sync.dirs  tool.usermenu  tool.history(cmd|view|folder)  tool.macro.record|play
job.queue.show  job.pause  job.resume  job.cancel
shell.run  shell.output.show  shell.term.toggle  shell.insert(name|path|other)
app.menu  app.help  app.settings  app.palette  app.theme.next  app.keymap.profile(name)
app.font(N)
app.quit  app.suspend
```

Each command carries an id, a localisable label, an optional default key per profile, and a
"scope" (panel / viewer / editor / dialog / global). The palette (`Ctrl+Shift+P`) lists every
command with its current binding — which also makes the whole keymap self-documenting.

## 11. Profiles

Two are built in — `fcmd --list-themes`' sibling, `fcmd --keymap <name> --check-keymap`, says
which, and refuses a name it does not have:

| Profile | Base | Notes |
| --- | --- | --- |
| `far` (default) | FAR 3 | the table above |
| `mc` | Midnight Commander | `Alt+.` hidden files, `Ctrl+x` prefix chords, `Insert` tag, `+`/`\` select/unselect, `Alt+?` find |

Planned, and not built yet — each is a file of bindings rather than any new behaviour, and
`docs/spec/09-roadmap.md` has them:

| Profile | Base | What it would be |
| --- | --- | --- |
| `tc` | Total Commander | `Alt+F5` pack, `Alt+F9` unpack, `Ctrl+M` multi-rename, `Ctrl+B` branch view, `Ctrl+Q` quick view, `F2` reread |
| `nc` | Norton Commander 5 | minimal set, no Ctrl+F-key sorting beyond name/ext/date/size |
| `vi` | vifm/yazi-ish | modal: `hjkl`, `gg`/`G`, `dd`/`yy`/`p`, `/` search, `:` command line, `Space` select |

Profiles are compiled into the executable. A `[[keymap]]` block in `fcmd.toml` may
`inherit = "far"` and override single bindings, and a block whose `name` matches a built-in
profile replaces it. Switching is runtime (`app.keymap.profile`), no restart.
