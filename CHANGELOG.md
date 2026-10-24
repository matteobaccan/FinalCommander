# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[semantic versioning](https://semver.org/), which before 1.0 means anything may still move.

## [Unreleased]

### The screen looks like FAR's

- A **`far` theme**: FAR Manager's own default palette, element for element, read from
  `far/palette.cpp` of `FarGroup/FarManager` (BSD-3-Clause). Light cyan on blue panels, yellow
  for marked files and column titles, a cyan bar on the active panel's path, cyan menus with
  yellow shortcuts, light grey dialogs with cyan input fields, red warning dialogs.
- **Column titles** (`Name │ Size │ Date Time`) under the panel frame, with the sort-mode letter
  in the top-left corner — lower case for ascending order, upper case for descending — and a
  `*` beside it while a filter is on, exactly where FAR puts them. `panel.column_titles` turns
  the row off; with it off the sort mode goes back to the footer.
- The **active panel's path** is drawn inverted, as FAR draws it.
- **Menus have their own colours** instead of borrowing the dialog's: a dropdown is cyan with
  yellow shortcuts and a black selection bar, and the bar across the top is black on cyan.
- **Shadows.** Dialogs, menus, lists and the progress window darken two columns to their right
  and one row below, which is FAR's `DropShadow` geometry. The shadow halves what is already
  there rather than painting a colour, so it works in every theme and in 16 colours.
