# Feature inventory

The union of FAR Manager, Total Commander, Krusader, Midnight Commander, Double Commander and
the modern TUIs (yazi, MTN2), normalised into one list with a target milestone.

Milestones: **M1** = usable daily driver (MVP), **M2** = feature parity with mc + basic FAR,
**M3** = parity with FAR/TC, **M4** = beyond (new ideas). M1 closed long ago, so no row
carries it any more: what it owned is a `✓`.

**How to read the `State` column**, as of 0.41.0:

| | |
| --- | --- |
| `✓` | the program has it |
| `~` | partly — the cell says what is left |
| `✗` | not there — followed by the milestone the idea stands against (`M2` is the one in progress; `M3` and `M4` are not a promise), or by the decision that keeps it out |
| `—` | a decision, not a feature |

Every `M2` row was read against the code again for 0.26.0, and thirty of the thirty-six were
out of date: twelve because the work had been finished and nobody changed the character, and
eighteen because most of it had and the cell now says what is left. Six still said `M2` and
meant it; the multi-rename tool, the disk-usage view and the branch view have gone since, in
0.31.0, and the three left are a stacked layout, icons, and opening the item in the desktop's
own file manager. Read again for 0.37.0 against the survey of November 2026, whose gaps are in
`docs/BACKLOG.md`, and again for 0.41.0 against the sources of the other commanders themselves:
section 12 says how, what it found and how much of each of them is covered.

Until 0.22.0 this column held a paragraph per row about what had shipped and in which
version, which was a second changelog to keep in step with the first — and a tax on every
feature, since the rules said to rewrite that paragraph before calling the work finished. It
does not any more. **What landed, and why it was built that way, is in
[`CHANGELOG.md`](../CHANGELOG.md)**; what is missing *and wanted* is in
[`docs/BACKLOG.md`](BACKLOG.md), one line each with the reason; and this table is what it was
written to be — the map of everything the commanders of the world do, and where each idea came
from. Finishing a feature here is now one character.

The numbers are addresses and never change: a row that arrives later takes a letter
(`1.15b`, `5.21`) rather than pushing the others along.

For FAR specifically, [`04-far-inventory.md`](04-far-inventory.md) is the harder check: FAR 3.x
read from its own repository — 283 help topics, 311 settings keys — with each feature marked
against what FinalCommander actually does today.
Origin: `FAR`, `TC`, `KRU` (Krusader), `MC`, `DC` (Double Commander), `MTN2`, `YAZI`, `NEW`;
since 0.41.0 also `MU` (muCommander), `NNN`, `RANGER`, `LF`, `BROOT`, `VIFM`, `XPLR`, `SPF`
(superfile), `OPUS` (Directory Opus), `NINJA` (TheFile.Ninja), `C1` (Commander One), and
`far2l`, ForkLift and Marta by name.

