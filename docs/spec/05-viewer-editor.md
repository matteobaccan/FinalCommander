# SPEC 05 — Viewer and editor

**Implemented today** (M1): the viewer's text, dump and hex modes; no-wrap / character / word
wrapping; encoding detection (BOM, UTF-8 validity, the UTF-16 NUL pattern) with manual
switching between UTF-8, UTF-16LE/BE, CP1252 and CP437; plain, regex and hex-pattern search
with highlighted hits and next/previous; go to offset, `0x` offset or percentage; position undo;
bookmarks 0–9; follow mode; windowed reads with no line index, so file size does not matter.
Also implemented: **disassembly** of ELF/PE/Mach-O/raw x86 and x86-64 with sections and
symbols, **JVM class files and .NET assemblies** read in process (constant pool, members and
JVM bytecode; metadata tables and CIL), and **decompilation** through WebAssembly's text format
in process or an installed external tool (§1.6).
Also implemented: the **editor** (`F4`, `Shift+F4` for a new file) — see the note under §2.
Also implemented: **syntax highlighting** in both (§2.5), for the languages and markups
`scmd-syntax` knows, off with `ui.syntax = false`.
**Still to come**: the "all matches" list (§1.4), the preview pipeline (§1.7), filtered view
(§1.10), next/previous file in the panel, Python bytecode and non-x86
decoding in process, and the diff viewer (§3).

## 1. Viewer (`F3`)

### 1.1 Modes

| Mode | Purpose | Notes |
| --- | --- | --- |
| `text` | decoded text, wrap off/char/word | arbitrary line length, tabs expanded per config |
| `dump` | raw bytes as characters, fixed width | for binaries where text decoding lies |
| `hex` | offset + hex + ASCII/decoded pane | bytes-per-row adjustable (`Alt+←/→`), 16-aligned jumps |
| `preview` | rendered content (image, markdown, PDF page, media info, archive listing) | pluggable, see §1.7 |
| `structure` | a side pane that decodes what the file *is*: header fields, section and segment tables, imports and exports | `F11`, see §1.8; it rides alongside `hex` and `disasm` rather than replacing them |

