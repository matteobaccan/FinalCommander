# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[semantic versioning](https://semver.org/), which before 1.0 means anything may still move.

## [Unreleased]

### Finding things (0.0.4, in progress)

- **A panel can be a list of files rather than a directory.** One method decides whether a
  row's path is the panel's own path with the name on the end or something the panel was told,
  and every operation, the viewer, the editor and the metasymbols go through it. A list panel
  re-reads by asking about its own paths again, so a file deleted since the list was built
  drops out; `Enter` on a row goes to the directory the file lives in; navigating away turns
  the panel back into a directory.
- **Find file (`Alt+F7`)**: masks in the filter grammar, text or a hex pattern, case
  sensitivity, whole words, files that do *not* contain it, five scopes, following symbolic
  links, folders in the results. It opens on the last search. Results stream into a list that
  is usable while the walk runs — `Enter` goes to the file, `F3` reads it, `F5` turns the
  results into the panel, `Esc` stops the search and keeps what it found.

## [0.0.3] — 2026-10-24

A command line, and the keys a FAR user reaches for without thinking.

### A command line, and the screen it writes on

- A **command line under the panels** (`ui.command_line`): everything printable goes to it once
  it has anything in it, and to quick search while it is empty — FAR's rule, so neither habit
  gets in the other's way. Word jumps break on the separators a path uses, `Ctrl+K` cuts to the
  end, and the history walks with `Ctrl+↑`/`Ctrl+↓` keeping the half-typed line safe.
