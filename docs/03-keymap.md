# Keyboard reference and default keymap

Sources: FAR Manager's own help (`far/FarEng.hlf.m4`, BSD-3), the `mc(1)` man page (GPL-3
project, facts only), Krusader's handbook (`doc/handbook/keyboard-commands.docbook`), MTN2's
README, and third-party cheat sheets for Total Commander (proprietary — **secondary sources,
marked `?` where unverified**). All descriptions below are written from scratch as factual
key→action pairs; no documentation text is reproduced. See `01-licensing.md` §3.

Column `SC` = SupremeCommander default (profile `far`, our baseline).

---

## 1. Function keys — plain

| Key | FAR | MC | TC | Krusader | SC |
| --- | --- | --- | --- | --- | --- |
| F1 | help (context) | help | help | help | help (context) |
| F2 | user menu | user menu | reread panel? | rename | user menu |
| F3 | view (internal/assoc.); on a dir: compute size | view | view | view | view / dir size |
| F4 | edit; on a dir: attributes dialog | edit | edit | edit | edit |
| F5 | copy | copy | copy | copy | copy |
| F6 | rename / move | rename/move | move/rename | move | move / rename |
| F7 | make directory | make directory | new folder | new folder | make directory |
| F8 | delete (to trash per settings) | delete | delete | delete/trash | delete (to trash) |
| F9 | menu bar | pull-down menus | left menu? | terminal | menu bar |
| F10 | quit | quit | exit? | quit | quit |
| F11 | plugin commands | — | — | — | plugin/command palette |
| F12 | — | screen list | custom filter? | — | panel/tab list |

## 2. Shift + F-keys

| Key | FAR | MC | TC `?` | Krusader | SC |
| --- | --- | --- | --- | --- | --- |
| Shift+F1 | add to archive | — | — | what's this | add to archive |
| Shift+F2 | extract from archive | — | compare by content | multi-rename | extract archive |
| Shift+F3 | archive commands menu | — | — | view URL | archive commands |
| Shift+F4 | edit a new file | edit new file | create new text file | edit new file | edit new file |
| Shift+F5 | copy the item under the cursor (same panel) | — | copy in same panel | copy by queue | copy here |
| Shift+F6 | rename/move the item under the cursor | — | rename in place | move by queue | rename in place |
| Shift+F8 | delete only the item under the cursor | — | delete permanently | — | delete under cursor |
| Shift+F9 | save configuration | — | — | — | save configuration |
| Shift+F10 | select the last used item of the user menu | — | — | view all files | same, once the user menu exists (unbound today) |
| Shift+F11 | sort groups on/off (named mask groups that sort ahead of or behind everything else — not the same as "directories first") | — | — | — | same, once sort groups exist (unbound today) |
| Shift+F12 | selected files first | — | — | custom view | selected first (a setting today, `Shift+F12` unbound) |
| Shift+Del | delete skipping trash | delete | delete permanently | delete permanently | delete permanently |
| Shift+Enter | run in a separate window / OS open | — | run in own window | — | OS-open item |
| Shift+arrows | extend selection | extend selection | extend selection | extend selection | extend selection |

## 3. Alt + F-keys

| Key | FAR | TC `?` | Krusader | SC |
| --- | --- | --- | --- | --- |
| Alt+F1 | drive menu, left panel | drive left | left bookmarks | drive/mount menu, left |
| Alt+F2 | drive menu, right panel | drive right | right bookmarks | drive/mount menu, right |
| Alt+F3 | alternative (external/internal) viewer | alternative viewer | — | alternative viewer |
| Alt+F4 | alternative editor | exit | — | alternative editor |
| Alt+F5 | print | pack | — | print / export list |
| Alt+F6 | create hard/symlink | — | — | create link |
| Alt+F7 | find files | search | — | find files |
| Alt+F8 | command history | command-line history | — | command history |
| Alt+F9 | toggle window size (console) | — | — | toggle maximise/zoom panel |
| Alt+F10 | find folder | directory tree | — | find folder / tree |
| Alt+F11 | view & edit history | — | — | view/edit history |
| Alt+F12 | folder history | — | — | folder history |
| Alt+Shift+F9 | plugin configuration | — | — | plugin settings |
| Alt+Del | wipe (secure delete) | — | — | wipe |
| Alt+Ins | screen grabber | — | — | (unbound) |
| Alt+Enter | — | properties | properties | properties |
| Alt+F1..F12 (KDE) | — | — | reserved by the desktop | — |

