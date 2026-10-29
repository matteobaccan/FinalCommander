# Feature inventory

The union of FAR Manager, Total Commander, Krusader, Midnight Commander, Double Commander and
the modern TUIs (yazi, MTN2), normalised into one list with a target milestone.

Milestones: **M1** = usable daily driver (MVP), **M2** = feature parity with mc + basic FAR,
**M3** = parity with FAR/TC, **M4** = beyond (new ideas).

**How to read the `M` column**, as of 0.10.0: it says which milestone *owns* a feature, not
whether it is done. M1 shipped long ago, so an `M1` row is a feature the program has. M2 is
the milestone in progress, which is why its rows are a mixture; a row that landed after this
table was written says so instead — struck through, with the version it landed in. When a
row's own text and the version note disagree, the version note is the newer fact.
`docs/BACKLOG.md` is the list of what is *not* there, each line with the reason, and
[`CHANGELOG.md`](../CHANGELOG.md) is what has landed.

For FAR specifically, [`04-far-inventory.md`](04-far-inventory.md) is the harder check: FAR 3.x
read from its own repository — 283 help topics, 311 settings keys — with each feature marked
against what FinalCommander actually does today.
Origin: `FAR`, `TC`, `KRU` (Krusader), `MC`, `DC` (Double Commander), `MTN2`, `YAZI`, `NEW`.