Mode indicator in the status bar (`t`/`d`/`h`/`p`) plus encoding, position (offset, %, line) and
file size. `F2` toggles wrap in text mode, `F4` toggles hex, `Shift+F4` picks the base mode
(FAR's base/hex distinction is kept — it is genuinely useful).

### 1.2 Large files

- No full load, ever. The backing store is a windowed reader: `mmap` when the provider allows
  it and the file is stable, otherwise a chunk cache (default 8 MiB window, 4 windows).
- Line index is built lazily and incrementally in the background; the scrollbar shows an
  estimate until the index completes. `End` jumps by seeking, not by scanning.
- Non-seekable sources (stdin, a stream inside a solid archive) are spooled to a temp file with
  progress, or viewed in "forward only" mode with a note.
- **Follow mode** (`Shift+F`, like `tail -f`): watch for appends and auto-scroll; works on
  local and SFTP.

### 1.3 Encodings

`encoding_rs` + BOM sniffing + heuristic detection (UTF-8 validity, UTF-16 nul pattern, legacy
single-byte frequency); manual override from a codepage menu (`Shift+F8`) with a recent list
and per-extension defaults. Invalid sequences render as a visible replacement glyph with the
raw byte shown in the status line. Line endings CRLF/LF/CR/NEL all recognised and reported.

### 1.4 Search

- Plain, case-sensitive/insensitive, whole word, **regex** (`regex` crate, with a note that it
  is line-scoped by default and a "multiline" toggle), **hex pattern** (`4D 5A ?? 00`), and
  "fuzzy" (diacritics- and ligature-insensitive — FAR's idea, implemented via Unicode NFKD
  folding).
- All hits highlighted; a hit list (`All matches`) opens a jump menu; `n`/`N`, `Shift+F7`,
  `Alt+F7` navigate. Search runs in the background on a copy of the window reader so the UI
  keeps scrolling, with a progress indicator on big files.
- Search history shared with the editor and the find tool.
- **One file here, many files there.** A pattern that is worth running on one file is usually
  worth running on a tree, so the hex pattern and the regular expression are the same objects
  the find tool takes (SPEC 06 §1), and the hit list is the same list. The viewer does not grow
  its own multi-file search; the finder grows a hex mode and hands a hit back to the viewer at
  its offset.

### 1.5 Navigation extras

Go to offset / line / percent (`Alt+F8`, `:`), bookmarks 0..9 (set `Ctrl+Shift+N`, go
`Alt+N`), position undo (`Ctrl+Z`), next/previous file in the panel (`+`/`-`) keeping mode and
encoding, "jump to this file in the panel" (`Ctrl+F10`), copy selection (`Ctrl+C`) including
hex-mode byte ranges.

### 1.6 Compiled files: disassembly and decompilation

Two different things, and the viewer is honest about which one it is doing.

**Disassembly — in process, no external tool.**

| What | How |
| --- | --- |
| Recognise the file | magic bytes: ELF, PE, Mach-O (incl. universal), WebAssembly, Java class, `.pyc`, `ar` archive, or raw |
| Read its structure | `object` (MIT/Apache-2.0): architecture, bitness, entry point, sections, function symbols |
| Decode instructions | `iced-x86` (MIT) for x86 and x86-64, Intel syntax, exact instruction lengths |
| Row format | `address  bytes  mnemonic operands`, with a `<symbol>:` heading where a function starts |
| Bad bytes | one `(bad)` line per byte and the sweep resyncs one byte on, so data between functions does not hide the code after it |
| Scrolling | down decodes the next instruction; up uses the trail of offsets we came down through, because backward decoding of a variable-length instruction set can only guess |
| Huge files | ELF and PE keep their section tables at the *end*, so a prefix cannot be parsed. Above a 64 MiB limit (a debug build of a Rust program is routinely 100 MB) the viewer sweeps **raw bytes** with the host architecture and the status line says `raw x86-64 sweep (over the 64 MiB parse limit)` |
| Other architectures | aarch64 and the rest are not decoded in process: the error names the external-tool route rather than pretending |

**Bytecode — in process, because it is a table not an analysis.** Where a format carries its
own symbols, there is nothing to guess, so we read it ourselves:

| Format | What `F5` produces |
| --- | --- |
| `.class` | a `javap -c`-style listing: class file version, flags, fields, and every method's bytecode with the constant pool resolved into the comments (`invokevirtual #17 // java/io/PrintStream.println:(Ljava/lang/String;)V`), absolute branch targets, `tableswitch`/`lookupswitch` with their cases, `wide` applied to the instruction it widens, and exception handlers |
| managed PE (.NET) | an `ildasm`-flavoured listing: the CLI header and metadata root, the `#~` table stream walked as far as MemberRef, methods grouped by type, tiny and fat method bodies, and IL with tokens resolved to names — `ldstr` literals come out of the `#US` heap as text |
| `.wasm` | the canonical text format via `wasmprinter`, as close to source as a module has |

The class-file parser is written here rather than taken as a dependency: the format is small
and completely specified (JVMS §4), and the interesting part — resolving the constant pool into
readable operands — is work no parser crate does for us. The .NET reader stops where honesty
does: signatures, generics and custom attributes are not parsed, and a token it cannot name
prints as `MemberRef(7)` rather than a guess.

Note on `CAFEBABE`: it is both a Java class file and a Mach-O universal binary. They are told
apart by what follows — a class file's major version is at least 45, a fat Mach-O's
architecture count is small.

**Decompilation — the specialists, called not reimplemented.** Turning machine code back into
C is a research project (Ghidra, RetDec, ILSpy); SupremeCommander runs the installed one and
views its output:
- Everything else goes through a `[[decompile]]` rule (SPEC 07): `javap -c -p` for `.class`,
  `ilspycmd` for .NET assemblies, `python -m dis` for `.pyc`, `objdump -d -C` for an
  architecture we do not decode, `retdec-decompiler` for an `.exe`. A rule is offered only when
  its program is actually installed; when none is, the message says what to install.
- The command runs **directly, never through a shell**, so a path cannot turn into a command,
  on a worker thread with its output capped at 32 MiB.
- The result is shown as generated text: the status line reads `javap of Main.class`, and
  search, wrapping and go-to work on it like on any file.

Keys: `F5` disassemble, `F6` decompile. Both are refusable: the viewer never silently stays on
the old content.

Nothing here circumvents a protection: it reads bytes the user can already read. What those
bytes may be used for is the user's licence question, not the file manager's.

### 1.8 Structure pane (`F11`)

The disassembler already parses a binary enough to find its architecture and entry point. The
structure pane shows the rest of what the parse already knows, which is most of what someone
opening a binary in a file manager actually wants: **what is this file, and where inside it is
anything.**

It is a pane, not a mode: it sits beside the hex or disassembly pane, `Tab` moves the focus
between them, and `Enter` on a row takes the other pane to that offset. That is the whole point
— the tables are navigation, not a report.

| Group | Rows |
| --- | --- |
| Identity | format, architecture, bitness, endianness, kind (executable, shared object, core, object file), entry point, and for a universal binary the list of slices |
| Header | the format's own header, field by field: name, raw bytes, and the decoded meaning (`e_machine 003E x86-64`) |
| Sections / segments | name, file offset, virtual address, size, flags, with the one the cursor is in marked |
| Imports / exports | library and symbol, with the file offset of the entry |
| Strings of note | the interpreter (`/lib64/ld-linux-x86-64.so.2`), the `rpath`, the SONAME, the build id |

Rules it follows:

- **It reads bytes, it never loads a file.** `object`'s `ReadCache` over the open handle, so an
  ELF whose section table lives at the end of a 3 GB file costs two seeks, not 3 GB of memory.
  The 64 MiB parse limit of §1.6 applies to the *disassembler's* need for a symbol table, not to
  reading a header, so a huge file still gets its identity and its section list.
- **A field it cannot decode is shown raw**, never guessed, and the pane says which parser
  produced it.
- **A broken file is a normal case**, not an error dialog: a truncated header fills the rows it
  can and marks the rest `(truncated)`. Anyone opening a corrupt binary is opening it *because*
  it is corrupt.
- **Nothing is executed and nothing is trusted.** Offsets out of range, sizes past the end of
  the file, a section count of 0xFFFF — all of it is hostile input from a program whose job is
  to look at other people's files, and every field is range-checked against the real length
  before it is used.

Formats, in the order they are worth having: ELF, PE (including .NET's CLI header) and Mach-O
first, because those are the files people open; then archives (`ar`, and the member list of a
`zip`), then the older DOS and EFI families if anyone asks.

### 1.9 Following a branch

In the disassembly pane, `Enter` on an instruction whose operand is an address — a `call`, a
`jmp`, a conditional branch, a `lea` of a string — moves to it; `Backspace` comes back. The
stack of places jumped from is shown at the foot of the pane, so a walk into a function has a
way home. `Alt+F8` already takes an offset and learns to take a virtual address too (`0x1B50`
means the file offset; `va:0x401B50` the loaded address), because a disassembly listing names
addresses, not offsets.

Where the structure pane has read a symbol table, the operand carries the name
(`call 0000001F80 <scmd_main>`); where it has not, the address stands alone. The viewer never
invents a name.

### 1.10 Raw devices

A block device is a file: on Linux and macOS the viewer and the hex pane open `/dev/sdb` or a
partition with no special case at all. Two things do need care, and they are the whole feature.

**The size.** `stat` reports zero for a device, so a viewer that believes it shows an empty
screen over a terabyte of disk. It is measured instead, by seeking to the end — which the
kernel answers for a block device without reading anything, and which needs no `ioctl` and no
C dependency (`scmd_core::device::length`). What cannot be measured that way — a character
device, a pipe — is reported as an unknown length rather than as an empty file. A directory
listing never measures: opening every entry of `/dev` to list it could block on a tape or a
modem, so the panel shows the metadata's zero and the size appears when the device is opened.

**Writing.** A device opens **read-only**, and the status line names it and the size it
measured. The refusal is not a policy but the shape of the save: an atomic save writes a new
file beside the old one and renames it over the top, and there is no such move onto a disk.
Writing the bytes *out* to a file is allowed and is how a device is dumped — `F12` on a
marked block, or `F2` to a name of its own.

Windows needs `\\.\PhysicalDrive0` handling, where neither the seek nor the open works the
same way: it is a later addition and the backlog carries it.

### 1.7 Preview pipeline

```toml
[[preview]]
match = "*.png,*.jpg,*.webp,*.gif"
kind  = "image"           # built-in: image | markdown | archive | media | json | csv | hexdump
[[preview]]
match = "*.pdf"
cmd   = "pdftoppm -png -r 72 -f {page} -l {page} {file} -"
kind  = "image-stream"
timeout = "2s"
[[preview]]
match = "*.odt"
cmd   = "odt2txt {file}"
kind  = "text"
```

Rules: every preview runs in a task with a timeout and a memory cap; failures degrade to text
or hex, never block; images use kitty/iTerm2/sixel protocols when detected, otherwise a
block-character renderer (`chafa`-style) is built in; results are cached by (path, size, mtime,
geometry). Quick-view panel uses the same pipeline at lower priority.

## 2. Editor (`F4`)

**Implemented today** (M1, `scmd-edit`): open and save with the encoding, EOL, BOM and
final-newline of the original preserved (`Eol::detect`, UTF-8/UTF-16LE/BE/CP1252/CP437);
`Shift+F4` for a new file; atomic save through `name.scmd-tmp` + rename with the permissions
carried over; save-as; reload; refusal to overwrite a file that changed on disk without a
confirmation; read-only files opened and marked as such. Editing: insert, newline with
auto-indent, backspace, delete, delete to end of line, delete word left, tab expansion, indent
and outdent, delete / duplicate / move line, undo and redo grouped per command, stream blocks
(`Shift+arrows`) and **column blocks** (`Alt+Shift+arrows`) with copy / cut / paste (a column
paste pads short lines), select all, plain search with wrap, replace once and replace all (the search engine
is the viewer's, so regex and case-insensitive matching are already there, waiting for a dialog
that offers them), go to line or `line:col`, bookmarks 0–9, smart home, and horizontal scrolling with tabs and wide
characters counted in display columns.
**Still to come**: the rope (today's buffer is a `Vec<String>` of lines — enough for the
256 MiB open limit and far simpler to keep correct; it will be swapped for a rope when
streaming edits of larger files land), typing-burst undo grouping, the OS clipboard (the
clipboard is internal to the editor for now), persistent-block mode, multi-caret, regex and
capture-group replace, `.editorconfig`, sort/unique/reverse on lines,
macros, the buffer list (§2.3) and the external editor (§2.4).

### 2.1 Buffer

- Rope (`ropey`-style) with line index, undo tree, and a dirty-region tracker for rendering.
- Opens files up to the configured limit (default 256 MiB) fully; above it, read-only "huge
  file" mode with chunked editing refused and an explanation.
- Encoding on open = viewer's detection; on save: keep original encoding and EOL by default
  (FAR's `KeepEditorEOL`), with explicit conversion commands. BOM preserved unless changed.
- Atomic save: write `name.scmd-tmp` in the same directory, copy permissions/owner where
  possible, fsync, rename. Option for in-place save for special files (`/proc`, sockets,
  hardlink-sensitive paths).
- External-change detection: if the file changed on disk while open, offer reload / keep /
  save-as / diff.

### 2.2 Editing model

| Area | Behaviour |
| --- | --- |
| Blocks | stream blocks (`Shift+arrows`) **and column blocks** (`Alt+Shift+arrows`) with the FAR semantics: typing replaces the block optionally, `Alt+I`/`Alt+U` indent/outdent, persistent-block mode copies/moves a block to the cursor (`Ctrl+Shift+P`/`Ctrl+Shift+M`) |
| Multi-caret | derived from a column block or `Ctrl+D`-style "add next occurrence" (M3) |
| Undo | grouped by typing burst (300 ms) and by command; configurable depth; redo; undo survives save |
| Clipboard | OS clipboard via `scmd-platform` (X11/Wayland/macOS/Windows, OSC 52 fallback over SSH); internal ring of the last 10 blocks; append-to-clipboard (`Ctrl+Shift+C`) |
| Indentation | detect per file (spaces vs tabs, width), auto-indent, smart home, trailing-space trim on save (opt-in), `.editorconfig` support |
| Lines | duplicate, delete, move up/down, join, sort selected lines, unique, reverse, number |
| Search/replace | shared engine with the viewer, plus **replace with capture groups**, replace-in-selection, count, and "preserve style" (match the case/separator convention of the replaced token — FAR's feature, re-implemented from the described behaviour) |
| Bookmarks | 0..9 per file, persisted in `history/editor.toml` along with the cursor position |
| Syntax | shipped — see §2.5. Bracket matching and a rainbow option are still to come |
| Big edits | search/replace across a whole file streams through the rope without materialising copies |
| Macros | record/play keystroke macros, scoped to the editor, saved by name |

### 2.5 Syntax highlighting

`scmd-syntax` is a scanner, not a parser, and deliberately: it knows how a language writes a
comment, a string and a number, plus the words anyone would point at and call keywords. That
colours real files correctly and fits on a page, where a grammar per language would be a
compiler front end and a build-time dependency per language on top.

The unit is a line and the state carried from the one before it — a block comment, an
unterminated string, a Markdown fence. That is what makes both callers possible:

- The **editor** keeps the state each line starts in and extends it as the screen asks for
  lines further down, throwing it away from the first line an edit touched. Closing a comment
  uncolours the lines below it in the same frame, and a file of a million lines costs the
  scan of what is on screen plus whatever is above it, once.
- The **viewer** starts the scan at the first row on screen, because it never reads more of a
  file than it shows. A block comment opened above the window is therefore not carried; that
  is the price of opening a two-gigabyte log instantly, and it is paid only by the viewer.

Recognition is by file name. The classes are `keyword`, `type`, `string`, `number`,
`comment`, `preproc`, `tag`, `attr`, `entity`, `function`; each is a `syntax.*` theme key, and
every built-in theme defines all of them in its own palette. Only the foreground is set, so a
coloured run never changes the colour of the page behind it. `ui.syntax = false` turns the
whole thing off, and the editor's status line names the language it is reading.

### 2.3 Multiple files

Buffer list (`F12`), next/previous buffer, "open all selected files from the panel", recent
files with positions, and a modified-indicator in the title. Closing with unsaved changes asks
per file with a "save all" action.

### 2.4 External editor

`$VISUAL`/`$EDITOR` or a per-mask association (`assoc.toml`) can replace the internal editor
for chosen types; the panel waits for the process and refreshes afterwards. `Alt+F4` always
flips internal↔external (FAR's rule).

### 2.5 Hex mode (`F4` on a binary)

The viewer is read-only by contract — that is what lets it open a 40 GB file by seeking — so
editing bytes belongs here, in the editor, where undo, atomic save and the unsaved-changes
question already exist. `F4` on a file the panel knows to be binary opens the editor in hex mode
instead of text mode; `Shift+F4` picks the mode by hand either way.

| Thing | Behaviour |
| --- | --- |
| Panes | offset, hex, decoded; `Tab` moves between hex and text, typing in either writes the same byte |
| Insert vs overwrite | `Ins` toggles, and the title says which. Overwrite is the default, because in a binary a shifted byte usually breaks the file |
| Changed bytes | drawn in the "modified" colour until written, with a patch list — offset, old byte, new byte, and where the bytes are code, the instruction before and after |
| Undo | the editor's own undo stack, one entry per edit, so `Ctrl+Z` works the same as in text |
| Save | `F2`, through the same temp-file-and-rename path as a text save, so an interrupted save never leaves a half-written binary. In-place writing (no copy, for a device or a file too big to duplicate) is a separate, explicitly confirmed mode |
| Growing a file | insert mode past the end extends it; truncating is an explicit block operation, never a side effect |

### 2.6 Blocks (`F3` marks, `F12` operates)

A marked byte range with operations on it, which is the other half of a usable hex editor:

| Operation | Notes |
| --- | --- |
| Write to a file | carve the block out; large ones go through the job engine (SPEC 03) with a progress bar and `Esc` |
| Read a file into it | overwrite or insert, the dialog says which and how the lengths compare |
| Fill with a byte | the byte is typed in hex or as a character |
| XOR with a key | a byte or a repeating key; the name says what it is, so nobody mistakes it for encryption |
| Copy / move from elsewhere | source offset and length in hex, within the same file; a move that would land inside its own source is refused rather than half done |
| Insert / delete | shifts everything after, with the new length shown before it happens |

Every one of them is a planned operation with a preview of the result's length, because the
failure mode of a hex editor is a file that is silently one byte wrong. `Esc` out of the menu,
or out of the question one of these asks, goes back to the editor with its patches intact:
changing one's mind about a fill byte must never be a way to lose an afternoon's work.

### 2.7 Search by instruction pattern (`F7` in the disassembly)

Over a disassembly, `F7` searches the instructions, not the bytes: nobody looking for a `call`
wants the four letters of it to appear in the file. The query is matched against the text the
disassembler prints, case-insensitively, with the operands' column padding collapsed so the
pattern does not have to know about it:

| Pattern | Finds |
| --- | --- |
| `call` | every call, `call rax` and `call 0x401000` alike |
| `mov *, eax` | every move into `eax`, whatever the destination |
| `jne *` | every conditional jump of that kind |
| `cmpsb` | `repe cmpsb` as well — a prefix does not hide the instruction |

`*` stands for whatever lies between two pieces of the pattern, and the pieces must appear in
the pattern's own order. With no `*` the query is simply a substring. `Shift+F7` and `Alt+F7`
walk to the next and the previous match, as they do for an ordinary search.

It decodes a window at a time, forwards from where the screen is or backwards towards the
start, because an instruction stream has no index to jump about in; `Esc` cancels a long walk.
For an architecture we cannot decode the search says so rather than searching the bytes behind
the reader's back.

### 2.8 Assembler (`F9`)

One instruction at a time, written where the cursor is. The dialog names the offset and the
architecture it is assembling for — the file's own header decides that, so a 32-bit PE is
assembled as 32-bit code on a 64-bit machine — and what comes back names the bytes, the
instruction the *disassembler* reads in them, and by how much the length changed if it did.
`Enter` writes into the patch list of §2.5, so `Ctrl+Z` takes it back and `F2` is still what
commits. The instruction typed last comes back with the dialog, because patching is repetitive
work.

The subset is the one a patcher types, and it is deliberately a subset:

| Form | Examples |
| --- | --- |
| no operands | `nop`, `ret`, `ret 8`, `int3`, `leave`, `syscall`, `ud2`, `cld`, `cqo` |
| register to register | `mov rbp, rsp`, `xor eax, eax`, `add r9, r10`, `cmp al, bl`, `xchg eax, ecx`, `test rcx, rcx` |
| register and a number | `mov eax, 0x2a`, `mov rax, 0x123456789`, `sub rsp, 0x18`, `and al, 0x0f` |
| one register | `inc rcx`, `dec rsi`, `not ebx`, `neg rdx`, `push rbp`, `pop r15`, `mul rbx` |
| a branch to an address | `jmp 0x401010`, `je 0x401040`, `call 0x402000`, `jmp short 0x401010`, `jne near 0x402000` |
| a branch through a register | `call rax`, `jmp r11` |

A number is whatever the calculator of §2.9 reads: `0x2a`, `2Ah`, `0b101010`, `42`, `0x10+6`.
A branch is written as the address to arrive at, not as a displacement, and the shortest form
that reaches it is chosen unless `short` or `near` says otherwise.

**Memory operands are not assembled.** `mov [rbx+rcx*4+8], eax` needs a full ModRM and SIB
parser, and brings with it exactly the addressing-mode mistakes a patcher cannot afford; it is
refused by name rather than guessed at, and the hex pane is still there for bytes nobody can
spell. The same goes for the floating-point and vector instruction sets: this is not a compiler
and does not pretend to be one.

Every encoding is decoded again by the disassembler before the dialog shows it, so what appears
is the instruction the processor will see rather than the one that was meant — and a bug in the
encoder shows up as a refusal rather than as a corrupted binary. The round trip is a test as
well: every form above is assembled, decoded, and assembled again from the decoder's own
spelling, and the bytes have to come out the same.

### 2.9 Calculator (`Alt+=`)

One line of arithmetic answered in decimal, hex, octal, binary and as characters at once, with
the bitwise and shift operators and literals in every base the tools around it use (`0x1f`,
`1Fh`, `0b1010`, `0o37`, `'PE'`). It is what a hex editor is asked for every ten minutes — how
far is this offset from that one, what is this field with the top bit set — and the same parser
reads the numbers typed into the assembler and the block operations, so a number means the same
thing everywhere in the program.

## 3. Diff viewer (M3)

Side-by-side and unified views of two files (or the two panels' cursor items, or a file against
its on-disk version), with word-level intra-line diff, synchronised scrolling, next/previous
change, copy-block-to-other-side (producing a normal job), ignore-whitespace/case/EOL options,
and binary comparison (first differing offset, hex context). Backed by a Myers/patience diff
implementation in `scmd-core`.