- Every theme gained the keys those three changes need (menu, menu bar, dialog fields and
  buttons, warning dialogs, column titles, the active panel's title, the scrollbar) — all seven
  built-in themes define every key, and a test keeps it that way.
- The key bar now uses FAR's own wording where FAR ships the language: `UserMn`, `MkFold`,
  `ConfMn`, and their Italian, German, Spanish and Russian forms.

### Documentation

- `docs/04-far-inventory.md`: FAR 3.x read from its own repository — 283 help topics, 311
  settings keys — with every feature marked against what we do, the three grammars worth
  matching exactly (panel columns, file masks, metasymbols), and a section on exactly how close
  the screen now is and what is still different.
- Three facts corrected in `docs/03-keymap.md` and one in the backlog, now that the source
  rather than the documentation is the reference.

## [0.0.2] — 2026-10-24

The menus. Everything the keys do can now be found by reading, which is the half of FAR that
makes it learnable.

### Menu bar (`F9`)

- FAR's five menus — Left, Files, Commands, Options, Right — with `←`/`→` between them, `↑`/`↓`
  over the entries, a letter to pick one, and a `√` on the view mode and the hidden-files
  setting the panel is actually in.
- `Left` and `Right` act on their own panel whichever one has the focus, as FAR's do.
- Every entry is a command from the same vocabulary the keys use, so there is no behaviour
  reachable only from a menu, and the shortcut shown beside an entry is read from the keymap in
  force — rebind it in `scmd.toml` and the menu says the new key.
- An entry whose feature is not here yet stays in the menu, drawn dim, and says
  `not implemented: <command>` when chosen: the menu is a map of the program, and a map with
  the gaps marked beats one that lies. A test stops those entries from advertising a key that
  already does something else.

### Settings dialogs

- System, Panel, Interface and Confirmations. Every row is bound to a configuration key and
  goes in through `Config::set` — the same code path as `--set key=value` — so a dialog cannot
  write something the file format does not accept.
- `OK` (or `F10`) applies to the running panels at once: no restart, nothing to save first.
  `Esc` keeps nothing.
- `Save setup` (`Shift+F9`) writes `scmd.toml` as the configuration now stands, themes that came
  from the file included. What it writes parses back into the same configuration.

### Lists to choose from

- Languages — switches the whole interface on the spot — and themes.
- Drives, mount points and open connections (`Alt+F1` left, `Alt+F2` right): the platform's
  roots plus every live `sftp://` connection, and a configured-but-closed site connects first.
- Folder history per panel (`Alt+F12`).
- Sort modes (`Ctrl+F12`), where choosing the order already in force turns it round, the way FAR
  does it, unless `panel.allow_reverse_sort` says otherwise. `Ctrl+Shift+F12` reverses directly.

### Netbox session editor

- `Alt+N` then `Ins`/`F7` stores a connection, `F4` edits the one under the cursor: name, host,
  port, user, authentication (SSH agent, key file or ask), key file, remote folder, group,
  keep-alive, compression. Connections no longer need `scmd.toml` to be edited by hand.
- It is validated before it is stored — a host is required, port and keep-alive must be numbers,
  the key authentication needs a file, and the result has to be a site the netbox can use. A
  dialog that cannot be applied stays open with the reason in the status line.
- No password or passphrase is ever a field: a session says *how* to authenticate and the secret
  is asked for when the connection needs it.

### Also

- The dialog framework gained text rows with a real caret and the editing keys.
- New commands: `app.themes`, `app.languages`, `app.setup.save`, `opt.system`, `opt.panel`,
  `opt.interface`, `opt.confirmations`, `panel.sort.menu`, `panel.drive(left|right)`,
  `panel.history`.
- Two new settings: `panel.allow_reverse_sort` and `general.confirm_overwrite`.
- 105 new messages in each of the seven languages.
- `docs/spec/10-menus-and-dialogs.md` specifies all of it; `docs/BACKLOG.md` lists what is
  specified but not built, with the open questions marked — including the two keys where we
  differ from FAR today (`Ctrl+A` is select-all here, `Ctrl+L` is change-directory).

## [0.0.1] — 2026-10-24

The first release anyone can download. It is a preview: the parts that are here are tested and
behave, and what is missing says so instead of failing quietly.

### Panels and navigation

- Two panels, keyboard-first, with the FAR key layout (`mc` is the second shipped profile).
- Brief, full, detail and wide views; sorting by name, extension, size, date or unsorted, with
  natural ordering, directories first and a second press to reverse.
- Navigation with history (`Alt+←`/`Alt+→`), parent, root, `Ctrl+L` to type a path, quick
  search by typing, `Alt+I` for a live filter, `Ctrl+H` for hidden files, `Space` for a
  recursive directory size, `Ctrl+=` to send the other panel here.
- Selection by `Ins`, by mask (`+`, `-`, `*`), invert and restore, with totals.

### File operations (`F5`–`F8`)

- Copy, move, create directory and delete, planned before anything is touched: the planner
  counts files and bytes and refuses the impossible (a directory into itself, a file onto
  itself) before the first write.
- A copy is written beside its destination and renamed into place, so a crash or a cancel
  never leaves a half file; a move deletes the source only once the destination is there, and
  a move on the same filesystem is a rename, so it copies nothing.
- Conflicts and errors are questions, not guesses: overwrite, overwrite if newer, skip,
  rename, and the `…All` variants; retry, skip, skip all or cancel on an error.
- Progress with per-file and total bars, cancellable at every step.

### Viewer (`F3`)

- Text, dump and hex on files of any size — position is a byte offset, so a 40 GB log opens in
  one read.
- Encoding detected (UTF-8, UTF-16 with or without a BOM, CP1252, CP437) and switchable;
  no-wrap, character and word wrapping; search (plain, regex or hex pattern) with highlighted
  hits; go to offset or percentage; follow mode; bookmarks.
- `F5` shows the code inside a compiled file: ELF, PE, Mach-O and raw x86/x86-64 disassembled
  with sections and symbols, Java class files and .NET assemblies read in process, WebAssembly
  printed as its text format. `F6` hands anything else to an installed decompiler through a
  `[[decompile]]` rule.

### Editor (`F4`)

- The file's encoding, line ending, BOM and final newline survive a round trip; the save is
  atomic, and a file changed on disk is never overwritten without a confirmation.
- Stream blocks and **column blocks**, grouped undo and redo, search and replace, go to
  `line:column`, bookmarks, auto-indent, line operations.
- `Shift+F4` edits a new file.

### Netbox: SFTP (`Alt+N`)

- A panel can sit on a server. One connection per `user@host:port` is shared by both panels
  and every job, so `F5` and `F6` copy and move up, down, between two servers or inside one,
  with the same progress and conflict handling as a local copy.
- Host keys are checked against `known_hosts`: an unknown one is shown as a fingerprint for
  you to accept, and a changed one is refused.
- Authentication through the SSH agent, a key file, or a password asked for in a masked
  prompt. Nothing is stored: `[[site]]` blocks say only *how* to authenticate.
- `F3` views a remote file through a temporary copy; `F4` edits that copy and `F2` writes it
  back, uploaded beside the original and renamed into place.

### The rest

- Six themes (commander, black, white, matrix, tron, blade) and seven interface languages
  (English, Italian, German, French, Spanish, Russian, Simplified Chinese).
- One executable: themes, keymaps and languages are compiled in. The only files on disk are
  `scmd.toml` (if you want one) and an optional plugins folder.
- Non-interactive modes for scripts and CI: `--list-themes`, `--list-commands`,
  `--list-languages`, `--check-keymap`, `--export-theme`, `--dump-config`, `--paths`.

### Not here yet

Archives, the trash (`F8` deletes permanently), the command line and the user screen, the menu
(`F9`), find files (`Alt+F7`), FTP/WebDAV/S3, the plugin host, syntax highlighting, several
open editor buffers, tabs, the diff viewer and the background job queue. Anything
unimplemented says so rather than doing nothing.

[0.0.1]: https://github.com/matteobaccan/SupremeCommander/releases/tag/v0.0.1