## 1. Panels

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 1.1 | Two equal panels, active/passive, `Tab` to switch | NC | ✓ |
| 1.2 | Panel layouts: side-by-side and stacked (horizontal split) | KRU | ✗ M2 |
| 1.3 | Resizable split, saved per session; restore defaults | FAR | ✓ `Ctrl+←/→/↑/↓`, the mouse on the rule, and `Ctrl+Alt+R` for both back to the defaults |
| 1.4 | View modes: brief (multi-column names), medium, full (size/date), wide, detailed, owners, links, custom columns | FAR | ✓ |
| 1.5 | User-definable column sets (which fields, widths, alignment, formats) | FAR/TC | ✓ |
| 1.6 | Tabs per panel, named, persisted, lockable, duplicate/move-to-other-side | TC/MTN2 | ✓ |
| 1.7 | Quick view panel (preview of the file under the cursor in the other panel) | FAR/MC | ✓ |
| 1.8 | Info panel (drive info, disk usage, selected size, archive/VFS metadata) | FAR/MC | ✓ |
| 1.9 | Tree panel (directory tree in a panel, synced with the other side) | FAR/MC | ✓ |
| 1.10 | More than two panels / floating panel windows | DN/MTN2 | ✗ M4 |
| 1.11 | Status line (name, size, date of the current item) and total/selected counters | FAR | ✓ |
| 1.11b | Column titles row (`Name │ Size │ Date Time`), with the sort-mode letter in the corner — lower case ascending, upper case descending — and a `*` beside it while a filter is on | FAR | ✓ |
| 1.12 | Function-key bar (`F1..F12`, follows modifiers: Shift/Alt/Ctrl rows) | NC | ✓ |
| 1.13 | Sort by name/ext/size/mtime/ctime/atime/owner/unsorted, reversible, group dirs first, selected first, natural ("version") sort, locale-aware collation | FAR/TC | ~ the Latin accents are folded (`é` with `e`, `ß` as `ss`); a locale's own alphabet is decided against (`docs/BACKLOG.md`, Settled) |
| 1.14 | Secondary sort criteria chain (tie-breakers, configurable): per sort mode an ordered set, with its own editor (add, remove, replace, direction, inherit, reorder, reset) | FAR | ~ `panel.sort_chain`: up to three tie-breakers, the name last, `reverse` over the whole; one chain for every sort mode, and no editor — it is written in the file |
| 1.14b | Sort groups: named mask groups that sort ahead of or behind everything else, ordered, applied on top of the sort mode | FAR (`Shift+F11`) | ✓ |
| 1.14c | Sort by creation time, access time, owner and description, besides name/ext/size/write time | FAR (`Ctrl+F8…F11`) | ✓ |
| 1.15 | Filters: per-panel display filter, quick filter bar, filter by mask/size/date/attrs, named filter sets, filter menu | FAR/KRU | ~ locking a filter so a changed directory keeps it; the bar takes size, age, attributes and kind, and named sets are `[[filter]]` blocks on `Ctrl+Alt+S` with their own dialog |
| 1.15b | A column grammar for view modes: per column a property letter with modifiers (name with selection marks / no extension / right-aligned, extension, size / allocated size / stream size with grouping, decimal units, fractions, four date-and-time pairs with brief and month-name forms, attributes, description, owner with domain, hard-link count, stream count), widths absolute or automatic or a percentage, repeated groups giving newspaper stripes, and the same grammar for the panel status line and for search results | FAR | ✓ |
| 1.15c | Per view mode: fullscreen, align file and folder extensions, folders uppercase, files lowercase, uppercase names shown lowercase | FAR | ✓ |
| 1.16 | Hidden/system files toggle (`Ctrl+H`). A dotfile counts as hidden on every platform, and Windows adds the hidden and system attributes, so the key means the same thing everywhere | all | ✓ |
| 1.17 | Branch view (flat recursive listing of the current tree) | TC | ✓ `Ctrl+Alt+B` |
| 1.18 | Panelize: fill a panel with the result of a search or an external command | MC | ✓ |
| 1.19 | Directory sizes computed on demand (`Space`/`F3` on a dir) and in background | FAR/TC | ✓ |
| 1.20 | Long-path support (`\\?\` on Windows, >260 chars), UNC paths | FAR/MTN2 | ✓ |
| 1.21 | Per-panel encoding/codepage for names and previews | MC/FAR | ~ the names, yes; a preview pane has its own page already |
| 1.22 | Icons / nerd-font glyphs per file type, optional | YAZI/LF/NNN/SPF/BROOT | ✗ M2 |
| 1.23 | Colour-by-file-type rules (mask, attrs, size, age) — "highlight groups", with the dialog that edits them | FAR/TC | ✓ |
| 1.24 | Panel history (back/forward), directory history, folder shortcuts/bookmarks (0..9 + named), hotlist | FAR/TC/MC | ✓ |
| 1.25 | Auto-refresh via filesystem watch (inotify/FSEvents/ReadDirectoryChangesW), manual `Ctrl+R` | TC/MTN2 | ✓ |
| 1.26 | Names and full paths to the system clipboard | FAR (`Ctrl+Ins`, `Alt+Shift+Ins`) | ✓ |
| 1.27 | Columns whose values a plugin computes — Total Commander's content plugins (`.wdx`): a photo's EXIF date, a track's tags, a document's page count, sortable and searchable like any other column | TC | ~ a `[[column]]` of a plugin, named in a `[[column_set]]`; not a sort key, not searched, and only for folders on this machine |
| 1.28 | Each folder keeps the view and the sort it was last given, and gets them back when the panel returns | FAR/NNN/VIFM | ✓ `panel.remember_view`, off by default |
| 1.29 | Where a file stands in its git repository, as a column and as a sort key | YAZI/BROOT/NINJA/RANGER | ✓ `type = "git"` in a `[[column_set]]`, *Sort by git status* |

## 2. Navigation & selection

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 2.1 | Enter = run/chdir/enter archive; parent dir via `..`; root jump | all | ✓ |
| 2.2 | Fast find / quick search inside the panel (type to jump, wildcards, cycle matches) | FAR/MC | ✓ |
| 2.3 | Fuzzy "go to anywhere" (zoxide-style frecency + fuzzy path jump) | broot/NEW | ✓ `Alt+Shift+F12` (SPEC 02 §11b) |
| 2.4 | Drive/mount menu (`Alt+F1/F2`), MountMan-style mount/unmount, removable eject | FAR/KRU | ~ mounting, unmounting and ejecting |
| 2.5 | Select/deselect: `Ins`, `Shift+arrows`, by mask (`+`/`-`/`*`), by extension, by name, same-name/different-size, invert, select all, restore previous selection | FAR/TC | ✓ |
| 2.6 | Selection by regex, by size/date/attribute predicates, from clipboard list | FAR/KRU | ~ from a list on the clipboard, which a terminal will not hand back |
| 2.7 | Mark-and-act across directory changes (persistent selection set / "staging area") | NNN/RANGER/LF/YAZI/BROOT/XPLR/DC | ~ the temporary panel (4.8) collects files from anywhere and acts on them as one selection; the marks themselves are lost when the panel changes directory |
| 2.8 | Sync panel paths (`=`), open current dir in the other panel, swap panels | all | ✓ |
| 2.9 | Command line under the panels with history, completion, insert-current-name macros | FAR/MC | ✓ |
| 2.10 | Mouse: click, drag-select, wheel scroll, drag&drop between panels and with the OS | FAR/TC | ~ click, drag-select, the wheel, and a drop from one panel on the other, which opens the copy dialog; not with the desktop, which is decided against (`docs/BACKLOG.md`, Settled) |

## 3. File operations

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 3.1 | Copy / move / rename (`F5`/`F6`), same-panel copy (`Shift+F5`), rename in place (`Shift+F6`) | all | ✓ |
| 3.2 | Delete to trash and permanent delete; wipe (overwrite then delete) | FAR | ✓ |
| 3.3 | Create directory (`F7`), create file (`Shift+F4`), create nested path `a/b/c` in one go | all | ✓ |
| 3.4 | Hard links, symlinks (absolute/relative), junctions/reparse points, link targets shown and followed | FAR/MC | ✓ |
| 3.5 | Attributes/permissions dialog: chmod (numeric + rwx matrix), timestamps, recursive apply; chown, the Windows compressed and encrypted attributes | FAR/MC | ~ the shell's own property sheet, the NTFS compressed and encrypted bits, and the owner on Windows |
| 3.6 | Extended attributes / ADS (NTFS streams), xattr on Linux/macOS, resource forks | FAR | ~ a copy carries them on Linux and macOS (`operations.preserve`), the resource fork among them; nothing lists or edits them, and NTFS streams are decided against |
| 3.7 | Operation engine: queue of jobs, parallel or serial, pause/resume/cancel, background with progress, speed/ETA, per-job log | TC/KRU/MTN2 | ~ a per-job log kept on disk |
| 3.8 | Conflict resolution: overwrite / skip / rename / append / newer-only / size-differs, "apply to all", per-conflict dialog | all | ✓ |
| 3.9 | Verify after copy (hash compare), preserve mode/owner/times/xattrs, sparse files, reflink/`copy_file_range`/CoW, `FSCTL` server-side copy | TC/NEW | ~ the owner, sparse files; the read-back compare is local only |
| 3.10 | Resume interrupted copy of large files | TC | ~ to and from a server or a bucket, which copy through their own providers |
| 3.11 | Multi-rename tool: patterns (`[N]`, `[E]`, counters, date, regex, case changes), live preview, undo | TC/KRU | ~ one step of undo, kept across a restart; the plan cannot be exported, and transliteration, `[X]` (the clipboard) and plugin fields (`[=exif:…]`) are not there |
| 3.12 | Split / combine files; checksum create & verify (`.md5`, `.sha1`, `.sha256`, `.sfv`) | TC/KRU | ✓ split and combine in Total Commander's `.001` and `.crc`, checksums in all four formats |
| 3.13 | Apply command to each selected file (`Ctrl+G` style), with `%`-macros | FAR | ✓ |
| 3.14 | Compare directories (by name/size/date/content), synchronise directories with preview and one-way/two-way modes | TC/KRU | ✓ |
| 3.15 | Compare files by content (internal diff viewer, side-by-side or unified, word-level; two binaries byte against byte; saved as a patch) | MC/KRU | ✓ |
| 3.16 | Change encoding / line endings in bulk (CRLF↔LF, codepage recode) | MC | ✓ |
| 3.17 | File descriptions/comments (`descript.ion`, `files.bbs`) | FAR | ✓ |
| 3.18 | Undo for destructive operations where the FS allows (trash-based) | NEW | ✓ `Ctrl+Alt+Z` undoes the last copy (to the wastebasket), move, delete to the wastebasket and new folder, the last twenty remembered (SPEC 03 §14) |
| 3.19 | Elevated operations (UAC / `sudo` / `pkexec` helper) for privileged paths | FAR/KRU | ✗ out of scope until someone needs it badly enough to design it (`docs/BACKLOG.md`, Settled) |
| 3.20 | Taking the metadata out of pictures | NEW | ✓ *Files → Strip picture metadata* (`file.strip_metadata`) |

## 4. VFS: archives, remote, special

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 4.1 | Archives browsable as directories, read: zip, tar(+gz/bz2/xz/zst), 7z, rar(read-only), cab, iso, ar/deb/rpm, cpio, lha, arj, zip64, encrypted zip/7z | FAR/TC/MC | ✓ read in process: zip and the packages that are zips (`.jar .war .ear .aar .apk .aab .ipa .whl .nupkg .vsix .xpi`), the tar family, 7z with its passwords, `ar` and the `.deb` family (whose inner tars open as archives in an archive), `cpio` and `rpm` (since 0.30.0), `cab` where it is stored or MSZIP, and a bare `.gz`, `.bz2`, `.xz` or `.zst` as an archive of one entry. `rar`, `iso`, `lha`, `arj` and a `cab` packed with LZX or Quantum go through a program already on the machine, from the table in `fcmd-arc::ext` (`docs/05-multiarc.md`) — one `7z` also brings `msi`, `wim`, `chm`, `dmg` and the rest it reads — so on a machine without it they do not open, and the panel says which program to install, in the reader's own language. A tool driven from a table only ever reads — never creates, adds or deletes (§9) |
| 4.1b | **A format the user adds**, without waiting for us: a table naming a program on the machine, its command lines and how to read its listing | MC (`extfs` scripts) | ✓ `[[archiver]]` blocks in `fcmd.toml` (SPEC 07 §2b-quinquies), in MultiArc's own language so a `custom.ini` table keeps its keys. Asked before the shipped `7z` table, and one named `7z` replaces it. Read only, like every table: a block with a line that would change an archive is refused by name. A wrong block is a startup warning naming it, never installed half-working |
| 4.2 | Archive write: create/add/delete/update in zip, 7z, tar family; pack dialog with format options (level, solid, encryption, split volumes) | TC | ~ split volumes. A bare `.gz`/`.bz2`/`.xz`/`.zst` is written from one file since 0.30.0, and inside one the entry is rewritten on save. The dialog's list is the formats *this program* writes and never a property of the machine: an external tool only ever reads (`docs/05-multiarc.md` §9) |
| 4.3 | Pack/unpack shortcuts (`Alt+F5`/`Alt+F6`/`Shift+F1`/`Shift+F2`), test archive, extract-here vs extract-to | FAR/TC | ~ `Alt+F5`/`Alt+F6`, which are Total Commander's keys and taken here |
| 4.4 | Open a file inside an archive in the viewer/editor; edit-in-place writes back | FAR | ✓ |
| 4.4b | **Remote file in the viewer and the editor**: `F3` views a fetched copy, `F4` edits it and `F2` writes it back (rename into place, change check, the local copy named if it fails) | FAR NetBox | ✓ |
| 4.5 | **SFTP/SSH** panels: listing, navigation, mkdir, rename, delete, symlinks, mode and mtime | far2l NetRocks/termscp | ✓ |
| 4.5b | FTP/FTPS, WebDAV, SMB, NFS, MTP/ADB (Android), rclone bridge | far2l NetRocks/termscp | ~ FTP, FTPS and WebDAV are in, less FTP resume and active mode and WebDAV digest and locking; SMB, NFS, MTP/ADB and an rclone bridge are not there at all |
| 4.5d | **`fish://`**: a filesystem over a plain ssh shell, for a host where sftp is turned off — mc ships a set of shell snippets it feeds to the remote shell | MC | ✓ another way, since 0.35.0: no scheme of its own, but an `sftp://` session whose server refuses the SFTP subsystem falls back to the shell by itself (SPEC 04 §6.5b) — POSIX `ls`, `cat`, `mkdir`, `mv`, `rm`, `chmod`, `touch -t`, with `ls` read under `LC_ALL=C` |
| 4.5c | **S3-compatible object storage, read**: prefix as directory, any endpoint, credentials from the environment or `~/.aws/credentials`, a cap on a flat listing of a huge bucket | far2l NetRocks | ✓ |
| 4.6 | **Connection manager** (`Alt+N`): saved `[[site]]` list with groups, agent / key file / asked password, host-key verification with a trust-on-first-use prompt, one pooled connection per host, one jump host per site (`ssh -J`), explicit disconnect | TC/KRU | ~ the OS keyring, and a code page of its own |
| 4.7 | **Transfers through the local operation engine**: copy and move up, down, between two servers and inside one, with progress, conflicts and cancellation; a `sftp://` URL in `Ctrl+L` connects | TC | ~ resuming an interrupted transfer; a speed limit per server (`max_kbps`) since 0.34.0 |
| 4.8 | Temporary panel (collect unrelated files into a virtual panel and act on them) | FAR `tmppanel` | ✓ |
| 4.9 | Special VFS: process list, services, registry (Windows), `/proc`-style views, package contents, git status/worktree view | FAR/far2l | ~ the registry and package contents; the system log is *Commands → System log* (SPEC 06 §16); git is `Ctrl+Alt+G`, all of it panels: what changed with stage, unstage, diff and commit, the branches and switching, the log of the repository or of a file, pull and push (SPEC 06 §15) |
| 4.10 | Trash as a browsable VFS | KRU/MU/BROOT/VIFM/YAZI/TC | ✗ M3 |
| 4.11 | VFS API available to plugins (so a plugin can add a scheme) | FAR/TC(WFX) | ✗ M3 |
| 4.12 | SQLite databases: tables and views as CSV entries, `schema.sql`, and `sql:` queries opened in table mode, read only | FAR (SQLiteDB) | ✓ (SPEC 04 §4b) |
| 4.13 | JSON documents as folders: objects and arrays as directories, values as files (strings as their text), read only | FarNet (JsonKit) | ✓ (SPEC 04 §4c) |
| 4.14 | Mail as folders: an mbox as its messages, a message as its headers, text and decoded attachments; read only | FAR (FARMail/MailView) | ✓ files on disk (SPEC 04 §4d), and `imap://`/`imaps://` mailboxes read only since 0.35.0 (SPEC 04 §6.6) |
| 4.15 | Tunnels: a local port forwarded through the SSH connection the panel is on, and a list of the open ones | WinSCP/NetBox | ✓ `net.tunnel`, `net.tunnels` |

## 5. Viewer (`F3`)

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 5.1 | Text mode with and without wrap (char/word wrap), horizontal scroll, arbitrary line length | FAR | ✓ |
| 5.2 | Hex mode and dump mode, configurable bytes per row | FAR | ✓ |
| 5.3 | Encoding detection + manual codepage selection, BOM handling, invalid-byte display | FAR/MC | ✓ |
| 5.4 | Search (plain, case-insensitive, whole word, regex, hex pattern), search next/previous, highlight all hits, copy selection | FAR | ✓ |
| 5.5 | Go to offset/line/percent; bookmarks 0..9; "undo position" | FAR | ✓ |
| 5.6 | Streaming/mmap for huge files (GB+) without loading; follow mode (`tail -f`) | FAR/MTN2 | ✓ |
| 5.7 | Syntax highlighting in the viewer; Markdown rendering; tabs expansion | MTN2 | ✓ |
| 5.8 | Previews: images (sixel/kitty/iterm protocols), video thumbnails, PDF, archives, media metadata, office/JSON/CSV pretty print — pluggable pipeline with timeouts | YAZI | ~ pictures (5.22), video frames (5.24), archives as folders, tables (5.18) and the metadata of media, PDF and office files in `F11` are in; the `[[preview]]` rules with a timeout (SPEC 05 §1.7) are not built, a PDF's text or page comes only through a `[[decompile]]` rule, and fonts and SVG are not drawn |
| 5.9 | Filtered view: pipe the file through a command and view the output | MC | ✓ |
| 5.18 | **Table mode**: one viewer mode for the tabular formats — a header row, aligned columns, horizontal scroll — with CSV/TSV, JSONL/NDJSON and DBF behind it | NEW (MTN2 has none either) | ✓ `Ctrl+F3` sorts by the column the view is on, `Ctrl+F7` keeps the rows that hold a text; a row cursor, `Ins` marks rows and `Ctrl+C` copies them as tab-separated text; the heading is guessed and `Ctrl+H` turns the guess round |
| 5.21 | **Base64 decoded in the viewer** (`Alt+B`): the bare payload, PEM armour or a `data:` URI, shown as text or in the hex pane depending on what came out | NEW | ✓ |
| 5.22 | **Pictures in the viewer**: `F3` on a PNG, JPEG, GIF or BMP draws it in real pixels where the terminal has them (kitty, iTerm2, sixel) and in half blocks in any other; `F4` goes on to the bytes | TC/KRU (lister plugins) | ✓ |
| 5.24 | **Video frames through `ffmpeg`**: `F3` and `Ctrl+Q` on a video show a frame, `PgUp`/`PgDn` step by a tenth of it, `Space` plays without sound; `ffmpeg` is run as a program and named when it is missing | YAZI (video thumbnails) | ✓ |
| 5.19 | **DBF**: dBase III/IV and FoxPro tables in table mode — the typed header, fixed-length records read at an offset so a 2 GB table opens at once, the deleted flag, and the memo files beside it (`.dbt`, `.fpt`) | NEW | ✓ |
| 5.10 | Chrome toggles (key bar, status line, scrollbar) and the viewer's own settings | FAR | ✓ |
| 5.11 | **Disassembly** (`F5`): ELF/PE/Mach-O/raw recognition, sections and symbols, x86/x86-64 decoding with symbol headings, instruction-accurate scrolling, raw sweep for files over the parse limit | NEW | ✓ |
| 5.12 | **Decompilation** (`Shift+F6`): `[[decompile]]` rules call installed tools (javap, ilspycmd, python -m dis, wasm2wat, objdump, retdec) and the output is viewed as text; a rule is used only when its program is on the `PATH`, and a format with no tool says so | NEW | ✓ |
| 5.13 | **JVM bytecode and .NET IL in process**: class files listed javap-style with the constant pool resolved; managed PEs listed ildasm-style with metadata-resolved tokens and `#US` string literals | NEW | ✓ |
| 5.14 | Still external: Python bytecode, aarch64/riscv decoding, .NET signatures and generics | NEW | ~ AArch64 is decoded in process; Python bytecode, RISC-V and .NET signatures and generics are not |
| 5.15 | **Structure pane** (`F11`): what the file is and where anything inside it is — format, architecture, kind, entry point, the header field by field, sections and segments, imports and exports, the interpreter and build id — and since 0.38.0 what the program says about itself: a PE's version resource, manifest, mitigations, PDB and signature, ELF notes, a Mach-O's UUID, platform and code signature, and the tags of photos and music (EXIF, PNG text, ID3, FLAC, Ogg); `Tab` moves the focus, `Enter` takes the hex or disassembly pane to that offset. Read through a seeking reader, so a section table at the end of a 3 GB file costs two seeks | NEW | ✓ |
| 5.16 | Following a branch in the disassembly (`Enter` on a `call`/`jmp`, `Backspace` back, the jumped-from stack on screen), and `Alt+F8` taking a virtual address as well as an offset | NEW | ~ the jumped-from stack on the screen, and a virtual address in the goto |
| 5.17 | Raw devices: a block device or a partition opened as a file, size from an `ioctl`, read-only unless told otherwise | NEW | ✓ |
| 5.23 | Next/previous file in panel without leaving the viewer (`+`/`-`) | FAR | ✓ |

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
| search and replace across multiple files | **Shipped** — `Ctrl+Shift+R` (`tool.replace`, feature 7.24): a job of its own over the selected files, text, pattern or hex bytes, whose confirmation is the dry run |
| keyboard macros | **Done (0.1.0).** `Ctrl+.` records, the key one presses afterwards plays it, `Ctrl+Shift+.` lists them. A macro is a list of commands in a readable TOML file, with a mode that runs it once for every selected file; anything destructive asks first unless the file trusts it (SPEC 08 §5) |
| unicode/utf8 support | **Shipped** throughout: the panel, the viewer, the editor and the command line are UTF-8, with the codepage list for files that are not |
| Hiew External Module (HEM) support | **Our answer is the plugin system** (SPEC 08): manifests on disk today, a host API when there is something worth hosting |
| ArmV6 disassembler | **Backlog** — AArch64 first, since a viewer that only decodes x86 is half a tool on an Apple Silicon machine; a decoder crate rather than new design |

Two things on this list we have and Hiew does not, so they are worth saying out loud: the
structure pane decodes imports, exports and the .NET header beside the raw bytes, and the
decompilers (JVM and .NET IL in process, external tools by rule) turn a binary into
something readable rather than only into instructions.

## 6. Editor (`F4`)

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 6.1 | Full-screen editor, 256 MiB open limit, read-only files opened and marked | FAR | ~ more than one file open at once |
| 6.2 | Block operations: stream blocks and **column/vertical blocks**, copy/cut/paste (a column paste pads short lines), indent/outdent | FAR | ✓ |
| 6.2b | **Hex mode** (`F4` on a binary, `Ins` for insert/overwrite): byte editing with a patch list, the editor's own undo, search and replace as text or hex bytes with `??` (`F7`, `Ctrl+F7`), and the same temp-file-and-rename save; in-place writing only when explicitly chosen | NEW | ✓ |
| 6.2c | **Byte blocks** (`F3` marks, `F12` operates): write out, read in, fill, XOR with a key, copy or move from elsewhere, insert, delete — large ones through the job engine, each showing the resulting length first | NEW | ✓ |
| 6.2d | **Inline assembler** (`F9`): one instruction, showing the bytes it produces, the instruction it replaces and whether the length changed, written into the patch list rather than to disk | NEW | ✓ |
| 6.3 | Undo/redo grouped per command and per burst of typing, `Backspace` or `Del`; the depth a setting (`editor.undo_depth`) | FAR | ✓ |
| 6.4 | Search/replace: text or regular expression, case, whole words, wrap; replace next and replace all with capture references (`$1`, `${name}`) and "preserve style"; a list of every matching line | FAR | ~ replace inside a block only; the separator half of FAR's preserve style (`foo_bar` ↔ `fooBar`) |
| 6.5 | Go to `line` or `line:column`, bookmarks 0..9 | FAR | ✓ and `Alt+B` to the matching bracket |
| 6.6 | Keep the original encoding, EOL, BOM and final newline on save; `Shift+F8` converts the end of line, the BOM and the code page, each one undo step | FAR | ✓ |
| 6.7 | Syntax highlighting (tree-sitter), bracket matching, auto-indent, tabs/spaces policy, trailing-space trim | MTN2/NEW | ~ colouring for 25 languages and markups by a scanner of our own rather than tree-sitter, auto-indent, the three tab policies of `editor.tabs`, bracket matching (`Alt+B`), trailing blanks taken off (`Alt+Shift+T`, or at every save with `editor.trim_trailing`) |
| 6.8 | Insert file name / path / current date, character by code, macros | FAR | ~ a character from a list filtered by its code (`Alt+Shift+C`); the name, path and date are not there yet |
| 6.9 | External editor delegation (`$EDITOR`, per-type associations) | all | ✓ |
| 6.10 | LSP-lite: completion/diagnostics for the open file (optional plugin) | NEW | ✗ M4 |
| 6.11 | **Atomic save** (`name.fcmd-tmp` + rename, permissions carried over) and a save-as prompt | NEW | ✓ |
| 6.12 | **External-change detection**: a file changed on disk is never overwritten without a confirmation, which offers to overwrite, to reload or to see what changed in the side-by-side diff first; reload on demand | FAR | ✓ |
| 6.13 | `Shift+F4` edits a new file, saved into the active panel's directory | FAR | ✓ |
| 6.14 | Line editing: delete, duplicate, move up/down, delete to end of line, delete word left, auto-indent, smart home, tab expansion | FAR | ✓ and on a block: join (`Alt+J`), sort, unique, reverse and number (`Alt+Shift+S/U/R/N`) |
| 6.15 | `.editorconfig`: indentation, width and trailing-blank trimming per project, line ending and final newline for new files | NEW (every modern editor) | ✓ (SPEC 05 §2.2c) |
| 6.16 | Word completion from the words of the file, nearest first, cycling (`Alt+/`) | FAR (EditorCompletion)/emacs | ✓ |
| 6.17 | A marked block rewritten by a language model from an instruction in words: the model's version shown as a diff against the block, applied on `Enter` as one undo step | NEW | ✓ `Ctrl+Alt+W` (SPEC 05 §2.10) |

## 7. Tools & dialogs

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 7.1 | Find files: mask, content (text/regex/hex), size/date/attr filters, archives included, encoding list, results panel, panelize results, "find folder" | FAR/TC | ~ an absolute date range and attributes in the dialog (an age in days is there), code pages |
| 7.2 | Grep-like content search with result navigation and jump-to-match in the editor | NEW | ✓ `F3` at the match, `F4` on its line |
| 7.3 | Disk usage / occupied space view (per-dir sizes, sortable, graphical bars) | KRU/TC | ✓ |
| 7.4 | User menu (`F2`) with user-defined entries, submenus, hotkeys, per-directory menus | FAR/MC | ~ an editor for the entries; separators, `Shift+F10` and FAR's `FarMenu.ini` read as it stands are in |
| 7.5 | File associations by mask: view/edit/execute/alternative commands | FAR/MC | ✓ |
| 7.6 | Command history, view/edit history, folder history — all searchable and persisted | FAR | ✓ |
| 7.7 | Macro recorder/player, keyboard macros with conditions, assignable to any key | FAR/far2m | ~ conditions on a macro, which are decided against (`docs/BACKLOG.md`, Settled) |
| 7.8 | Scripting console (Lua) with the full command vocabulary | far2m/YAZI | ✗ decided against (`docs/BACKLOG.md`, Settled) |
| 7.9 | Print files / print manager | FAR | ✗ M4 |
| 7.10 | Screen grabber (copy a rectangle of the screen to clipboard) | FAR | ✓ `Alt+Ins` |
| 7.11 | Learn-keys / terminal key diagnostics | MC | ~ the escape sequence cannot be typed in: it is read by a library with a table for the terminals in use |
| 7.12 | Help system: context-sensitive `F1`, searchable, localised, offline | FAR/KRU | ✓ `F7` searches every chapter, written and generated |
| 7.13 | Saved searches: a find-file query kept under a name and run again from a list, mask, text and conditions and all | TC/DC/KRU | ✓ `F2`/`F9` in the find dialog, and `Alt+Shift+F7` to open one as a panel (SPEC 06 §1c) |
| 7.14 | Duplicate files: by size, then the first 64 KiB, then SHA-256; the groups as a panel with every copy but the oldest selected | FAR (FindDup)/TC | ✓ *Commands → Find duplicate files* (SPEC 06 §13) |
| 7.15 | Asking a language model from the command line (`ai:`): OpenAI, Anthropic or Ollama, the token from an environment variable, files only after a dialog that names the host, answers kept as Markdown files | NEW | ✓ (SPEC 06 §14) |
| 7.16 | Find files by approximate name: the letters in order anywhere in the name, results ranked best first | broot/NEW | ✓ a box in `Alt+F7` (SPEC 06 §1d) |
| 7.17 | Find files by metadata: EXIF, ID3, document properties and the rest of what `F11` reads, as `field=value` conditions | TC/Wikipedia | ✓ a row in `Alt+F7` (SPEC 06 §1d) |
| 7.18 | Indexed search: the system's own index (plocate/locate, Spotlight, Everything) asked for candidates instead of a walk, each checked again on the disk | Opus/NEW | ✓ a box in `Alt+F7`, `find.index` (SPEC 06 §1e) |
| 7.19 | Redaction before a question goes to a language model: keys, tokens, private keys, passwords in addresses and `.env`-style secrets masked, with a count and a preview of what is sent; the user's own patterns | NEW | ✓ on by default, `[ai_guard]` (SPEC 06 §14.2) |
| 7.20 | A local-only switch for the language models: only a service on this machine (`localhost`, `127.0.0.0/8`, `::1`, a unix socket, the user's own list) is asked, anything else refused before it is sent | NEW | ✓ `F6` in the AI panel, `[ai_guard] local_only` (SPEC 06 §14.2) |
| 7.21 | Words to a rename: an instruction in words on the selected files, answered by a language model with a pattern that lands in the multi-rename dialog, previewed, editable, renamed only on `OK` | NEW | ✓ `Ctrl+Alt+U` (SPEC 03 §8a) |
| 7.22 | Explain what failed: after a command that ended with a non-zero code, one key sends the command, the code and the end of its output to a language model and opens the answer | NEW | ✓ `Ctrl+Alt+X` in the panels (SPEC 06 §14) |
| 7.23 | Words to a search: what is wanted in words, answered by a language model with the rows of the find dialog — mask, size, age, text — which are read, changed and searched only on `OK`; only the words are sent | NEW | ✓ `Ctrl+Alt+Q` (SPEC 06 §1f) |
| 7.24 | Replace across the selected files, as a job whose confirmation is its own dry run: text, pattern or hex bytes | Hiew | ✓ `Ctrl+Shift+R` (`tool.replace`) |
| 7.25 | Command palette: every command in one list that narrows as it is typed into | TC (command browser)/DC/Marta/ForkLift | ✗ `app.palette` is in the vocabulary and on `Ctrl+Shift+P`, and answers that it is not implemented; `F9`, `F1` and *All settings* (9.8) are what there is |
| 7.26 | A secrets file of the program's own: keys and servers' passwords encrypted under one password, forgotten after five idle minutes | TC (master password) | ✓ `fcmd-secrets.vault` |

## 8. Shell & terminal integration

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 8.1 | Command line with the panel as context; `%`-macros for current/other panel, names, paths, selections | FAR/MC | ✓ |
| 8.2 | Run a command and keep the output visible (`Ctrl+O` user screen) | FAR/MC | ✓ |
| 8.3 | Real subshell / PTY: persistent shell session sharing the cwd with the panel | MC/MTN2 | ✓ |
| 8.4 | Embedded terminal pane(s), tabs of terminals, alternate-screen apps inside the pane | MTN2/KRU | ~ tabs of terminals |
| 8.5 | Shell integration both ways: `cd` in the shell moves the panel, panel moves the shell | MC/NEW | ✓ |
| 8.6 | Run as administrator / another user | FAR | ✗ out of scope with 3.19 |
| 8.7 | Output search (`Ctrl+Shift+F`) and output capture into a panel | MTN2 | ✓ `Ctrl+Shift+F` searches the output; capture into a panel is *Commands → Panelize a command's output* |
| 8.8 | Open OS file manager / terminal / shell menu for the current item | TC/KRU | ~ a file with no association goes to the desktop's opener (`xdg-open`, `open`); showing the item in the desktop's file manager and opening the desktop's terminal there are not |
| 8.10 | **The shell is left where the panel was**: quitting leaves the shell that started it in the directory the active panel ended in — the gesture `mc` is used with. A process cannot change its parent's working directory, so it takes two halves: `-P FILE` (`--printwd`, mc's own spelling) writes it, and a shell function reads it and goes there. `fcmd --shell-init bash|zsh|fish|powershell` prints the function, so nothing is installed anywhere (SPEC 11 §1) and it cannot be out of step with the binary | MC (`mc-wrapper.sh`) | ✓ and better on one point: quitting inside an archive leaves the shell in the folder holding it, where mc writes the archive's own path and its wrapper refuses it without saying why |
| 8.9 | **Called from outside instead of opened**: the viewer, the editor and the comparison reachable from the command line, so the program can be `$VISUAL`, a `git difftool` or a pager in a script | MC (`mcview`, `mcedit`, `mcdiff`) | ✓ `--view FILE`, `--edit FILE`, `--diff A B`; leaving that screen ends the run, and a name that is not there opens an empty editor under it |

## 9. Configuration, UI, i18n

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 9.1 | Human-editable config files (TOML) + in-app settings dialogs (System, Panel, Interface, Confirmations — every row bound to a config key and applied through the same code as `--set`, SPEC 10 §3); portable mode (config next to the binary) | MTN2 | ✓ |
| 9.2 | Full keymap remapping, multiple **keymap profiles** (`far`, `tc`, `mc`, `nc`, `vi`) switchable at runtime | KRU/MTN2 | ~ the `tc`, `nc` and `vi` profiles |
| 9.3 | Themes: 10 built-in (`commander`, `far` — FAR's own default palette element for element, `black`, `white`, `matrix`, `tron`, `blade`, `github-dark`, `delphi-dark-2027`, `delphi-inspiration`), bold/italic/underline per key in `[styles]`, + `[[theme]]` blocks in `fcmd.toml` or from plugins, hot-reloaded, `inherit`-able, exportable with `--export-theme`; 24-bit colour with 256/16/mono degradation | MTN2 | ✓ |
| 9.4 | Localisation (fluent-based): EN, IT, DE, FR, ES, RU, ZH-Hans; UI + help + key names, runtime switchable from the `Languages` list, drop-in language dirs | FAR/MTN2 | ✓ |
| 9.5 | CJK/wide-character correct layout, combining marks, RTL-safe truncation, emoji width | MTN2 | ~ combining marks and RTL-safe truncation |
| 9.6 | Accessibility: no-colour mode, screen-reader-friendly output mode, configurable focus indication | NEW | ~ `NO_COLOR`, `CLICOLOR=0` and `TERM=dumb` give a screen without colour; a mode for screen readers and a focus mark that can be set are not there |
| 9.7 | Session save/restore (tabs, paths, selections, viewer positions) | TC | ~ the selection, which is not kept |
| 9.8 | Settings search ("where is that option?") | NEW | ✓ *Options → All settings*, a list of every setting that narrows as you type |
| 9.9 | No telemetry, no network calls except explicit user actions; optional update check off by default | MTN2 | ✓ *Options → Check for updates*, and `general.update_check` once a day (SPEC 11 §5) |
| 9.10 | Menu bar (`F9`) with FAR's five menus; every entry is a command from the vocabulary, the shortcut shown is read from the keymap in force, and an entry whose feature is missing is drawn dim and says so (SPEC 10 §2) | FAR | ✓ |
| 9.14 | Screens (`F12`, `Ctrl+Tab`): a viewer or an editor one steps away from and comes back to, keeping its position and its unsaved changes, with the panels as screen zero of the ring and a count of what is behind them in the corner. `F10` from the panels names every background screen with unwritten changes (SPEC 02 §10) | FAR | ✓ |
| 9.15 | Help (`F1`) as a book: written chapters compiled into the binary, and the keys, the commands and the settings generated from the keymap and configuration in force so they cannot drift. A manual page generated from the command line's own options, with a test that fails when the two disagree | FAR / Unix | ✓ |
| 9.11 | `Save setup` (`Shift+F9`) writes the configuration as it stands, including themes that came from the file | FAR | ✓ |
| 9.13 | Session editor for stored connections (`Alt+N` → `Ins`/`F4`): name, host, port, user, authentication, key file, remote folder, group, keep-alive, compression — validated before it is stored, `Shift+F9` makes it permanent (SPEC 10 §3.5) | FAR (NetBox) | ✓ |
| 9.12 | Choose-one lists: languages, themes, drives and open connections (`Alt+F1`/`Alt+F2`), folder history (`Alt+F12`), sort modes (`Ctrl+F12`, where re-choosing the current order reverses it) (SPEC 10 §4) | FAR | ✓ |

## 9b. What a FAR user expects and we had not written down

Found by reading FAR's own help and configuration tables; the detail is in
[`04-far-inventory.md`](04-far-inventory.md) and each line has a backlog entry.

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 9b.1 | The selection matrix: the modifier picks the criterion — mask, same extension as the cursor, same name, everything — across select, deselect and invert, with separate rules for folders | FAR | ✓ |
| 9b.2 | `Shift+F5`/`Shift+F6`/`Shift+F8`: copy, move and delete the item under the cursor, ignoring the selection | FAR | ✓ |
| 9b.3 | Filters as one shared object used by the panel display, by find and by copy: mask, size range (binary and decimal suffixes), date range on any of four timestamps, absolute or relative in days, attributes as three-state include/exclude/ignore, more-than-one-hard-link | FAR/KRU | ~ the copy takes only the masks, not the size, date and attribute conditions |
| 9b.4 | Mask groups (`<arc>`, `<temp>`, `<exec>`, nestable) and the full mask grammar: character ranges, `/regex/i`, `include | exclude` | ✓ |
| 9b.5 | Four histories (commands, folders, view/edit, dialogs), each with a size and a lifetime in days, persisted, with items that can be locked so new entries do not evict them | FAR | ✓ |
| 9b.6 | File descriptions (`descript.ion`, `files.bbs`): edit, display as a column, carry through copy/move/rename/delete, show the folder's own description file in the info panel | FAR/NC | ✓ |
| 9b.7 | Copy dialog depth: several destinations, append to existing, only-newer as a policy, copy symlink contents, a filter on the operation, destination from the folder tree, speed and ETA | FAR | ✓ |
| 9b.8 | Metasymbol language for the user menu, file associations and apply-command: name forms, the selected-file list as a file or inline, path forms, resolved paths, the description, a prompt with history, panel-switching prefixes, and quoting/case/encoding modifiers | FAR | ~ resolved paths, the description, and the case and encoding modifiers |
| 9b.9 | Command prefixes on the command line (`view:`, `edit:`, `goto:`, `clip:`, `whereis:`, `link:`, `run:`), as FAR's `farcmds` does | FAR | ~ `clip:` and `link:` |
| 9b.10 | Internal shell commands that need no shell: `cls`, `cd`/`chdir` with a configurable `cd ~`, `chcp`, `set`, nestable `if [not] exist` / `if [not] defined`, `pushd`/`popd`/`clrd`, `title`, `exit` | FAR | ~ `title`, `chcp`, and the nestable `if` |
| 9b.11 | Autocomplete with four separately switchable sources (filesystem, history, `PATH`, environment), each three-state, as a list or appended inline | FAR | ✓ |
| 9b.12 | Screen grabber: select any region of the screen, stream or block, extend, move, append to the clipboard | FAR (`Alt+Ins`) | ~ a rectangle, marked from the cursor, copied whole; no stream mode, no appending to what is on the clipboard |
| 9b.13 | Viewer ⇄ editor at the same position, jump to the open file in the panel, next/previous file of the panel from inside the viewer | FAR (`F6`, `Ctrl+F10`, `Gray ±`) | ✓ |
| 9b.14 | Remember per file: position, code page, view mode, wrap mode, bookmarks | FAR | ✓ |
| 9b.15 | Sorting collation as three settings — collation (ordinal/invariant/linguistic), digits-as-numbers, case sensitivity — rather than one "natural sort" flag | FAR | ~ digits (`panel.natural_sort`) and case (`panel.sort_case`) are a setting each; the collation is one and not a choice: the Latin accents folded |
| 9b.16 | A searchable settings browser, because the setting count only grows (FAR: 311) | FAR (`far:config`) | ✓ the same feature as 9.8 |
| 9b.17 | Dialog framework: history in edit fields with lockable items, autocomplete, three-state checkboxes, movable dialogs, click outside to accept or cancel | FAR | ~ moving a dialog with the mouse |
| 9b.18 | Menus that filter as you type, with a lock and horizontal scrolling | FAR (`RAlt`) | ~ every list filters as you type, `Esc` clears before it closes, `Alt+←/→` slides the rows; FAR's lock on the filter is not there |

## 10. Extensibility

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 10.1 | Command vocabulary (`cmd.panel.copy`, …) that every key, menu entry, macro and plugin targets | DC (`cm_*`) | ✓ |
| 10.1b | Plugin commands (`F11`): a plugin declares a `[[command]]` and the host runs it as a process, with the context as JSON on standard input and one action per line back — each action naming a command of the vocabulary, so a plugin can do exactly what a key can (SPEC 08 §1b) | FAR (plugin menu) | ✓ |
| 10.2 | Lua scripting API (panels, selection, jobs, dialogs, VFS, events) | far2m/YAZI | ✗ decided against (`docs/BACKLOG.md`, Settled); 10.1b is the answer |
| 10.3 | WASM plugins (WASI + a component interface): VFS providers, panels, viewers, commands, highlighters — sandboxed, portable | MTN2 | ✗ M3 |
| 10.4 | Native plugin ABI (stable C ABI) for performance-critical/OS-specific plugins | FAR | ✗ M4 |
| 10.5 | Plugin manager: discovery, install from a registry, versions, permissions | YAZI/NNN/KRU/far2l | ✗ M4 |
| 10.6 | Event hooks (on-copy, on-enter-dir, on-select) for automation | LF/YAZI/XPLR/VIFM/RANGER | ✗ M3 |
| 10.7 | Headless/CLI mode: run commander operations from scripts (`fcmd copy --queue …`) | NEW/NINJA | ✗ M4 |

## 11. Platform & quality

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 11.1 | Linux, macOS, Windows first-class; FreeBSD best-effort | — | ✓ |
| 11.2 | TUI backend (any terminal, works over SSH/tmux) as the primary UI | — | ✓ |
| 11.3 | **No GUI backend, ever** — strictly TUI; terminal extras (inline images, kitty keyboard, OSC 52) are capability-detected, not a second UI | decision | — |
| 11.4 | Single static binary, no installer required; packages: `.deb`, `.rpm`, Homebrew, Scoop/winget, AUR, Flatpak | — | ~ the packages: the release publishes the four archives and their sums |
| 11.5 | Startup under ~50 ms, listing of 100k entries without blocking the UI | YAZI | ✓ |
| 11.6 | Crash safety: no data loss on operation interruption; journaled job state | NEW | ~ the jobs waiting in the queue are not journalled, only the one running |
| 11.7 | Tests: unit + golden-screen snapshot tests + VFS integration + fuzzed archive/path parsing | — | ✓ |

## 12. Read from the others' sources, for 0.41.0

The question was "do we do everything the other commanders do?", and the sections above could
not answer it: they were written from documentation and from use. For 0.41.0 the sources
themselves were read — fetched on 11 November 2026 into a folder outside this repository,
read as data and deleted afterwards — and each program's own list of what it does was set
against this table and against the code in `crates/`.

What was read, per program:

| Program | What its list of features was taken from |
| --- | --- |
| FAR Manager 3 | `FarGroup/FarManager`: `plugins/`, and the reading already in [`04-far-inventory.md`](04-far-inventory.md) |
| far2l | `elfmz/far2l`: `plugins/` (27), NetRocks' `Protocol/` (9), `changelog.md` |
| Midnight Commander | `MidnightCommander/mc`: the menus of `src/filemanager/filemanager.c`, every action of `misc/mc.default.keymap`, `src/vfs/` and the `extfs` helpers |
| Double Commander | `doublecmd/doublecmd`: the `cm_*` procedures of `src/umaincommands.pas` (about 215), `plugins/` |
| Total Commander | closed, and its site was not reachable: the 394 internal commands with their descriptions as Double Commander records them for compatibility (`src/platform/win/uTotalCommander.pas`) |
| Krusader | `KDE/krusader`: the actions of `app/krusaderui.rc`, `doc/handbook/features.docbook` |
| muCommander | `mucommander/mucommander`: the `ActionType` identifiers, the `mucommander-protocol-*`, `-format-*` and `-viewer-*` modules |
| nnn | `jarun/nnn`: the *Features* of its README, `plugins/` |
| ranger | `ranger/ranger`: `ranger/config/commands.py`, `rc.conf` |
| lf | `gokcehan/lf`: the quick reference of `doc.md` |
| yazi | `sxyazi/yazi`: `yazi-config/preset/keymap-default.toml`, the preset plugins, its documentation repository |
| broot | `Canop/broot`: the internal verbs of `src/verb/internal.rs` |
| vifm | `vifm/vifm`: the sections and the `:commands` of `data/man/vifm.1` |
| superfile | `yorukot/superfile`: `hotkeys.toml` |
| xplr | `sayanarijit/xplr`: the *messages* page of its manual |
| Directory Opus 13 | closed: the feature list of its home page |
| TheFile.Ninja | closed: its home page |
| Commander One | closed, and its site was not reachable: the feature list of its Wikipedia article |
| ForkLift, Marta | closed, and neither site was reachable: named in the rows below only where the feature is one they are known for, and left out of the count |

**How much of each is covered.** Each program's list was folded into groups — one per thing a
user would name, so `cm_LeftSortByName` and its nine siblings are one — and each group marked
against the code. What only a window system can be (toolbars, floating windows, the Explorer's
context menu, drag and drop with the desktop) is left out of the count rather than held against
a terminal program. The last column counts a `~` as half. These are groups counted by hand:
read them to within five points.

| Program | Groups | `✓` | `~` | `✗` | Covered |
| --- | ---: | ---: | ---: | ---: | ---: |
| FAR Manager 3 (rows of `04-far-inventory.md`, as of 0.26.0, so a floor) | 83 | 49 | 28 | 6 | 76% |
| far2l (what it adds to FAR: plugins and protocols) | 36 | 22 | 2 | 12 | 64% |
| Midnight Commander | 145 | 112 | 6 | 27 | 79% |
| Double Commander | 95 | 70 | 5 | 20 | 76% |
| Total Commander | 123 | 88 | 5 | 30 | 74% |
| Krusader | 80 | 58 | 5 | 17 | 76% |
| muCommander | 95 | 62 | 5 | 28 | 68% |
| Directory Opus (home page) | 28 | 16 | 2 | 10 | 61% |
| TheFile.Ninja (home page) | 22 | 13 | 1 | 8 | 61% |
| Commander One (Wikipedia) | 18 | 14 | 2 | 2 | 83% |
| nnn | 42 | 30 | 3 | 9 | 75% |
| ranger | 46 | 29 | 5 | 12 | 68% |
| lf | 40 | 23 | 5 | 12 | 64% |
| yazi | 55 | 36 | 4 | 15 | 69% |
| broot | 32 | 21 | 3 | 8 | 70% |
| vifm | 52 | 34 | 5 | 13 | 70% |
| superfile | 26 | 20 | 2 | 4 | 81% |
| xplr | 20 | 9 | 4 | 7 | 55% |
| **All of them** | **1038** | **706** | **92** | **240** | **72%** |

The seven orthodox commanders at the top come to 74% (657 groups, 461 `✓`, 56 `~`). So the
answer is **no, about three quarters** — and the quarter that is missing is the same few things
many times over, which is why the rows below are short. They are the features no row above
had; what a row above already names (a stacked layout 1.2, icons 1.22, the staging area 2.7,
elevation 3.19, SMB and phones 4.5b, the wastebasket 4.10, a filesystem from a plugin 4.11, the
preview rules 5.8, scripting 7.8 and 10.2, the `vi` and `tc` profiles 9.2, hooks 10.6, the
command line without the screen 10.7) is not repeated. The same reading found six things
the program does and this table did not say — rows 1.28, 1.29, 3.20, 4.15, 7.24 and 7.26 —
and one it said nothing about that is declared and not built, 7.25.

[`docs/BACKLOG.md`](BACKLOG.md), *Parity after 0.41*, has every gap with who has it, how much
work it is and whether the house rules allow it.

| # | Feature | Origin | State |
| --- | --- | --- | --- |
| 12.1 | Cut, copy and paste of files inside the program: marked with one key in one folder, pasted with another in the next | TC/DC/KRU/MU/RANGER/LF/YAZI/NNN/VIFM/SPF | ✗ |
| 12.2 | The files themselves on the desktop's clipboard, both ways | TC/DC/KRU/MU/FAR | ✗ |
| 12.3 | Thumbnail view: a panel of pictures instead of names | TC/DC/KRU/OPUS/C1 | ✗ |
| 12.4 | Miller columns: the parent, the folder and a preview of the row side by side | RANGER/LF/YAZI/VIFM/NINJA | ✗ |
| 12.5 | Synchronised browsing: entering a folder in one panel enters the folder of the same name in the other | TC/DC/KRU/ForkLift | ✗ |
| 12.6 | Tabs: a closed tab opened again, the duplicates or all the others closed, a set of tabs kept under a name | TC/DC/KRU/MU | ✗ |
| 12.7 | Sessions under a name: panels, tabs and paths kept and switched between | VIFM/NNN/KRU (panel profiles) | ~ one session, restored at start (9.7) |
| 12.8 | Showing only the selected rows | TC/RANGER | ✗ |
| 12.9 | The selection written to a file and read back; the listing with its columns written to a file or the clipboard, or printed | TC/DC/OPUS | ✗ |
| 12.10 | The number of entries in each folder, as a column | LF/BROOT/RANGER | ✗ |
| 12.11 | Sort by permissions and by group; a random order | MU/RANGER/YAZI | ✗ |
| 12.12 | The cursor to the next and the previous selected row | TC | ✗ |
| 12.13 | A tree with the files in it, folded and unfolded where it stands | BROOT/VIFM | ~ the tree panel (1.9) holds folders, the branch view (1.17) is flat |
| 12.14 | A standing list of names never shown | TC/DC/LF | ~ hidden files and the named filters (1.15), which are put on and taken off |
| 12.15 | Renaming many files by editing their names as lines in the editor | RANGER/VIFM/NNN/YAZI/TC | ✗ the multi-rename tool (3.11) is the other way to the same end |
| 12.16 | Encoding and decoding UUE, XXE and MIME | TC | ~ Base64 is decoded in the viewer (5.21) |
| 12.17 | More digests: SHA-512, SHA-3, BLAKE | KRU/DC/TC/MU | ~ CRC32, MD5, SHA-1 and SHA-256 (3.12) |
| 12.18 | Changing where a symbolic link points | MC/RANGER | ✗ |
| 12.19 | Extended file flags (`chattr`) and access-control lists | MC/TC | ✗ |
| 12.20 | A log of every operation, kept in a file | TC/DC | ✗ with the per-job log of 3.7 |
| 12.21 | Merging in the comparison: a hunk taken from one side to the other, and the result saved | MC | ✗ |
| 12.22 | Several files or folders made at once from a list of names | YAZI/NNN | ✗ |
| 12.23 | The selected files sent by mail | MU/MC | ✗ |
| 12.24 | A file encrypted or signed with `gpg` | NNN | ✗ |
| 12.25 | Google Drive, Dropbox, OneDrive | MU/C1/ForkLift/DC | ✗ |
| 12.26 | A torrent file as a folder of what it describes | MC/DC | ✗ |
| 12.27 | A patch as a folder of its hunks | MC | ✗ |
| 12.28 | The installed packages as a folder (`apt`, `dpkg`, `rpm`) | MC | ✗ |
| 12.29 | Servers found on the local network (Bonjour, the network neighbourhood) | MU/FAR/TC | ✗ |
| 12.30 | Containers as panels | far2l (a Python plugin) | ✗ |
| 12.31 | Virtual-machine disks (`qcow2`, `vmdk`, `vhd`) as folders | MU | ~ what the `7z` on the machine reads |
| 12.32 | Fonts and SVG drawn in the preview | YAZI | ✗ |
| 12.33 | Manual pages laid out (`nroff`) | MC | ~ through the filtered view (5.9) |
| 12.34 | A ruler in the viewer, a right margin in the editor | MC | ✗ |
| 12.35 | Sound played | TC/DC/NNN | ✗ decided with the video: sound stays out |
| 12.36 | HTML and RTF laid out to be read | TC/DC | ✗ Markdown is (5.7) |
| 12.37 | Line numbers beside the text in the editor | MC | ✗ |
| 12.38 | Spell check | MC | ✗ |
| 12.39 | A paragraph reflowed, wrapping while typing, a block aligned | MC/FAR/far2l | ✗ |
| 12.40 | The case of a block changed | FAR/far2l | ✗ |
| 12.41 | Lines and boxes drawn with the arrows | FAR/far2l | ✗ |
| 12.42 | A block sent through a command and replaced by what it prints; a file or a command's output inserted; a block written to a file | MC/far2l | ✗ in the text editor; the hex editor's byte blocks do (6.2c) |
| 12.43 | The syntax chosen by hand | MC/far2l | ✗ it follows the file's name |
| 12.44 | Search while typing in the editor; the next occurrence of the word under the cursor | far2l | ✗ |
| 12.45 | Git's changes marked in the editor's margin | far2l | ✗ the git panel and blame are in (4.9) |
| 12.46 | A scratchpad kept between runs | far2l | ✗ |
| 12.47 | Chooser mode: started by another program, it prints the files that were chosen and ends | NNN/RANGER/LF/YAZI/VIFM/BROOT/XPLR | ~ `-P` writes the folder it ended in (8.10); nothing writes the files chosen |
| 12.48 | A running instance told what to do from outside | VIFM/LF/YAZI/XPLR/NNN | ✗ |
| 12.49 | A list of paths on standard input opened as a panel | NNN/VIFM | ~ a command's output is (1.18); standard input is not read |
| 12.50 | Suspending to the shell with `Ctrl+Z` | MC/YAZI/RANGER/VIFM/LF | ✗ `Ctrl+O` and the terminal pane answer the need |
| 12.51 | A dialog that binds keys | TC/DC/KRU/MU/C1 | ✗ the keymap is a file, read back by `--check-keymap` |
| 12.52 | A notice when a long job ends: the bell, or the desktop's notification | NNN | ✗ |
| 12.53 | The terminal locked after an idle time | NNN | ✗ |
| 12.54 | "Open with": the programs the desktop has registered for this kind of file | RANGER/YAZI/MU/far2l/NNN | ~ the `[[assoc]]` blocks (7.5); the desktop's own table is not read |
| 12.55 | The listing grouped under headings | OPUS | ~ the sort groups (1.14b) order it, without headings |
| 12.56 | Tags, labels and ratings on files | OPUS/NINJA/RANGER/LF/ForkLift | ✗ parked by the author |
| 12.57 | Metadata written, pictures converted and resized | OPUS/NNN | ✗ read (5.15) and stripped (3.20) only |
| 12.58 | A sidebar of pinned folders and drives | SPF/OPUS/ForkLift | ✗ the hotlist and `Alt+F1` hold the same things |
| 12.59 | A scheduler, versioned backup, a transfer to another machine by a code | NINJA | ✗ |
