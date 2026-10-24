# Feature inventory

The union of FAR Manager, Total Commander, Krusader, Midnight Commander, Double Commander and
the modern TUIs (yazi, MTN2), normalised into one list with a target milestone.

Milestones: **M1** = usable daily driver (MVP), **M2** = feature parity with mc + basic FAR,
**M3** = parity with FAR/TC, **M4** = beyond (new ideas).

For FAR specifically, [`04-far-inventory.md`](04-far-inventory.md) is the harder check: FAR 3.x
read from its own repository — 283 help topics, 311 settings keys — with each feature marked
against what SupremeCommander actually does today.
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
| 1.11b | Column titles row (`Name │ Size │ Date Time`), with the sort-mode letter in the corner — lower case ascending, upper case descending — and a `*` beside it while a filter is on | FAR | M1 |
| 1.12 | Function-key bar (`F1..F12`, follows modifiers: Shift/Alt/Ctrl rows) | NC | M1 |
| 1.13 | Sort by name/ext/size/mtime/ctime/atime/owner/unsorted, reversible, group dirs first, selected first, natural ("version") sort, locale-aware collation | FAR/TC | M1 core, M2 full |
| 1.14 | Secondary sort criteria chain (tie-breakers, configurable): per sort mode an ordered set, with its own editor (add, remove, replace, direction, inherit, reorder, reset) | FAR | M3 |
| 1.14b | Sort groups: named mask groups that sort ahead of or behind everything else, ordered, applied on top of the sort mode | FAR (`Shift+F11`) | M2 |
| 1.14c | Sort by creation time, access time, owner and description, besides name/ext/size/write time | FAR (`Ctrl+F8…F11`) | M2 |
| 1.15 | Filters: per-panel display filter, quick filter bar, filter by mask/size/date/attrs, named filter sets, filter menu | FAR/KRU | M2 |
| 1.15b | A column grammar for view modes: per column a property letter with modifiers (name with selection marks / no extension / right-aligned, extension, size / allocated size / stream size with grouping, decimal units, fractions, four date-and-time pairs with brief and month-name forms, attributes, description, owner with domain, hard-link count, stream count), widths absolute or automatic or a percentage, repeated groups giving newspaper stripes, and the same grammar for the panel status line and for search results | FAR | M2 |
| 1.15c | Per view mode: fullscreen, align file and folder extensions, folders uppercase, files lowercase, uppercase names shown lowercase | FAR | M2 |
| 1.16 | Hidden/system files toggle (`Ctrl+H`). A dotfile counts as hidden on every platform, and Windows adds the hidden and system attributes, so the key means the same thing everywhere | all | M1 |
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
| 4.4b | **Remote file in the viewer and the editor**: `F3` views a fetched copy, `F4` edits it and `F2` writes it back (rename into place, change check, the local copy named if it fails) | FAR NetBox | M1 |
| 4.5 | **SFTP/SSH** panels: listing, navigation, mkdir, rename, delete, symlinks, mode and mtime | far2l NetRocks/termscp | M1 |
| 4.5b | FTP/FTPS, WebDAV, SMB, NFS, S3-compatible object storage, MTP/ADB (Android), rclone bridge | far2l NetRocks/termscp | M3 |
| 4.6 | **Connection manager** (`Alt+N`): saved `[[site]]` list with groups, agent / key file / asked password, host-key verification with a trust-on-first-use prompt, one pooled connection per host, explicit disconnect | TC/KRU | M1 (OS keyring, `ProxyJump`, charset M3) |
| 4.7 | **Transfers through the local operation engine**: copy and move up, down, between two servers and inside one, with progress, conflicts and cancellation; a `sftp://` URL in `Ctrl+L` connects | TC | M1 (background queue, resume and bandwidth limit M3) |
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
| 5.13 | **JVM bytecode and .NET IL in process**: class files listed javap-style with the constant pool resolved; managed PEs listed ildasm-style with metadata-resolved tokens and `#US` string literals | NEW | M1 |
| 5.14 | Still external: Python bytecode, aarch64/riscv decoding, .NET signatures and generics | NEW | M3 |
| 5.15 | **Structure pane** (`F11`): what the file is and where anything inside it is — format, architecture, kind, entry point, the header field by field, sections and segments, imports and exports, the interpreter and build id; `Tab` moves the focus, `Enter` takes the hex or disassembly pane to that offset. Read through a seeking reader, so a section table at the end of a 3 GB file costs two seeks | NEW | M1 |
| 5.16 | Following a branch in the disassembly (`Enter` on a `call`/`jmp`, `Backspace` back, the jumped-from stack on screen), and `Alt+F8` taking a virtual address as well as an offset | NEW | M2 |
| 5.17 | Raw devices: a block device or a partition opened as a file, size from an `ioctl`, read-only unless told otherwise | NEW | M2 |
| 5.10 | Next/previous file in panel without leaving the viewer (`+`/`-`) | FAR | M2 |