## 4. Ctrl + F-keys (panel modes and sorting)

| Key | FAR | TC `?` | SC |
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

| Action | FAR | MC | Krusader | SC |
| --- | --- | --- | --- | --- |
| switch panel | Tab | Tab, Ctrl+I, ←/→ | Tab | Tab |
| swap panels | Ctrl+U | Ctrl+U | Ctrl+U | Ctrl+U |
| re-read panel | Ctrl+R | Ctrl+R | Ctrl+R | Ctrl+R |
| info panel | Ctrl+L | Ctrl+x i | Alt+↓ (sidebar) | Ctrl+L |
| quick view panel | Ctrl+Q | Ctrl+x q | — | Ctrl+Q |
| tree panel | Ctrl+T | — | — | Ctrl+T |
| hide both panels | Ctrl+O (user screen) | Ctrl+O | — | Ctrl+O |
| hide inactive panel | Ctrl+P | — | — | Ctrl+P |
| panel height / width | Ctrl+↑/↓, Ctrl+←/→ | — | — | Ctrl+↑/↓, Ctrl+←/→ |
| toggle hidden files | Ctrl+H | Alt+. (Shift+Alt+H in some builds) | Alt+. | Ctrl+H |
| short/long names | Ctrl+N | Alt+T (cycle formats) | Alt+Shift+B/D | Ctrl+N |
| key bar on/off | Ctrl+B | — | — | Ctrl+B |
| view modes 1..0 | LeftCtrl+1..0 | Alt+T cycles | Alt+Shift+B/D | Ctrl+1..0 |
| parent directory | Ctrl+PgUp | Ctrl+PgUp | Ctrl+PgUp | Ctrl+PgUp, Backspace |
| enter dir / archive | Ctrl+PgDn | Ctrl+PgDn | Enter | Ctrl+PgDn |
| root directory | Ctrl+\ | — | Ctrl+Backspace | Ctrl+\ |
| same dir in other panel | — | Alt+I | Ctrl+= | Ctrl+= |
| other panel follows cursor | — | Alt+O | Ctrl+←/→ | Alt+O |
| directory hotlist/bookmarks | folder shortcuts: Ctrl+Shift+0..9 set, RightCtrl+0..9 go (and insert the path when in an edit line), plus a list in the Commands menu | Ctrl+\ | Ctrl+D / Ctrl+B add | Ctrl+D; Ctrl+Shift+0..9 set; Alt+0..9 go |
| directory history | Alt+F12 | Alt+Shift+H; Alt+Y / Alt+U back/forward | Ctrl+H; Alt+←/→ | Alt+F12; Alt+←/→ |
| quick cd / path entry | — | Alt+C | Ctrl+L | Ctrl+L |
| new tab / close tab | — | — | Ctrl+Alt+N / Ctrl+W | Ctrl+T… see note | 
| next/prev tab | — | — | Alt+←/→ (tabs: Shift+←/→) | Ctrl+PgDn is taken → Alt+1..9, Ctrl+Tab |
| find file | Alt+F7 | Alt+? | Ctrl+S | Alt+F7 |
| disk usage | — | Ctrl+Space on a dir | Alt+Shift+S | Alt+Shift+S |
| compare directories | Commands menu, no default key (marks what is missing or newer in the other panel; names, sizes and times only, no subfolders) | — | Alt+Shift+C | Alt+Shift+C |
| synchronise directories | — | — | Ctrl+Y | Ctrl+Y |
| pack / unpack | Shift+F1 / Shift+F2 | — | Alt+Shift+P / Alt+Shift+U | Shift+F1 / Shift+F2 |
| test archive | — | — | Alt+Shift+E | Alt+Shift+E |
| multi-rename | — | — | Shift+F2 | Ctrl+M |
| select group / unselect group | Gray + / Gray − | + / \ | Ctrl++ / Ctrl+− | + / − |
| invert selection | Gray * | — | Alt+* | * |
| select all / none | Shift+Gray + / Shift+Gray − | — | Alt++ / Alt+− | Ctrl+A / Ctrl+Shift+A |
| by extension | Ctrl+Gray + / − | — | — | Alt++ / Alt+− |
| restore previous selection | Ctrl+M | — | — | Ctrl+Shift+M |
| copy names to clipboard | Ctrl+Ins / Ctrl+Shift+Ins names, Alt+Shift+Ins full names, Ctrl+Alt+Ins real/UNC names, Ctrl+Shift+C / Ctrl+Shift+X copy/cut the files themselves | — | — | Ctrl+Shift+C (names), Alt+Shift+C (full paths) |
| attributes / permissions | Ctrl+A | Ctrl+x c (chmod), Ctrl+x o (chown) | Alt+Enter | Ctrl+A |
| apply command to selection | Ctrl+G | — | — | Ctrl+G |
| describe files | Ctrl+Z | — | — | Ctrl+Shift+D |
| toggle terminal pane | — | Ctrl+O | Ctrl+Alt+E / Ctrl+Shift+↑ | Ctrl+Alt+E |
| command palette | — | — | — | **Ctrl+Shift+P** (new) |