- **Commands we run ourselves**, because a commander that shells out for `cd` cannot keep the
  panel in step: `cd`/`chdir` (with `~`, `-` and a remote panel's own notion of a path), `cls`,
  `set`, `pushd`/`popd`/`clrd`, `title`, `exit`, and `if [not] exist` / `if [not] defined`
  conditions, nestable.
- **Prefixes**: `view:`, `edit:`, `goto:`, `whereis:`, and `run:` with its output to a file, the
  screen, or nowhere.
- **Metasymbols** (SPEC 06 §5): FAR's `!.!`, `!`, `` !` ``, `!\`, `!@!`, `!&`, `!?title?init!`
  and the side prefixes, plus our own `{named}` forms, quoted per platform. An unrecognised
  `{…}` passes through untouched, so `awk '{print $1}'` still works, which is the whole reason
  for the rule.
- **The user screen** (`Ctrl+O`): what the commands printed, scrollable, capped, with each
  command as its heading.
- **Command history** (`Alt+F8`) as a list, and a small on-disk store that keeps it, the folder
  history and both panels' directories between sessions.
- **`Ctrl+G` applies a command to every selected file**, each file taking its turn as "the
  current file". They run in sequence rather than all at once, counted through in the status
  line, and the run stops at the first failure instead of pushing ninety-nine more over a
  broken one.

### The keys a FAR user reaches for

- **The selection matrix**: the modifier picks the criterion, not just the action. `Ctrl++` and
  `Ctrl+−` take everything with the extension of the file under the cursor, `Alt++` and `Alt+−`
  everything with the same name whatever its extension, `Alt+*` inverts folders too. One
  restore (`Ctrl+Shift+M`) undoes a whole sweep rather than one row.
- **`Shift+F5`, `Shift+F6`, `Shift+F8`** act on the item under the cursor whatever is selected —
  the case they exist for is a selection you forgot about. The list travels with the dialog, so
  selecting something else while it is open cannot widen the copy.
- **`Ctrl+F1` / `Ctrl+F2`** give the screen to one panel, and the focus goes with it.
- **`Ctrl+F8` / `Ctrl+F9`** sort by creation and access time. An entry whose filesystem does not
  keep the field sorts with the others that do not, at the old end, so the order stays total.
- **`Alt+Del` wipes**: the bytes are overwritten once, then the file is unlinked. The dialog
  says plainly what that cannot promise on an SSD, on a copy-on-write filesystem or with
  snapshots — including that the name survives in the directory and the journal.
- **`Alt+F6` creates a link** in the other panel, as a copy would: a relative symbolic link, the
  one that survives the tree being moved elsewhere. The engine does hard and absolute links
  too; the chooser is still to come.
- **Long names scroll sideways** with `Ctrl+Shift+←/→` (FAR's `Alt+←/→` is history here).

### The panel's own frame

- **Free space** on the device at the right-hand end of the bottom frame — the one thing that
  needs a platform call, and honest about saying nothing on a server or on Windows.
- **A scrollbar** in the right frame, drawn only when there is more than fits.
- **The panel's own status line** (`panel.status_line`), a rule and a row inside the frame with
  the item under the cursor. Off by default, because the screen-wide status bar says the same
  thing and two rows saying it wastes a row.

### One filter object

- The quick filter bar now takes the whole grammar: a comma or semicolon list, `*`, `?`,
  `[c-f]` ranges, `/regex/i`, an exclusion after `|`, and named groups (`<arc>`, `<temp>`,
  `<exec>`, nestable). A bare word still means "contains", so nothing anybody was already doing
  changed.
- It is **one value type** rather than three, the way FAR has it, and it already carries the
  conditions the find tool and the copy dialog will need: size ranges with binary and decimal
  suffixes, date ranges that can be relative in days, attributes as three states, and a
  hard-link condition. A mask that does not parse leaves the previous filter alone instead of
  flickering the panel empty.

### The viewer says what a binary is

- **Structure pane** (`F11`): beside the hex or disassembly rows, a pane that decodes the file —
  identity (format, architecture, bitness, kind, entry point), the header field by field with
  its raw bytes next to its meaning, and then the tables: ELF sections and segments, PE
  sections, Mach-O load commands and dylibs, the slices of a universal binary, the members of
  an `ar` archive, plus an ELF's interpreter and build id. `Tab` moves the keys between the
  bytes and the pane, `Enter` on a row takes the bytes to that offset.
- **Imports and exports**: an ELF's `DT_NEEDED` libraries and its `.dynsym` symbols with their
  binding and type, the PE import and export directories including forwarders and exports with
  no name at all, Mach-O's load-dylib commands and symbol table. The **.NET CLI header** too,
  with the runtime version, the flags in words, the entry-point token and the metadata root —
  and the identity line then says a PE is managed.
- **Symbols feed the disassembly**, and `Enter` on a `call` or `jmp` follows it, `Backspace`
  comes back, with the jumped-from stack on screen.
- It reads only the offsets that matter: an ELF keeps its section table at the end of the file,
  and our own 162 MB debug binary is decoded in four reads rather than loaded.
- Every offset is range-checked before it becomes a jump target, every table is capped, and a
  file that stops early fills the rows it can and says `truncated` — a corrupt binary is the
  normal case for this pane, not an error.
- **A hex row now fits the window**: never more bytes than the setting, never fewer than four,
  always a power of two so an offset stays aligned, and the structure pane takes the room left
  over instead of cutting the printable gutter in half.
- **`F6` crosses to the editor** on the line the bytes were showing, and `F6` in the editor
  crosses back to the byte the cursor was on. **`+` and `−`** walk to the next and previous file
  of the panel without going back to it, skipping directories and keeping the mode.
  **`Ctrl+F10`** brings the panel to whatever is being viewed. Decompiling moved to `Shift+F6`,
  since FAR's `F6` in the viewer means the editor.

### Jobs

- **Speed, elapsed and what is left** in the progress box. All three come from the average
  rather than the last tick, and the estimate is absent until there is enough to compute it: a
  number that jumps between four seconds and four hours is worse than no number.
- **Append** and **only newer** as copy policies, beside ask / overwrite / skip / rename.
- **Fixed, and it was a data-loss bug**: a move plans a copy and then a removal of every source,
  and the removals ran whatever happened to the copy. Answering "skip" to a conflict in a move
  left the destination alone *and deleted the source*. The runner now declines the removal of
  any source whose copy did not happen, and of any directory above it.

### Plugins

- `scmd-plug`: manifests discovered from disk, with themes, keymaps, associations, decompile
  rules and mask groups loaded from them, a version requirement checked against ours, and every
  path inside a plugin verified to stay inside it — relative, no `..`, and the canonical target
  inside the canonical directory, so a symlink cannot walk out either.
- `--list-plugins` says what was found, what is waiting for a host API, and what was refused and
  why.

### The screen looks like FAR's

- A **`far` theme**: FAR Manager's own default palette, element for element, read from
  `far/palette.cpp` and `far/colormix.cpp` of `FarGroup/FarManager` (BSD-3-Clause). Light cyan
  on blue panels, yellow for marked files and column titles, a cyan bar on the active panel's
  path, cyan menus with yellow shortcuts, light grey dialogs with cyan input fields, red warning
  dialogs.
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
- **Four things that were not FAR's, checked against its source and corrected**: the key bar's
  background between the labels is grey on black rather than the label's cyan; the totals turn
  yellow on cyan while something is selected; file colours are FAR's six default highlighting
  groups in FAR's order, so a hidden directory is cyan rather than white and `*.bak` is
  litter-coloured; and the size column says `Up` and `Folder`, localised and centred the way
  FAR fits them, instead of `UP--` and `FOLDER`. Nothing is bold any more either: FAR draws with
  console attributes, which have no bold.
- Every theme gained the keys those changes need (menu, menu bar, dialog fields and buttons,
  warning dialogs, column titles, the active panel's title, the scrollbar, the key bar's
  background, the selected totals, the `<temp>` group) — all seven built-in themes define every
  key, and a test keeps it that way.
- The key bar now uses FAR's own wording where FAR ships the language: `UserMn`, `MkFold`,
  `ConfMn`, and their Italian, German, Spanish and Russian forms.

### Also

- **Quoting on Windows was wrong**: the backslash was in the set of characters that need
  quoting, which is right on Unix (it escapes the next one) and wrong on Windows (it *is* the
  path separator) — so every path a metasymbol expanded came back wrapped in quotes, which
  `cmd` then reads differently. Each platform has its own set now.
- A panel can be **a list of files rather than a directory**: one method decides whether a
  row's path is the panel's own path plus a name or something it was told, and every
  operation goes through it. Nothing reaches it from the keyboard yet — it is what find
  results and `panelize` will be shown in — but it is in this build.
- Four tests that only held on Unix now hold on both platforms, and one that needed a
  Windows privilege no longer insists on it.

### Documentation

- `docs/04-far-inventory.md`: FAR 3.x read from its own repository — 283 help topics, 311
  settings keys — with every feature marked against what we do, the three grammars worth
  matching exactly (panel columns, file masks, metasymbols), and a section on exactly how close
  the screen now is and what is still different.
- `docs/02-features.md` §5b: Hiew release VIII's published feature list, item for item, against
  what we have — what is shipped, what is in the backlog, and the two we are not doing.
- `docs/spec/02-panels.md` §9b (what FAR draws in a panel's frame and what each element costs),
  `docs/spec/06-tools.md` §7.3 (the four decisions apply-command had to take), and
  `docs/spec/10-menus-and-dialogs.md` for the menus and the dialogs.
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