## 1. Panels

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 1.1 | Two equal panels, active/passive, `Tab` to switch | NC | M1 |
| 1.2 | Panel layouts: side-by-side and stacked (horizontal split) | KRU | M2 |
| 1.3 | Resizable split, saved per session; restore defaults | FAR | M2 |
| 1.4 | ~~View modes: brief (multi-column names), medium, full (size/date), wide, detailed, owners~~, links, custom columns | FAR | **shipped**: the five FAR has on `Ctrl+1..5`, with the extension in its own column in `full` and FAR's dates and allocated size in `detailed`, plus **the owners view on `Ctrl+8`** (0.6.0) — name, size, owner, group, fetched in a worker only while that view is on screen. Links and custom column sets want data we do not collect yet — `docs/BACKLOG.md` |
| 1.5 | ~~User-definable column sets (which fields, widths, alignment, formats)~~ | FAR/TC | **shipped**: `[[column_set]]` blocks, one per view-mode digit (SPEC 02 §3a) |
| 1.6 | ~~Tabs per panel, named, persisted, lockable, duplicate/move-to-other-side~~ | TC/MTN2 | **shipped** (reordering within the row still owed) |
| 1.7 | ~~Quick view panel (preview of the file under the cursor in the other panel)~~ | FAR/MC | **shipped** (`Ctrl+Q`), and on a folder it shows what the device under it has to say, as FAR does |
| 1.8 | ~~Info panel (drive info, disk usage, selected size, archive/VFS metadata)~~ | FAR/MC | **shipped** (`Ctrl+L`): what the device holds, what is left, its allocation unit and what that costs the listing, the machine's memory, what the directory holds, what is selected, and the row under the cursor. The filesystem's *name* is still only in the drives dialog |
| 1.9 | ~~Tree panel (directory tree in a panel, synced with the other side)~~ | FAR/MC | **shipped** (`Ctrl+T`) |
| 1.10 | More than two panels / floating panel windows | DN/MTN2 | M4 |
| 1.11 | Status line (name, size, date of the current item) and total/selected counters | FAR | M1 |
| 1.11b | Column titles row (`Name │ Size │ Date Time`), with the sort-mode letter in the corner — lower case ascending, upper case descending — and a `*` beside it while a filter is on | FAR | M1 |
| 1.12 | Function-key bar (`F1..F12`, follows modifiers: Shift/Alt/Ctrl rows) | NC | M1 |
| 1.13 | ~~Sort by name/ext/size/mtime/ctime/atime/owner/unsorted, reversible, group dirs first, selected first, natural ("version") sort~~, locale-aware collation | FAR/TC | **shipped** (all nine keys by 0.10.0); locale-aware collation — ordinal / invariant / linguistic as FAR has it — is M3 and a backlog line |
| 1.14 | Secondary sort criteria chain (tie-breakers, configurable): per sort mode an ordered set, with its own editor (add, remove, replace, direction, inherit, reorder, reset) | FAR | M3 |
| 1.14b | ~~Sort groups: named mask groups that sort ahead of or behind everything else, ordered, applied on top of the sort mode~~ | FAR (`Shift+F11`) | **shipped** (0.7.2): `[[sort_group]]` blocks, in the order they are written |
| 1.14c | ~~Sort by creation time, access time, owner and description, besides name/ext/size/write time~~ | FAR (`Ctrl+F8…F11`) | **shipped**: the two times in 0.0.6, the owner and the description in 0.10.0, each fetched by a worker and re-sorted when the answer lands |
| 1.15 | Filters: per-panel display filter, quick filter bar, filter by mask/size/date/attrs, named filter sets, filter menu | FAR/KRU | M2 |
| 1.15b | A column grammar for view modes: per column a property letter with modifiers (name with selection marks / no extension / right-aligned, extension, size / allocated size / stream size with grouping, decimal units, fractions, four date-and-time pairs with brief and month-name forms, attributes, description, owner with domain, hard-link count, stream count), widths absolute or automatic or a percentage, repeated groups giving newspaper stripes, and the same grammar for the panel status line and for search results | FAR | M2 — **shipped in both spellings** (SPEC 02 §3a and §3b): our own TOML columns, and FAR's `Column types`/`Column widths` strings read as they are. Still owed: the per-mode flags and the `+1`/`+2` precision suffixes |
| 1.15c | Per view mode: fullscreen, align file and folder extensions, folders uppercase, files lowercase, uppercase names shown lowercase | FAR | M2 |
| 1.16 | Hidden/system files toggle (`Ctrl+H`). A dotfile counts as hidden on every platform, and Windows adds the hidden and system attributes, so the key means the same thing everywhere | all | M1 |
| 1.17 | Branch view (flat recursive listing of the current tree) | TC | M2 |
| 1.18 | Panelize: fill a panel with the result of a search or an external command | MC | M2 |
| 1.19 | Directory sizes computed on demand (`Space`/`F3` on a dir) and in background | FAR/TC | M1 |
| 1.20 | Long-path support (`\\?\` on Windows, >260 chars), UNC paths | FAR/MTN2 | M1 |
| 1.21 | Per-panel encoding/codepage for names and previews | MC/FAR | M3 |
| 1.22 | Icons / nerd-font glyphs per file type, optional | YAZI | M2 |
| 1.23 | ~~Colour-by-file-type rules (mask, attrs, size, age) — "highlight groups", with the dialog that edits them~~ | FAR/TC | **shipped**: `[[highlight]]` groups in `fcmd.toml`, the first match deciding, with FAR's built-in six after them, and **`F9 → Options → Files highlighting`** (0.7.0) — the groups as a list in the order they are tried, each row in its own colours, `Ins`/`Del`/`Ctrl+↑↓` and a dialog per group |
| 1.24 | ~~Panel history (back/forward), directory history, folder shortcuts/bookmarks (0..9 + named), hotlist~~ | FAR/TC/MC | **shipped** |
| 1.25 | ~~Auto-refresh via filesystem watch (inotify/FSEvents/ReadDirectoryChangesW), manual `Ctrl+R`~~ | TC/MTN2 | **shipped** |
| 1.26 | ~~Names and full paths to the system clipboard~~ | FAR (`Ctrl+Ins`, `Alt+Shift+Ins`) | **Shipped: 0.6.0** — through the terminal with `OSC 52` (SPEC 06 §11), so it also works over `ssh`; `ui.clipboard` turns it off. The *files themselves* on the clipboard, for another application to paste, is window-system work and stays in the backlog |

## 2. Navigation & selection

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 2.1 | Enter = run/chdir/enter archive; parent dir via `..`; root jump | all | M1 |
| 2.2 | Fast find / quick search inside the panel (type to jump, wildcards, cycle matches) | FAR/MC | M1 |
| 2.3 | Fuzzy "go to anywhere" (zoxide-style frecency + fuzzy path jump) | broot/NEW | M3 |
| 2.4 | ~~Drive/mount menu (`Alt+F1/F2`)~~, MountMan-style mount/unmount, removable eject | FAR/KRU | **the menu is shipped** (0.9.0, FAR's own dialog with its columns, its switches on `F9`, `Ctrl+1…9` for the panels that are not drives); mounting, unmounting and ejecting are M3 |
| 2.5 | Select/deselect: `Ins`, `Shift+arrows`, by mask (`+`/`-`/`*`), by extension, by name, same-name/different-size, invert, select all, restore previous selection | FAR/TC | M1 |
| 2.6 | Selection by regex, by size/date/attribute predicates, from clipboard list | FAR/KRU | M3 |
| 2.7 | Mark-and-act across directory changes (persistent selection set / "staging area") | NEW | M4 |
| 2.8 | Sync panel paths (`=`), open current dir in the other panel, swap panels | all | M1 |
| 2.9 | Command line under the panels with history, completion, insert-current-name macros | FAR/MC | M1 |
| 2.10 | Mouse: click, drag-select, wheel scroll, drag&drop between panels and with the OS | FAR/TC | **click, double click, right-click select, wheel, key bar and menus: done** (SPEC 02 §12, `ui.mouse`, off by default). Drag-select, dragging a column rule and OS drag-and-drop: M3 |

## 3. File operations

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 3.1 | Copy / move / rename (`F5`/`F6`), same-panel copy (`Shift+F5`), rename in place (`Shift+F6`) | all | M1 |
| 3.2 | ~~Delete to trash and permanent delete; wipe (overwrite then delete)~~ | FAR | **shipped** |
| 3.3 | Create directory (`F7`), create file (`Shift+F4`), create nested path `a/b/c` in one go | all | M1 |
| 3.4 | ~~Hard links, symlinks (absolute/relative), junctions/reparse points, link targets shown and followed~~ | FAR/MC | **shipped** (`Alt+F6`): hard link, symbolic link and a junction on Windows, with the target shown in the panel and followed on `Enter` |
| 3.5 | ~~Attributes/permissions dialog: chmod (numeric + rwx matrix), timestamps, recursive apply~~; chown, the Windows compressed and encrypted attributes | FAR/MC | **shipped** (`Ctrl+A`): the mode bit by bit with setuid, setgid and sticky, the read-only, hidden and system flags where the platform has them, the write and read times, and recursion into a folder. Changing the owner, and the NTFS compressed and encrypted bits, are M3 |
| 3.6 | Extended attributes / ADS (NTFS streams), xattr on Linux/macOS, resource forks | FAR | M3 |
| 3.7 | Operation engine: queue of jobs, parallel or serial, pause/resume/cancel, background with progress, speed/ETA, per-job log | TC/KRU/MTN2 | M1 basic, M2 queue |
| 3.8 | Conflict resolution: overwrite / skip / rename / append / newer-only / size-differs, "apply to all", per-conflict dialog | all | M1 |
| 3.9 | Verify after copy (hash compare), preserve mode/owner/times/xattrs, sparse files, reflink/`copy_file_range`/CoW, `FSCTL` server-side copy | TC/NEW | M2 |
| 3.10 | Resume interrupted copy of large files | TC | M3 |
| 3.11 | Multi-rename tool: patterns (`[N]`, `[E]`, counters, date, regex, case changes), live preview, undo | TC/KRU | M2 |
| 3.12 | Split / combine files; ~~checksum create & verify (`.md5`, `.sha1`, `.sha256`)~~ | TC/KRU | **checksums shipped** (MD5, SHA-1, SHA-256, in the format those tools read); split, combine and `.sfv` still to come |
| 3.13 | Apply command to each selected file (`Ctrl+G` style), with `%`-macros | FAR | M2 |
| 3.14 | ~~Compare directories (by name/size/date/content), synchronise directories with preview and one-way/two-way modes~~ | TC/KRU | **shipped**; comparing subfolders in the *compare* command, and a saved sync job, still to come |
| 3.15 | ~~Compare files by content (internal diff viewer, side-by-side, word-level)~~ | MC/KRU | **shipped**: `Ctrl+Shift+D` on the file under each cursor, patience diff, word-level marks inside a changed line, hunk-to-hunk with `Alt+↑↓`, blanks, case and line endings each a keystroke; a unified view, copying a hunk across and a byte comparison of two binaries still to come |
| 3.16 | Change encoding / line endings in bulk (CRLF↔LF, codepage recode) | MC | M3 |
| 3.17 | ~~File descriptions/comments (`descript.ion`, `files.bbs`)~~ | FAR | **shipped** (0.7.3): read, shown in two views, `Ctrl+Z` writes one, and the jobs carry them |
| 3.18 | Undo for destructive operations where the FS allows (trash-based) | NEW | M3 |
| 3.19 | Elevated operations (UAC / `sudo` / `pkexec` helper) for privileged paths | FAR/KRU | M3 |

## 4. VFS: archives, remote, special

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 4.1 | Archives browsable as directories, read: zip, tar(+gz/bz2/xz/zst), 7z, rar(read-only), cab, iso, ar/deb/rpm, cpio, lha, arj, zip64, encrypted zip/7z | FAR/TC/MC | **zip, the tar family and 7z: done**; an encrypted 7z is recognised and says it wants a password. M3 for the rest |
| 4.2 | Archive write: create/add/delete/update in zip, 7z, tar family; pack dialog with format options (level, solid, encryption, split volumes) | TC | **create, add, delete and update in all three: done** (a change rebuilds the container transactionally). Solid, encryption and split volumes in the pack dialog: M3 |
| 4.3 | Pack/unpack shortcuts (`Alt+F5`/`Alt+F6`/`Shift+F1`/`Shift+F2`), test archive, extract-here vs extract-to | FAR/TC | M2 |
| 4.4 | Open a file inside an archive in the viewer/editor; edit-in-place writes back | FAR | M3 |
| 4.4b | **Remote file in the viewer and the editor**: `F3` views a fetched copy, `F4` edits it and `F2` writes it back (rename into place, change check, the local copy named if it fails) | FAR NetBox | M1 |
| 4.5 | **SFTP/SSH** panels: listing, navigation, mkdir, rename, delete, symlinks, mode and mtime | far2l NetRocks/termscp | M1 |
| 4.5b | FTP/FTPS, WebDAV, SMB, NFS, MTP/ADB (Android), rclone bridge | far2l NetRocks/termscp | **FTP, FTPS and WebDAV done** — FTP with `MLSD` where the server has it, explicit TLS, `MFMT`/`SITE CHMOD` and a connection of its own per transfer; WebDAV hand-written over `reqwest`, with the streamed `PUT` and the live-property truth about times. SMB, NFS, MTP/ADB and rclone: M3 |
| 4.5c | **S3-compatible object storage, read**: prefix as directory, any endpoint, credentials from the environment or `~/.aws/credentials`, a cap on a flat listing of a huge bucket | far2l NetRocks | **implemented** |
| 4.6 | **Connection manager** (`Alt+N`): saved `[[site]]` list with groups, agent / key file / asked password, host-key verification with a trust-on-first-use prompt, one pooled connection per host, explicit disconnect | TC/KRU | M1 (OS keyring, `ProxyJump`, charset M3) |
| 4.7 | **Transfers through the local operation engine**: copy and move up, down, between two servers and inside one, with progress, conflicts and cancellation; a `sftp://` URL in `Ctrl+L` connects | TC | M1 (background queue, resume and bandwidth limit M3) |
| 4.8 | Temporary panel (collect unrelated files into a virtual panel and act on them) | FAR `tmppanel` | M2 |
| 4.9 | Special VFS: process list, services, registry (Windows), `/proc`-style views, package contents, git status/worktree view | FAR/far2l | **process list, socket list, installed programs and services implemented** (the services panel starts, restarts and stops, and never escalates); registry, package contents and the git view M4 |
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
| 5.7 | ~~Syntax highlighting in the viewer; Markdown rendering; tabs expansion~~ | MTN2 | **shipped** |
| 5.8 | Previews: images (sixel/kitty/iterm protocols), video thumbnails, PDF, archives, media metadata, office/JSON/CSV pretty print — pluggable pipeline with timeouts | YAZI | M3 |
| 5.9 | Filtered view: pipe the file through a command and view the output | MC | M3 |
| 5.18 | ~~**Table mode**: one viewer mode for the tabular formats — a header row, aligned columns, horizontal scroll — with CSV/TSV, JSONL/NDJSON and DBF behind it~~ | NEW (MTN2 has none either) | **Shipped: 0.6.0, DBF in 0.7.0** — `Shift+F3`, and a tabular name opens that way; the heading stays put while the rows scroll, the status line admits the guessed delimiter and gives the shape. A dBase table is read a window at a time, so its size does not matter. Memo files (`.dbt`/`.fpt`), and sorting or filtering a table: M3 |
| 5.19 | **DBF**: dBase III/IV and FoxPro tables in table mode — the typed header, fixed-length records read at an offset so a 2 GB table opens at once, the deleted flag, and the memo files beside it (`.dbt`, `.fpt`) | NEW | M3 |
| 5.10 | Chrome toggles (key bar, status line, scrollbar) and the viewer's own settings | FAR | **Shipped: 0.4.0** — `Ctrl+B`, `Ctrl+Shift+B` and `Ctrl+S` on FAR's own keys, each row taken away giving a row of content back, and `[viewer]` plus `F9 → Options → Viewer settings` behind them (the chrome, the wrap a file opens with, the tab width). Remembering a file's position and code page: M3, with the state store |
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
| view and edit files of unlimited size in text, hex and disassembler modes | **Viewing: shipped** — the viewer streams and seeks, so a 40 GB file opens at once in text, dump, hex or disassembly. **Editing: shipped in 0.0.8** as the hex mode of the editor, deliberately in `fcmd-edit`, because the viewer's read-only contract is exactly what lets it seek through 40 GB |
| x86-64 disassembler and assembler, AVX included | **Disassembler: shipped** (`iced-x86`, which decodes AVX through AVX-512), and **AArch64 too** (`yaxpeax-arm`, 0.18.0) — on an ARM machine a viewer that only read x86 was half a tool. **Assembler: shipped (0.0.8)** for the instructions a patcher types — registers, numbers, addresses and branches written as the address to arrive at, one at a time into the patch list (SPEC 05 §2.8). It is x86 only, and says so on an ARM binary rather than writing x86 bytes into it; the vector sets are refused by name rather than guessed at |
| physical and logical drive view and edit | **Done on Linux and macOS (0.0.8), read-only.** A block device is a file; its length is measured with a seek rather than read from `stat`, it opens read-only, and the bytes are written *out* to a file rather than back over the disk. Windows needs its own path and is on the backlog |
| NE, LE, LX, PE/PE32+, ELF/ELF64, Mach-O, TE/TE64 | **All shipped** in the structure pane. PE/PE32+, ELF/ELF64, Mach-O (thin and universal) and `ar` with imports, exports and the .NET CLI header; NE with its segment table, LE and LX with their objects, and TE — the stripped PE of UEFI firmware, which also answers for TE64, since its header always carries a 64-bit image base and `Machine` says which machine it is |
| Netware Loadable Modules (NLM, DSK, LAN…) | **No.** A format with no living platform behind it; if anybody asks, it belongs in a plugin, not the core |
| navigate direct call/jmp with a single click | **Shipped** — `Enter` on a `call` or `jmp` follows it, `Backspace` comes back, and the jumped-from stack is on screen |
| search by instruction pattern in the disassembler | **Done (0.0.8).** `F7` over a disassembly searches the decoded instructions: a mnemonic, an operand shape, `*` for anything in between. SPEC 05 §2.7 |
| built-in 64-bit decrypt/crypt system | **XOR shipped (0.0.8)** as a block operation, named for what it is and not for security. A keyed cipher stays on the backlog, under the same condition |
| built-in 64-bit calculator | **Done (0.0.8).** One line of arithmetic answered in decimal, hex, octal, binary and as characters at once, with the bitwise and shift operators; literals in every base the tools around it use (`0x1f`, `1Fh`, `0b1010`, `'PE'`) |
| file operations on blocks: read, write, fill, copy, move, insert, delete, crypt | **Done (0.0.8), except crypt.** `F3` marks, `F12` operates: write out, read in, fill, exclusive-or, copy and move from elsewhere in the file, insert zeros, delete — each showing the resulting length before it happens. A named cipher stays on the backlog, as a block operation that says what it is not |
| search and replace across multiple files | **Backlog, new line.** The find tool (SPEC 06 §1) gets the files; replacing in them is a job with its own confirmation, a dry run, and a count per file |
| keyboard macros | **Done (0.1.0).** `Ctrl+.` records, the key one presses afterwards plays it, `Ctrl+Shift+.` lists them. A macro is a list of commands in a readable TOML file, with a mode that runs it once for every selected file; anything destructive asks first unless the file trusts it (SPEC 08 §5) |
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
| 6.2 | Block operations: stream blocks and **column/vertical blocks**, copy/cut/paste (a column paste pads short lines), indent/outdent | FAR | M1; block-to-cursor and append-to-clipboard shipped; the system clipboard shipped in 0.6.0 — a copied or cut block goes on it as well as on the editor's own |
| 6.2b | **Hex mode** (`F4` on a binary, `Ins` for insert/overwrite): byte editing with a patch list, the editor's own undo, and the same temp-file-and-rename save; in-place writing only when explicitly chosen | NEW | M2 |
| 6.2c | **Byte blocks** (`F3` marks, `F12` operates): write out, read in, fill, XOR with a key, copy or move from elsewhere, insert, delete — large ones through the job engine, each showing the resulting length first | NEW | M2 |
| 6.2d | **Inline assembler** (`F9`): one instruction, showing the bytes it produces, the instruction it replaces and whether the length changed, written into the patch list rather than to disk | NEW | M3 |
| 6.3 | Undo/redo grouped per command | FAR | M1 (typing-burst grouping, configurable depth and persisted positions M2) |
| 6.4 | Search/replace: plain search with wrap, replace once, replace all (the engine already does regex and case-insensitive; the dialog that exposes them is M2) | FAR | M1; whole word, capture refs, "preserve style", all-matches list M2/M3 |
| 6.5 | Go to `line` or `line:column`, bookmarks 0..9 | FAR | M1 (matching-bracket jump M2) |
| 6.6 | Keep the original encoding, EOL, BOM and final newline on save | FAR | M1 (explicit conversion commands M2) |
| 6.7 | Syntax highlighting (tree-sitter), bracket matching, auto-indent, tabs/spaces policy, trailing-space trim | MTN2/NEW | M3 |
| 6.8 | Insert file name / path / current date, character by code, macros | FAR | M3 |
| 6.9 | External editor delegation (`$EDITOR`, per-type associations) | all | **0.4.0**: `Alt+F4` takes what an association names, else `$VISUAL` or `$EDITOR`; `Alt+F3` the same for `$PAGER` |
| 6.10 | LSP-lite: completion/diagnostics for the open file (optional plugin) | NEW | M4 |
| 6.11 | **Atomic save** (`name.fcmd-tmp` + rename, permissions carried over) and a save-as prompt | NEW | M1 |
| 6.12 | **External-change detection**: a file changed on disk is never overwritten without a confirmation; reload on demand | FAR | M1 (diff-on-conflict M3) |
| 6.13 | `Shift+F4` edits a new file, saved into the active panel's directory | FAR | M1 |
| 6.14 | Line editing: delete, duplicate, move up/down, delete to end of line, delete word left, auto-indent, smart home, tab expansion | FAR | M1 (join, sort, unique, reverse, number M2) |

## 7. Tools & dialogs

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 7.1 | Find files: mask, content (text/regex/hex), size/date/attr filters, archives included, encoding list, results panel, panelize results, "find folder" | FAR/TC | **Shipped: 0.0.4, size condition 0.6.0** — `Alt+F7`: masks in the §5 grammar, text or a hex pattern, case, whole words, files that do *not* contain it, a size condition (`100M`, `1M-1G`, `-4K`), five scopes, symbolic links, folders in the results; results stream while the walk runs and `F5` panelises them. Dates and attributes in the dialog, archives, encodings, saving a search and "find folder": M3 |
| 7.2 | Grep-like content search with result navigation and jump-to-match in the editor | NEW | M2 |
| 7.3 | Disk usage / occupied space view (per-dir sizes, sortable, graphical bars) | KRU/TC | M2 |
| 7.4 | User menu (`F2`) with user-defined entries, submenus, hotkeys, per-directory menus | FAR/MC | **Shipped: 0.4.0** — `fcmd-menu.toml` beside the settings and `.fcmd-menu.toml` in a directory, merged above it; a row is a label, a key and command lines that run in order and stop at the first failure, with the §5b substitutions and `{ask:…}`; `when` hides a row the directory has no use for. Reading `FarMenu.ini` as it stands, separators, an editor for the entries and `Shift+F10`: M3 |
| 7.5 | File associations by mask: view/edit/execute/alternative commands | FAR/MC | **`[[assoc]]` blocks with `open`, `view` and `edit`: 0.4.0** — `Enter`, `Alt+F3` and `Alt+F4`, with the desktop opener and `$PAGER`/`$VISUAL` behind them and an *Open with* list. Defaults read off the OS, a dialog to edit the blocks and the `execute`/`extract`/`preview` entries: M3 |
| 7.6 | Command history, view/edit history, folder history — all searchable and persisted | FAR | M1 |
| 7.7 | Macro recorder/player, keyboard macros with conditions, assignable to any key | FAR/far2m | **Recorder, player and list: 0.1.0** (`Ctrl+.`, `Ctrl+Shift+.`) — a macro is a list of commands in a readable file, assigned to the key one presses after recording, with `once` and `each-selected` play modes. Conditions and the Lua form need a macro step that can ask the application a question, which is the plugin host's job: M3 |
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
| 8.4 | Embedded terminal pane(s), tabs of terminals, alternate-screen apps inside the pane | MTN2/KRU | **shipped** (one at a time; tabs of terminals still owed) |
| 8.5 | Shell integration both ways: `cd` in the shell moves the panel, panel moves the shell | MC/NEW | M2 |
| 8.6 | Run as administrator / another user | FAR | M3 |
| 8.7 | Output search (`Ctrl+Shift+F`) and output capture into a panel | MTN2 | M3 |
| 8.8 | Open OS file manager / terminal / shell menu for the current item | TC/KRU | M2 |

## 9. Configuration, UI, i18n

| # | Feature | Origin | M |
| --- | --- | --- | --- |
| 9.1 | Human-editable config files (TOML) + in-app settings dialogs (System, Panel, Interface, Confirmations — every row bound to a config key and applied through the same code as `--set`, SPEC 10 §3); portable mode (config next to the binary) | MTN2 | M1 |
| 9.2 | Full keymap remapping, multiple **keymap profiles** (`far`, `tc`, `mc`, `nc`, `vi`) switchable at runtime | KRU/MTN2 | M1 (far+mc), M2 rest |
| 9.3 | Themes: 7 built-in (`commander`, `far` — FAR's own default palette element for element, `black`, `white`, `matrix`, `tron`, `blade`) + `[[theme]]` blocks in `fcmd.toml` or from plugins, hot-reloaded, `inherit`-able, exportable with `--export-theme`; 24-bit colour with 256/16/mono degradation | MTN2 | M1 |
| 9.4 | Localisation (fluent-based): EN, IT, DE, FR, ES, RU, ZH-Hans; UI + help + key names, runtime switchable from the `Languages` list, drop-in language dirs | FAR/MTN2 | M2 |
| 9.5 | CJK/wide-character correct layout, combining marks, RTL-safe truncation, emoji width | MTN2 | M2 |
| 9.6 | Accessibility: no-colour mode, screen-reader-friendly output mode, configurable focus indication | NEW | M3 |
| 9.7 | Session save/restore (tabs, paths, selections, viewer positions) | TC | M2 |
| 9.8 | Settings search ("where is that option?") | NEW | M3 |
| 9.9 | No telemetry, no network calls except explicit user actions; optional update check off by default | MTN2 | M1 |
| 9.10 | Menu bar (`F9`) with FAR's five menus; every entry is a command from the vocabulary, the shortcut shown is read from the keymap in force, and an entry whose feature is missing is drawn dim and says so (SPEC 10 §2) | FAR | M1 |
| 9.14 | Screens (`F12`, `Ctrl+Tab`): a viewer or an editor one steps away from and comes back to, keeping its position and its unsaved changes, with the panels as screen zero of the ring and a count of what is behind them in the corner. `F10` from the panels names every background screen with unwritten changes (SPEC 02 §10) | FAR | **0.1.0** |
| 9.15 | Help (`F1`) as a book: written chapters compiled into the binary, and the keys, the commands and the settings generated from the keymap and configuration in force so they cannot drift. A manual page generated from the command line's own options, with a test that fails when the two disagree | FAR / Unix | **0.1.0** |
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
| 9b.5 | Four histories (commands, folders, view/edit, dialogs), each with a size and a lifetime in days, persisted, with items that can be locked so new entries do not evict them | FAR | **All four shipped: commands and folders earlier, the files read or edited in 0.7.0** (`Alt+F11`, `ui.file_history`), all persisted in the state file. A lifetime in days (`ui.history_lifetime`), pinned lines (`Ins`) and filtering as one types are shipped too; and the dialogs' own fields have one as well (`Ctrl+↓`), so all four are here |
| 9b.6 | ~~File descriptions (`descript.ion`, `files.bbs`): edit, display as a column, carry through copy/move/rename/delete, show the folder's own description file in the info panel~~ | FAR/NC | **shipped** (0.7.3): `Ctrl+Z` edits, `Ctrl+6`/`Ctrl+7` show, the information panel and the status line spell one out, and a description follows a copy, a move, a rename and a delete |
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
| 10.1b | Plugin commands (`F11`): a plugin declares a `[[command]]` and the host runs it as a process, with the context as JSON on standard input and one action per line back — each action naming a command of the vocabulary, so a plugin can do exactly what a key can (SPEC 08 §1b) | FAR (plugin menu) | **0.1.0** |
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