Notes on FAR specifics we cannot reproduce verbatim in a terminal: `LeftCtrl` vs `RightCtrl`
(FAR uses them for *set* vs *go to* bookmarks) and numpad "gray" keys are generally
indistinguishable in a TTY — see §9.

## 6. Command line

| Action | FAR | MC | SC |
| --- | --- | --- | --- |
| char left/right | ←/→, Ctrl+S / Ctrl+D | ←/→ | ←/→ |
| word left/right | Ctrl+←/→ | Alt+B / Alt+F | Ctrl+←/→ |
| start/end of line | Ctrl+Home / Ctrl+End | Ctrl+A / Ctrl+E | Home / End |
| delete word left/right | Ctrl+Backspace / Ctrl+Del | Alt+Backspace / Alt+D | Ctrl+W / Alt+D |
| delete to end of line | Ctrl+K | Ctrl+K | Ctrl+K |
| clear line | Ctrl+Y | Ctrl+U | Ctrl+U |
| history prev/next | Ctrl+E / Ctrl+X | Alt+P / Alt+N | ↑/↓ (when non-empty), Alt+F8 list |
| insert current file name | Ctrl+J, Ctrl+Enter | Alt+Enter | Ctrl+Enter |
| insert name from other panel | Ctrl+Shift+Enter | — | Ctrl+Shift+Enter |
| insert full path (active/passive) | Ctrl+F / Ctrl+; | — | Alt+Enter / Alt+Shift+Enter |
| insert left/right panel path | Ctrl+[ / Ctrl+] | — | Ctrl+[ / Ctrl+] |
| completion | — | Tab (also Alt+Tab list) | Tab |
| run keeping output | — | Ctrl+O after run | Ctrl+O |

## 7. Viewer

| Action | FAR | MC | SC |
| --- | --- | --- | --- |
| line / page / file bounds | ↑↓, PgUp/PgDn, Home/End | same | same |
| horizontal scroll | ←/→, Ctrl+←/→ (20 cols) | ←/→ | ←/→, Ctrl+←/→ |
| wrap toggle | F2 (wrap), Shift+F2 (word wrap) | F2 | F2 / Shift+F2 |
| hex mode | F4; Shift+F4 mode menu (text/hex/dump) | F4 | F4 / Shift+F4 |
| switch to editor | F6 | — | F6 |
| search / next / previous | F7 / Shift+F7 or Space / Alt+F7 | F7 / Ctrl+S, Ctrl+R | F7 / n / N (and Shift+F7/Alt+F7) |
| codepage | F8 toggle, Shift+F8 menu | Alt+E | Shift+F8 |
| go to position | Alt+F8 | Alt+G / m | Alt+F8, `:`|
| bookmarks set / go | Ctrl+Shift+0..9 / LeftCtrl+0..9 | — | Ctrl+Shift+0..9 / Alt+0..9 |
| copy selection | Ctrl+Ins, Ctrl+C | — | Ctrl+C |
| next / previous file in panel | Gray + / Gray − | — | + / − |
| quit | Esc, F3, F10, Numpad5 | Esc, F10, q | Esc, F3, F10, q |
| follow (tail) | End then auto-append | — | Shift+F | 

## 8. Editor

