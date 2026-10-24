# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[semantic versioning](https://semver.org/), which before 1.0 means anything may still move.

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
