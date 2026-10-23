# Feature inventory

The union of FAR Manager, Total Commander, Krusader, Midnight Commander, Double Commander and
the modern TUIs (yazi, MTN2), normalised into one list with a target milestone.

Milestones: **M1** = usable daily driver (MVP), **M2** = feature parity with mc + basic FAR,
**M3** = parity with FAR/TC, **M4** = beyond (new ideas).
Origin: `FAR`, `TC`, `KRU` (Krusader), `MC`, `DC` (Double Commander), `MTN2`, `YAZI`, `NEW`.

## 1. Panels

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 1.1 | Two equal panels, active/passive, `Tab` to switch | NC | M1 |
| 1.2 | Panel layouts: side-by-side and stacked (horizontal split) | KRU | M2 |
| 1.3 | Resizable split, saved per session; restore defaults | FAR | M2 |
| 1.4 | View modes: brief (multi-column names), medium, full (size/date), wide, detailed, owners, links, custom columns | FAR | M1 brief+full, M2 rest |
| 1.5 | User-definable column sets (which fields, widths, alignment, formats) | FAR/TC | M3 |
| 1.6 | Tabs per panel, named, persisted, lockable, duplicate/move-to-other-side | TC/MTN2 | M2 |
| 1.7 | Quick view panel (preview of the file under the cursor in the other panel) | FAR/MC | M2 |
| 1.8 | Info panel (drive/fs info, disk usage, selected size, archive/VFS metadata) | FAR/MC | M2 |
| 1.9 | Tree panel (directory tree in a panel, synced with the other side) | FAR/MC | M2 |
| 1.10 | More than two panels / floating panel windows | DN/MTN2 | M4 |
| 1.11 | Status line (name, size, date of the current item) and total/selected counters | FAR | M1 |
| 1.12 | Function-key bar (`F1..F12`, follows modifiers: Shift/Alt/Ctrl rows) | NC | M1 |
| 1.13 | Sort by name/ext/size/mtime/ctime/atime/owner/unsorted, reversible, group dirs first, selected first, natural ("version") sort, locale-aware collation | FAR/TC | M1 core, M2 full |
| 1.14 | Secondary sort criteria chain (tie-breakers, configurable) | FAR | M3 |
| 1.15 | Filters: per-panel display filter, quick filter bar, filter by mask/size/date/attrs, named filter sets, filter menu | FAR/KRU | M2 |
| 1.16 | Hidden/system files toggle; show dotfiles | all | M1 |
| 1.17 | Branch view (flat recursive listing of the current tree) | TC | M2 |
| 1.18 | Panelize: fill a panel with the result of a search or an external command | MC | M2 |
| 1.19 | Directory sizes computed on demand (`Space`/`F3` on a dir) and in background | FAR/TC | M1 |
| 1.20 | Long-path support (`\\?\` on Windows, >260 chars), UNC paths | FAR/MTN2 | M1 |
| 1.21 | Per-panel encoding/codepage for names and previews | MC/FAR | M3 |
| 1.22 | Icons / nerd-font glyphs per file type, optional | YAZI | M2 |
| 1.23 | Colour-by-file-type rules (mask, attrs, size, age) — "highlight groups" | FAR/TC | M2 |
| 1.24 | Panel history (back/forward), directory history, folder shortcuts/bookmarks (0..9 + named), hotlist | FAR/TC/MC | M1 history, M2 rest |
| 1.25 | Auto-refresh via filesystem watch (inotify/FSEvents/ReadDirectoryChangesW), manual `Ctrl+R` | TC/MTN2 | M2 |

## 2. Navigation & selection

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 2.1 | Enter = run/chdir/enter archive; parent dir via `..`; root jump | all | M1 |
| 2.2 | Fast find / quick search inside the panel (type to jump, wildcards, cycle matches) | FAR/MC | M1 |
| 2.3 | Fuzzy "go to anywhere" (zoxide-style frecency + fuzzy path jump) | broot/NEW | M3 |
| 2.4 | Drive/mount menu (`Alt+F1/F2`), MountMan-style mount/unmount, removable eject | FAR/KRU | M2 |
| 2.5 | Select/deselect: `Ins`, `Shift+arrows`, by mask (`+`/`-`/`*`), by extension, by name, same-name/different-size, invert, select all, restore previous selection | FAR/TC | M1 |
| 2.6 | Selection by regex, by size/date/attribute predicates, from clipboard list | FAR/KRU | M3 |
| 2.7 | Mark-and-act across directory changes (persistent selection set / "staging area") | NEW | M4 |
| 2.8 | Sync panel paths (`=`), open current dir in the other panel, swap panels | all | M1 |
| 2.9 | Command line under the panels with history, completion, insert-current-name macros | FAR/MC | M1 |
| 2.10 | Mouse: click, drag-select, wheel scroll, drag&drop between panels and with the OS | FAR/TC | M2 (TUI), M3 (OS DnD) |

## 3. File operations

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 3.1 | Copy / move / rename (`F5`/`F6`), same-panel copy (`Shift+F5`), rename in place (`Shift+F6`) | all | M1 |
| 3.2 | Delete to trash and permanent delete; wipe (overwrite then delete) | FAR | M1 delete, M2 trash/wipe |
| 3.3 | Create directory (`F7`), create file (`Shift+F4`), create nested path `a/b/c` in one go | all | M1 |
| 3.4 | Hard links, symlinks (absolute/relative), junctions/reparse points, link targets shown and followed | FAR/MC | M2 |
| 3.5 | Attributes/permissions dialog: chmod (numeric + rwx matrix), chown, Windows attrs (RASH, compressed, encrypted), timestamps (m/a/c/birth), recursive apply | FAR/MC | M2 |
| 3.6 | Extended attributes / ADS (NTFS streams), xattr on Linux/macOS, resource forks | FAR | M3 |
| 3.7 | Operation engine: queue of jobs, parallel or serial, pause/resume/cancel, background with progress, speed/ETA, per-job log | TC/KRU/MTN2 | M1 basic, M2 queue |
| 3.8 | Conflict resolution: overwrite / skip / rename / append / newer-only / size-differs, "apply to all", per-conflict dialog | all | M1 |
| 3.9 | Verify after copy (hash compare), preserve mode/owner/times/xattrs, sparse files, reflink/`copy_file_range`/CoW, `FSCTL` server-side copy | TC/NEW | M2 |
| 3.10 | Resume interrupted copy of large files | TC | M3 |
| 3.11 | Multi-rename tool: patterns (`[N]`, `[E]`, counters, date, regex, case changes), live preview, undo | TC/KRU | M2 |
| 3.12 | Split / combine files; CRC/SHA checksum create & verify (`.sfv`, `.md5`, `.sha256`) | TC/KRU | M2 |
| 3.13 | Apply command to each selected file (`Ctrl+G` style), with `%`-macros | FAR | M2 |
| 3.14 | Compare directories (by name/size/date/content), synchronise directories with preview and one-way/two-way modes | TC/KRU | M2 |
| 3.15 | Compare files by content (internal diff viewer, side-by-side, word-level) | MC/KRU | M3 |
| 3.16 | Change encoding / line endings in bulk (CRLF↔LF, codepage recode) | MC | M3 |
| 3.17 | File descriptions/comments (`descript.ion`, `files.bbs`) | FAR | M3 |
| 3.18 | Undo for destructive operations where the FS allows (trash-based) | NEW | M3 |
| 3.19 | Elevated operations (UAC / `sudo` / `pkexec` helper) for privileged paths | FAR/KRU | M3 |

## 4. VFS: archives, remote, special

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 4.1 | Archives browsable as directories, read: zip, tar(+gz/bz2/xz/zst), 7z, rar(read-only), cab, iso, ar/deb/rpm, cpio, lha, arj, zip64, encrypted zip/7z | FAR/TC/MC | M2 (zip/tar/7z), M3 rest |
| 4.2 | Archive write: create/add/delete/update in zip, 7z, tar family; pack dialog with format options (level, solid, encryption, split volumes) | TC | M2/M3 |
| 4.3 | Pack/unpack shortcuts (`Alt+F5`/`Alt+F6`/`Shift+F1`/`Shift+F2`), test archive, extract-here vs extract-to | FAR/TC | M2 |
| 4.4 | Open a file inside an archive in the viewer/editor; edit-in-place writes back | FAR | M3 |
| 4.5 | SFTP/SSH, FTP/FTPS, WebDAV, SMB, NFS, S3-compatible object storage, MTP/ADB (Android), rclone bridge | far2l NetRocks/termscp | M3 (sftp M2) |
| 4.6 | Connection manager: saved sites, keys/agent, passwords via OS keyring, per-site charset/transfer mode, proxy | TC/KRU | M3 |
| 4.7 | Background transfer queue shared with the local operation engine; resume, bandwidth limit | TC | M3 |
| 4.8 | Temporary panel (collect unrelated files into a virtual panel and act on them) | FAR `tmppanel` | M2 |
| 4.9 | Special VFS: process list, registry (Windows), services, `/proc`-style views, package contents, git status/worktree view | FAR/far2l | M4 |
| 4.10 | Trash as a browsable VFS | NEW | M3 |
| 4.11 | VFS API available to plugins (so a plugin can add a scheme) | FAR/TC(WFX) | M3 |

## 5. Viewer (`F3`)

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 5.1 | Text mode with and without wrap (char/word wrap), horizontal scroll, arbitrary line length | FAR | M1 |
| 5.2 | Hex mode and dump mode, configurable bytes per row | FAR | M2 |
| 5.3 | Encoding detection + manual codepage selection, BOM handling, invalid-byte display | FAR/MC | M2 |
| 5.4 | Search (plain, case-insensitive, whole word, regex, hex pattern), search next/previous, highlight all hits, copy selection | FAR | M1 plain, M2 regex/hex |
| 5.5 | Go to offset/line/percent; bookmarks 0..9; "undo position" | FAR | M2 |
| 5.6 | Streaming/mmap for huge files (GB+) without loading; follow mode (`tail -f`) | FAR/MTN2 | M2 |
| 5.7 | Syntax highlighting in the viewer; Markdown rendering; tabs expansion | MTN2 | M3 |
| 5.8 | Previews: images (sixel/kitty/iterm protocols), video thumbnails, PDF, archives, media metadata, office/JSON/CSV pretty print — pluggable pipeline with timeouts | YAZI | M3 |
| 5.9 | Filtered view: pipe the file through a command and view the output | MC | M3 |
| 5.11 | **Disassembly** (`F5`): ELF/PE/Mach-O/raw recognition, sections and symbols, x86/x86-64 decoding with symbol headings, instruction-accurate scrolling, raw sweep for files over the parse limit | NEW | M1 |
| 5.12 | **Decompilation** (`F6`): WebAssembly to its text format in process; `[[decompile]]` rules call installed tools (javap, ilspycmd, python -m dis, objdump, retdec) and the output is viewed as text | NEW | M1 |
| 5.13 | More formats in process: JVM bytecode, .NET IL, Python bytecode, aarch64/riscv decoding | NEW | M3 |
| 5.10 | Next/previous file in panel without leaving the viewer (`+`/`-`) | FAR | M2 |

## 6. Editor (`F4`)

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 6.1 | Full-screen editor, multiple files open, big-file support, read-only lock | FAR | M2 |
| 6.2 | Block operations: stream blocks and **column/vertical blocks**, indent/outdent, copy/move block to cursor, append to clipboard | FAR | M2 |
| 6.3 | Undo/redo with configurable depth, persistent position/bookmarks per file | FAR | M2 |
| 6.4 | Search/replace: case, whole word, regex (with capture refs), fuzzy, "preserve style" (camelCase/snake_case aware), all-matches list | FAR | M2/M3 |
| 6.5 | Go to line:column, bookmarks 0..9, jump to matching bracket | FAR | M2 |
| 6.6 | Encoding and EOL conversion on save, keep original EOL, optional BOM | FAR | M2 |
| 6.7 | Syntax highlighting (tree-sitter), bracket matching, auto-indent, tabs/spaces policy, trailing-space trim | MTN2/NEW | M3 |
| 6.8 | Insert file name / path / current date, character by code, macros | FAR | M3 |
| 6.9 | External editor delegation (`$EDITOR`, per-type associations) | all | M1 |
| 6.10 | LSP-lite: completion/diagnostics for the open file (optional plugin) | NEW | M4 |

## 7. Tools & dialogs

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 7.1 | Find files: mask, content (text/regex/hex), size/date/attr filters, archives included, encoding list, results panel, panelize results, "find folder" | FAR/TC | M2 |
| 7.2 | Grep-like content search with result navigation and jump-to-match in the editor | NEW | M2 |
| 7.3 | Disk usage / occupied space view (per-dir sizes, sortable, graphical bars) | KRU/TC | M2 |
| 7.4 | User menu (`F2`) with user-defined entries, submenus, hotkeys, per-directory menus | FAR/MC | M2 |
| 7.5 | File associations by mask: view/edit/execute/alternative commands | FAR/MC | M2 |
| 7.6 | Command history, view/edit history, folder history — all searchable and persisted | FAR | M1 |
| 7.7 | Macro recorder/player, keyboard macros with conditions, assignable to any key | FAR/far2m | M3 |
| 7.8 | Scripting console (Lua) with the full command vocabulary | far2m/YAZI | M3 |
| 7.9 | Print files / print manager | FAR | M4 |
| 7.10 | Screen grabber (copy a rectangle of the screen to clipboard) | FAR | M4 |
| 7.11 | Learn-keys / terminal key diagnostics | MC | M2 |
| 7.12 | Help system: context-sensitive `F1`, searchable, localised, offline | FAR/KRU | M2 |

## 8. Shell & terminal integration

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 8.1 | Command line with the panel as context; `%`-macros for current/other panel, names, paths, selections | FAR/MC | M1 |
| 8.2 | Run a command and keep the output visible (`Ctrl+O` user screen) | FAR/MC | M1 |
| 8.3 | Real subshell / PTY: persistent shell session sharing the cwd with the panel | MC/MTN2 | M2 |
| 8.4 | Embedded terminal pane(s), tabs of terminals, alternate-screen apps inside the pane | MTN2/KRU | M3 |
| 8.5 | Shell integration both ways: `cd` in the shell moves the panel, panel moves the shell | MC/NEW | M2 |
| 8.6 | Run as administrator / another user | FAR | M3 |
| 8.7 | Output search (`Ctrl+Shift+F`) and output capture into a panel | MTN2 | M3 |
| 8.8 | Open OS file manager / terminal / shell menu for the current item | TC/KRU | M2 |

## 9. Configuration, UI, i18n

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 9.1 | Human-editable config files (TOML) + in-app settings dialogs; portable mode (config next to the binary) | MTN2 | M1 |
| 9.2 | Full keymap remapping, multiple **keymap profiles** (`far`, `tc`, `mc`, `nc`, `vi`) switchable at runtime | KRU/MTN2 | M1 (far+mc), M2 rest |
| 9.3 | Themes: 6 built-in (`commander`, `black`, `white`, `matrix`, `tron`, `blade`) + `[[theme]]` blocks in `scmd.toml` or from plugins, hot-reloaded, `inherit`-able, exportable with `--export-theme`; 24-bit colour with 256/16/mono degradation | MTN2 | M1 |
| 9.4 | Localisation (fluent-based): EN, IT, DE, FR, ES, RU, ZH-Hans; UI + help + key names, runtime switchable, drop-in language dirs | FAR/MTN2 | M2 |
| 9.5 | CJK/wide-character correct layout, combining marks, RTL-safe truncation, emoji width | MTN2 | M2 |
| 9.6 | Accessibility: no-colour mode, screen-reader-friendly output mode, configurable focus indication | NEW | M3 |
| 9.7 | Session save/restore (tabs, paths, selections, viewer positions) | TC | M2 |
| 9.8 | Settings search ("where is that option?") | NEW | M3 |
| 9.9 | No telemetry, no network calls except explicit user actions; optional update check off by default | MTN2 | M1 |

## 10. Extensibility

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 10.1 | Command vocabulary (`cmd.panel.copy`, …) that every key, menu entry, macro and plugin targets | DC (`cm_*`) | M1 |
| 10.2 | Lua scripting API (panels, selection, jobs, dialogs, VFS, events) | far2m/YAZI | M3 |
| 10.3 | WASM plugins (WASI + a component interface): VFS providers, panels, viewers, commands, highlighters — sandboxed, portable | MTN2 | M3 |
| 10.4 | Native plugin ABI (stable C ABI) for performance-critical/OS-specific plugins | FAR | M4 |
| 10.5 | Plugin manager: discovery, install from a registry, versions, permissions | YAZI | M4 |
| 10.6 | Event hooks (on-copy, on-enter-dir, on-select) for automation | NEW | M3 |
| 10.7 | Headless/CLI mode: run commander operations from scripts (`sc copy --queue …`) | NEW | M4 |

## 11. Platform & quality

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 11.1 | Linux, macOS, Windows first-class; FreeBSD best-effort | — | M1 |
| 11.2 | TUI backend (any terminal, works over SSH/tmux) as the primary UI | — | M1 |
| 11.3 | **No GUI backend, ever** — strictly TUI; terminal extras (inline images, kitty keyboard, OSC 52) are capability-detected, not a second UI | decision | — |
| 11.4 | Single static binary, no installer required; packages: `.deb`, `.rpm`, Homebrew, Scoop/winget, AUR, Flatpak | — | M2 |
| 11.5 | Startup under ~50 ms, listing of 100k entries without blocking the UI | YAZI | M1 (budget), enforced in CI |
| 11.6 | Crash safety: no data loss on operation interruption; journaled job state | NEW | M2 |
| 11.7 | Tests: unit + golden-screen snapshot tests + VFS integration + fuzzed archive/path parsing | — | M1 |