| Action | FAR | MC (mcedit) | SC |
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
| append to clipboard | Ctrl+Gray + | — | Ctrl+Shift+C |
| copy / move block to cursor | Ctrl+P / Ctrl+M (persistent blocks) | F5 / F6 | Ctrl+Shift+P / Ctrl+Shift+M |
| delete block / line / to EOL | Ctrl+D / Ctrl+Y / Ctrl+K | F8 / Ctrl+Y / Ctrl+K | Ctrl+D / Ctrl+Y / Ctrl+K |
| delete word left / right | Ctrl+Backspace / Ctrl+T, Ctrl+Del | Alt+Backspace | Ctrl+Backspace / Alt+D |
| indent / outdent block | Alt+I / Alt+U | — | Tab / Shift+Tab (block), Alt+I / Alt+U |
| start/end of file | Ctrl+Home, Ctrl+PgUp / Ctrl+End, Ctrl+PgDn | Ctrl+Home / Ctrl+End | Ctrl+Home / Ctrl+End |
| start/end of screen | Ctrl+N / Ctrl+E | — | — |
| scroll screen | Ctrl+↑ / Ctrl+↓ | — | Ctrl+↑ / Ctrl+↓ |
| bookmarks set / go | Ctrl+Shift+0..9 / LeftCtrl+0..9 | — | Ctrl+Shift+0..9 / Alt+0..9 |
| codepage toggle / menu | F8 / Shift+F8 | — | Shift+F8 |
| read-only lock | Ctrl+L | — | Ctrl+Alt+L |
| literal next key (char code) | Ctrl+Q | — | Ctrl+Q |
| insert name of current file | Shift+Enter / Ctrl+Shift+Enter (other panel) | — | same |
| duplicate line | Ctrl+Shift+D (via block) | — | Ctrl+Shift+D |
| move line up / down | Alt+↑ / Alt+↓ (via block) | — | Alt+↑ / Alt+↓ |
| reload from disk | Ctrl+Shift+R | — | Ctrl+R |
| new file (from the panel) | Shift+F4 | Shift+F4 | Shift+F4 |

**Shipped today** in the `far` profile's `editor` scope (`scmd --check-keymap` prints the live
list): F2 save, Shift+F2 save as, F4 / F10 / Esc quit, Ctrl+R reload, F7 search, Shift+F7 next,
Alt+F7 previous, Ctrl+F7 replace, Alt+F8 go to line, Ctrl+Z undo, Ctrl+Shift+Z redo, Ctrl+A
select all, Ctrl+C / Ctrl+Ins copy, Ctrl+X / Shift+Del cut, Ctrl+V / Shift+Ins paste, Ctrl+Y
delete line, Ctrl+Shift+D duplicate line, Alt+↑ / Alt+↓ move line, Alt+I / Alt+U indent and
outdent, Ctrl+K delete to end of line, Ctrl+Backspace delete word left, Ctrl+Alt+L read-only
toggle. Movement, typing, Shift+arrows (stream block) and Alt+Shift+arrows (column block) are
handled by the editor itself and are not bindable yet.

## 8b. Netbox (connections)

| Action | FAR | MC | SC |
| --- | --- | --- | --- |
| connection list | F11 → NetBox | — | **Alt+N** |
| connect by URL | inside the plugin | `cd sftp://host/dir` | **Ctrl+L** with `sftp://user@host:port/dir` |
| disconnect the active panel | Shift+F11-ish, per plugin | `cd /` | **Alt+Shift+N** |
| inside the list | — | — | Enter connects · `n` a new URL · `Ins`/`F7` stores a session · `F4` edits one · `d` disconnects · Esc closes |
| session editor | NetBox session dialog | — | **`Alt+N` → `Ins`/`F7`/`F4`**: name, host, port, user, authentication, key file, remote folder, group, keep-alive, compression (SPEC 10 §3.5) |

## 7b. Viewer: the structure pane (shipped, SPEC 05 §1.8)

| Action | SC |
| --- | --- |
| open or close the pane | **F11** |
| move the keys between the bytes and the pane | **Tab** |
| inside the pane | ↑↓ PgUp PgDn Home End |
| take the bytes to the row's offset | **Enter** (a row that points nowhere says so) |

FAR has no equivalent: its `F11` is the plugin menu, which in the viewer we leave free because
there are no plugins yet. When they arrive the pane moves rather than the plugin menu.

## 8c. The menu bar and the dialogs (shipped, SPEC 10)