## 5b. Hiew release VIII, feature by feature

Hiew is proprietary and the rule from `docs/01-licensing.md` holds: its *published* list of
what it does is fair to read and answer, the program is not to be reverse engineered or
decompiled, and nothing is copied from its help text or its wording. What follows is that
published list, item for item, against what we have.

| Hiew VIII says | Us |
| --- | --- |
| view and edit files of unlimited size in text, hex and disassembler modes | **Viewing: shipped** — the viewer streams and seeks, so a 40 GB file opens at once in text, dump, hex or disassembly. Editing a file of unlimited size is the hex editor below: deliberately in `scmd-edit`, because the viewer's read-only contract is exactly what lets it seek through 40 GB |
| x86-64 disassembler and assembler, AVX included | **Disassembler: shipped** (`iced-x86`, which decodes AVX through AVX-512). **Assembler: backlog** — one instruction at a time, writing into a patch list rather than straight to disk (SPEC 05 §2.7) |
| physical and logical drive view and edit | **Backlog, SPEC 05 §1.10.** On Linux and macOS a block device is a file: the size comes from an `ioctl` rather than `stat`, it opens read-only, and a write is confirmed loudly. Windows needs its own path |
| NE, LE, LX, PE/PE32+, ELF/ELF64, Mach-O, TE/TE64 | **PE/PE32+, ELF/ELF64, Mach-O (thin and universal) and `ar`: shipped** in the structure pane, including imports, exports and the .NET CLI header. **NE, LE, LX, TE/TE64: backlog** — the DOS and EFI families, rainy-day additions to the same pane |
| Netware Loadable Modules (NLM, DSK, LAN…) | **No.** A format with no living platform behind it; if anybody asks, it belongs in a plugin, not the core |
| navigate direct call/jmp with a single click | **Shipped** — `Enter` on a `call` or `jmp` follows it, `Backspace` comes back, and the jumped-from stack is on screen |
| search by instruction pattern in the disassembler | **Backlog, new line.** Search the decoded stream rather than the bytes: a mnemonic, an operand shape, a wildcard byte. Needs the search engine to run over decoded instructions, which the disassembly pane already produces |
| built-in 64-bit decrypt/crypt system | **Backlog, new line.** As a block operation (XOR and a named cipher over a block), not as a promise of security: the dialog has to say what it is |
| built-in 64-bit calculator | **Backlog, new line.** Hex, decimal, octal, binary, with the bitwise operators and conversion between bases — the thing a hex editor is asked for every ten minutes. Cheap and self-contained |
| file operations on blocks: read, write, fill, copy, move, insert, delete, crypt | **Backlog, SPEC 05 §2.6.** `F3` marks, `F12` operates, the large ones through the job engine, each showing the resulting length before it happens |
| search and replace across multiple files | **Backlog, new line.** The find tool (SPEC 06 §1) gets the files; replacing in them is a job with its own confirmation, a dry run, and a count per file |
| keyboard macros | **Backlog**, with the rest of the macro engine (SPEC 08). The command vocabulary is the right foundation: a macro is a list of commands, not of keystrokes |
| unicode/utf8 support | **Shipped** throughout: the panel, the viewer, the editor and the command line are UTF-8, with the codepage list for files that are not |
| Hiew External Module (HEM) support | **Our answer is the plugin system** (SPEC 08): manifests on disk today, a host API when there is something worth hosting |
| ArmV6 disassembler | **Backlog** — AArch64 first, since a viewer that only decodes x86 is half a tool on an Apple Silicon machine; a decoder crate rather than new design |

Two things on this list we have and Hiew does not, so they are worth saying out loud: the
structure pane decodes imports, exports and the .NET header beside the raw bytes, and the
decompilers (WebAssembly in process, JVM and .NET IL in process, external tools by rule) turn
a binary into something readable rather than only into instructions.