| Action | FAR | SC |
| --- | --- | --- |
| open the menu bar | F9 | **F9** (the menu of the panel that has the focus) |
| inside the bar | ←→ menus, ↑↓ entries, letter, Enter, Esc | the same |
| drive / connection list | Alt+F1 left, Alt+F2 right | **Alt+F1** / **Alt+F2** |
| folder history | Alt+F12 | **Alt+F12** |
| sort modes list | Ctrl+F12 | **Ctrl+F12** (choosing the current order again reverses it) |
| reverse the order directly | — | **Ctrl+Shift+F12** |
| themes | Options → Colors | **Alt+T** cycles · Options → Themes lists them |
| languages | Options → Languages | Options → Languages, applied at once |
| save the configuration | Shift+F9 | **Shift+F9** |
| inside a settings dialog | ↑↓ rows, Space toggles, Tab to the buttons, Enter, Esc | the same, plus **F10** for OK |
| inside a choose-one list | ↑↓ PgUp PgDn Home End, Enter, Esc | the same |

Two of FAR's keys are not ours yet, and the menu says nothing rather than something untrue:
`Ctrl+A` (FAR: file attributes) selects all here, and `Ctrl+L` (FAR: info panel) is "change
directory". Both are in `docs/BACKLOG.md` as decisions to settle when those features land.

## 9. Terminal reality check (design constraints)

A TUI cannot receive every combination the Windows console gives FAR. Decisions:

1. **`LeftCtrl` vs `RightCtrl` is unavailable** in classic terminals. FAR's "set bookmark with
   Ctrl+Shift+N, jump with LeftCtrl+N" becomes `Ctrl+Shift+0..9` = set, `Alt+0..9` = jump.
2. **Numpad / "gray" keys** are not distinguishable from the main block in most terminals.
   `+`, `-`, `*` are therefore bound as plain characters when the command line is empty.
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
6. **`Ctrl+I` is `Tab`, `Ctrl+M` is `Enter`, `Ctrl+J` is a newline** in every terminal, so
   they are not bindable: quick filter is `Alt+I`, "restore selection" is `Ctrl+Shift+M`, and
   the help page states the rule instead of letting a user wonder why nothing happens.
7. **`Esc`** is both "cancel" and the Alt prefix: we use a 25 ms disambiguation timeout
   (configurable), and `Esc Esc` always means cancel.

## 10. Command vocabulary (what keys actually bind to)

Keys never hard-code behaviour; they bind to named commands, like Double Commander's `cm_*`.
Naming: `<area>.<object>.<verb>`.

```
panel.focus.toggle          panel.swap                 panel.reread
panel.view.set(brief|full|wide|detail|custom:N)
panel.sort.by(name|ext|size|mtime|ctime|atime|owner|none)   panel.sort.reverse
panel.filter.quick          panel.filter.set            panel.hidden.toggle
panel.tab.new|close|next|prev|move               panel.layout.toggle
nav.enter  nav.parent  nav.root  nav.history.back|forward  nav.bookmark.set(N)|go(N)
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
app.quit  app.suspend
```

Each command carries an id, a localisable label, an optional default key per profile, and a
"scope" (panel / viewer / editor / dialog / global). The palette (`Ctrl+Shift+P`) lists every
command with its current binding — which also makes the whole keymap self-documenting.

## 11. Profiles shipped

| Profile | Base | Notes |
| --- | --- | --- |
| `far` (default) | FAR 3 | the table above |
| `mc` | Midnight Commander | `Alt+.` hidden files, `Ctrl+x` prefix chords, `Insert` tag, `+`/`\` select/unselect, `Alt+?` find |
| `tc` | Total Commander | `Alt+F5` pack, `Alt+F9` unpack, `Ctrl+M` multi-rename, `Ctrl+B` branch view, `Ctrl+Q` quick view, `F2` reread |
| `nc` | Norton Commander 5 | minimal set, no Ctrl+F-key sorting beyond name/ext/date/size |
| `vi` | vifm/yazi-ish | modal: `hjkl`, `gg`/`G`, `dd`/`yy`/`p`, `/` search, `:` command line, `Space` select |

Profiles are compiled into the executable. A `[[keymap]]` block in `scmd.toml` may
`inherit = "far"` and override single bindings, and a block whose `name` matches a built-in
profile replaces it. Switching is runtime (`app.keymap.profile`), no restart.