## 6. Editor (`F4`)

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 6.1 | Full-screen editor, 256 MiB open limit, read-only files opened and marked | FAR | M1 (multiple open files M2) |
| 6.2 | Block operations: stream blocks and **column/vertical blocks**, copy/cut/paste (a column paste pads short lines), indent/outdent | FAR | M1 (block-to-cursor, append-to-clipboard, OS clipboard M2) |
| 6.2b | **Hex mode** (`F4` on a binary, `Ins` for insert/overwrite): byte editing with a patch list, the editor's own undo, and the same temp-file-and-rename save; in-place writing only when explicitly chosen | NEW | M2 |
| 6.2c | **Byte blocks** (`F3` marks, `F12` operates): write out, read in, fill, XOR with a key, copy or move from elsewhere, insert, delete — large ones through the job engine, each showing the resulting length first | NEW | M2 |
| 6.2d | **Inline assembler** (`F9`): one instruction, showing the bytes it produces, the instruction it replaces and whether the length changed, written into the patch list rather than to disk | NEW | M3 |
| 6.3 | Undo/redo grouped per command | FAR | M1 (typing-burst grouping, configurable depth and persisted positions M2) |
| 6.4 | Search/replace: plain search with wrap, replace once, replace all (the engine already does regex and case-insensitive; the dialog that exposes them is M2) | FAR | M1; whole word, capture refs, "preserve style", all-matches list M2/M3 |
| 6.5 | Go to `line` or `line:column`, bookmarks 0..9 | FAR | M1 (matching-bracket jump M2) |
| 6.6 | Keep the original encoding, EOL, BOM and final newline on save | FAR | M1 (explicit conversion commands M2) |
| 6.7 | Syntax highlighting (tree-sitter), bracket matching, auto-indent, tabs/spaces policy, trailing-space trim | MTN2/NEW | M3 |
| 6.8 | Insert file name / path / current date, character by code, macros | FAR | M3 |
| 6.9 | External editor delegation (`$EDITOR`, per-type associations) | all | M1 |
| 6.10 | LSP-lite: completion/diagnostics for the open file (optional plugin) | NEW | M4 |
| 6.11 | **Atomic save** (`name.scmd-tmp` + rename, permissions carried over) and a save-as prompt | NEW | M1 |
| 6.12 | **External-change detection**: a file changed on disk is never overwritten without a confirmation; reload on demand | FAR | M1 (diff-on-conflict M3) |
| 6.13 | `Shift+F4` edits a new file, saved into the active panel's directory | FAR | M1 |
| 6.14 | Line editing: delete, duplicate, move up/down, delete to end of line, delete word left, auto-indent, smart home, tab expansion | FAR | M1 (join, sort, unique, reverse, number M2) |

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
| 9.1 | Human-editable config files (TOML) + in-app settings dialogs (System, Panel, Interface, Confirmations — every row bound to a config key and applied through the same code as `--set`, SPEC 10 §3); portable mode (config next to the binary) | MTN2 | M1 |
| 9.2 | Full keymap remapping, multiple **keymap profiles** (`far`, `tc`, `mc`, `nc`, `vi`) switchable at runtime | KRU/MTN2 | M1 (far+mc), M2 rest |
| 9.3 | Themes: 7 built-in (`commander`, `far` — FAR's own default palette element for element, `black`, `white`, `matrix`, `tron`, `blade`) + `[[theme]]` blocks in `scmd.toml` or from plugins, hot-reloaded, `inherit`-able, exportable with `--export-theme`; 24-bit colour with 256/16/mono degradation | MTN2 | M1 |
| 9.4 | Localisation (fluent-based): EN, IT, DE, FR, ES, RU, ZH-Hans; UI + help + key names, runtime switchable from the `Languages` list, drop-in language dirs | FAR/MTN2 | M2 |
| 9.5 | CJK/wide-character correct layout, combining marks, RTL-safe truncation, emoji width | MTN2 | M2 |
| 9.6 | Accessibility: no-colour mode, screen-reader-friendly output mode, configurable focus indication | NEW | M3 |
| 9.7 | Session save/restore (tabs, paths, selections, viewer positions) | TC | M2 |
| 9.8 | Settings search ("where is that option?") | NEW | M3 |
| 9.9 | No telemetry, no network calls except explicit user actions; optional update check off by default | MTN2 | M1 |
| 9.10 | Menu bar (`F9`) with FAR's five menus; every entry is a command from the vocabulary, the shortcut shown is read from the keymap in force, and an entry whose feature is missing is drawn dim and says so (SPEC 10 §2) | FAR | M1 |
| 9.11 | `Save setup` (`Shift+F9`) writes the configuration as it stands, including themes that came from the file | FAR | M1 |
| 9.13 | Session editor for stored connections (`Alt+N` → `Ins`/`F4`): name, host, port, user, authentication, key file, remote folder, group, keep-alive, compression — validated before it is stored, `Shift+F9` makes it permanent (SPEC 10 §3.5) | FAR (NetBox) | M1 |
| 9.12 | Choose-one lists: languages, themes, drives and open connections (`Alt+F1`/`Alt+F2`), folder history (`Alt+F12`), sort modes (`Ctrl+F12`, where re-choosing the current order reverses it) (SPEC 10 §4) | FAR | M1 |

## 9b. What a FAR user expects and we had not written down

Found by reading FAR's own help and configuration tables; the detail is in
[`04-far-inventory.md`](04-far-inventory.md) and each line has a backlog entry.

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 9b.1 | The selection matrix: the modifier picks the criterion — mask, same extension as the cursor, same name, everything — across select, deselect and invert, with separate rules for folders | FAR | M2 |
| 9b.2 | `Shift+F5`/`Shift+F6`/`Shift+F8`: copy, move and delete the item under the cursor, ignoring the selection | FAR | M1 |
| 9b.3 | Filters as one shared object used by the panel display, by find and by copy: mask, size range (binary and decimal suffixes), date range on any of four timestamps, absolute or relative in days, attributes as three-state include/exclude/ignore, more-than-one-hard-link | FAR/KRU | M2 |
| 9b.4 | Mask groups (`<arc>`, `<temp>`, `<exec>`, nestable) and the full mask grammar: character ranges, `/regex/i`, `include|exclude` | FAR | M2 |
| 9b.5 | Four histories (commands, folders, view/edit, dialogs), each with a size and a lifetime in days, persisted, with items that can be locked so new entries do not evict them | FAR | M2 |
| 9b.6 | File descriptions (`descript.ion`, `files.bbs`): edit, display as a column, carry through copy/move/rename/delete, show the folder's own description file in the info panel | FAR/NC | M3 |
| 9b.7 | Copy dialog depth: several destinations, append to existing, only-newer as a policy, copy symlink contents, a filter on the operation, destination from the folder tree, speed and ETA | FAR | M2 |
| 9b.8 | Metasymbol language for the user menu, file associations and apply-command: name forms, the selected-file list as a file or inline, path forms, resolved paths, the description, a prompt with history, panel-switching prefixes, and quoting/case/encoding modifiers | FAR | M2 |
| 9b.9 | Command prefixes on the command line (`view:`, `edit:`, `goto:`, `clip:`, `whereis:`, `link:`, `run:`), as FAR's `farcmds` does | FAR | M2 |
| 9b.10 | Internal shell commands that need no shell: `cls`, `cd`/`chdir` with a configurable `cd ~`, `chcp`, `set`, nestable `if [not] exist` / `if [not] defined`, `pushd`/`popd`/`clrd`, `title`, `exit` | FAR | M2 |
| 9b.11 | Autocomplete with four separately switchable sources (filesystem, history, `PATH`, environment), each three-state, as a list or appended inline | FAR | M2 |
| 9b.12 | Screen grabber: select any region of the screen, stream or block, extend, move, append to the clipboard | FAR (`Alt+Ins`) | M3 |
| 9b.13 | Viewer ⇄ editor at the same position, jump to the open file in the panel, next/previous file of the panel from inside the viewer | FAR (`F6`, `Ctrl+F10`, `Gray ±`) | M2 |
| 9b.14 | Remember per file: position, code page, view mode, wrap mode, bookmarks | FAR | M2 |
| 9b.15 | Sorting collation as three settings — collation (ordinal/invariant/linguistic), digits-as-numbers, case sensitivity — rather than one "natural sort" flag | FAR | M3 |
| 9b.16 | A searchable settings browser, because the setting count only grows (FAR: 311) | FAR (`far:config`) | M3 |
| 9b.17 | Dialog framework: history in edit fields with lockable items, autocomplete, three-state checkboxes, movable dialogs, click outside to accept or cancel | FAR | M2 |
| 9b.18 | Menus that filter as you type, with a lock and horizontal scrolling | FAR (`RAlt`) | M3 |

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
