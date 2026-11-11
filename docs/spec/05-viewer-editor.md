# SPEC 05 — Viewer and editor

**Implemented today** (M1): the viewer's text, dump and hex modes; no-wrap / character / word
wrapping; encoding detection (BOM, UTF-8 validity, the UTF-16 NUL pattern) with manual
switching between UTF-8, UTF-16LE/BE, CP1252 and CP437; plain, regex and hex-pattern search
with highlighted hits and next/previous; go to offset, `0x` offset or percentage; position undo;
bookmarks 0–9; follow mode; windowed reads with no line index, so file size does not matter.
Also implemented: **disassembly** of ELF/PE/Mach-O/raw x86, x86-64 and AArch64 with sections and
symbols, **JVM class files and .NET assemblies** read in process (constant pool, members and
JVM bytecode; metadata tables and CIL), and **decompilation** through an installed external
tool (§1.6).
Also implemented: the **editor** (`F4`, `Shift+F4` for a new file) — see the note under §2.
Also implemented: **syntax highlighting** in both (§2.5), for the languages and markups
`fcmd-syntax` knows, off with `ui.syntax = false`.
Since then the filtered view (§1.10), the next and previous file of the panel from inside the
viewer, AArch64 decoding in process and the side-by-side diff (§3) have shipped.
**Still to come**: the "all matches" list (§1.4), the preview pipeline (§1.7), and Python
bytecode and the other architectures decoded in process.

## 1. Viewer (`F3`)

### 1.1 Modes

| Mode | Purpose | Notes |
| --- | --- | --- |
| `text` | decoded text, wrap off/char/word | arbitrary line length, tabs expanded per config |
| `dump` | raw bytes as characters, fixed width | for binaries where text decoding lies |
| `hex` | offset + hex + ASCII/decoded pane | bytes-per-row adjustable (the arrows, since sideways means nothing else in a dump) in **blocks of eight** — 8, 16, 24, 32, 40, 48, 56, 64, the group the dump already draws a gap between — the widest the window can hold, and jumps aligned to the width |
| `markdown` | Markdown laid out to be read: the marks off, headings ruled, lists bulleted, tables drawn | **Implemented** (`Shift+F2`, and in `Shift+F4`'s ring). The one mode that reads the whole file, because a table's columns cannot be laid out from the part of it on screen: limited to 4 MiB, over which the file stays its source and says so. Every rendered line carries the `(offset, sub)` pair of the source line it came from, so scrolling, the bookmarks and the search need no line index. A search here searches what is on screen, not the marks behind it. |
| `table` | a tabular file as columns: a header row, aligned columns, horizontal scroll | see §1.11 — one mode for CSV, TSV, JSONL and DBF, because what differs between them is the reader and not the screen |
| `image` | a picture or a frame of a film, in real pixels or half blocks | see §1.14 — PNG, JPEG, GIF and BMP, decided by the file's first bytes; `F3` on a picture opens here and `F4` goes on to the bytes. §1.14a is the pixels, §1.15 the films |
| `preview` | rendered content (markdown, PDF page, media info, archive listing) | pluggable, see §1.7 |
| `structure` | a side pane that decodes what the file *is*: header fields, section and segment tables, imports and exports | `F11`, see §1.8; it rides alongside `hex` and `disasm` rather than replacing them |

Mode indicator in the status bar (`t`/`d`/`h`/`m`/`b`/`p`) plus encoding, position (offset, %,
line) and file size. `F2` toggles wrap in text mode.

**`F4` turns the wheel**: text → dump → hex → the laid-out document → the table → text again,
skipping the turns this file has no use for (a plain text file reads as no document and no
table). A file is often none of the things its name says it is, and trying the next way of
reading it should be one key rather than a choice from a list; the key bar says where the next
turn lands rather than naming one destination for ever. `Shift+F4` is hex and straight back,
for when hex is the one that was wanted. Disassembly stays on `F5`: it applies to a program and
to nothing else, and a wheel that stopped on it for every text file would be a wheel with a
hole in it.

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
single-byte frequency); manual override from the code page list of §1.12. Invalid sequences
render as a visible replacement glyph with the raw byte shown in the status line. Line endings
CRLF/LF/CR/NEL all recognised and reported. Per-extension defaults are still owed.

### 1.12 The code page list (`Shift+F8`)

**What the program decodes with.** Five code pages are handled here because they have to be:
UTF-8 and the two UTF-16 byte orders are what a BOM names, and CP437 is the DOS box-drawing
page `encoding_rs` does not ship. Everything else is one of `encoding_rs`'s own encodings —
the Windows pages, CP866 and the two Macintosh ones, the ISO-8859 family, KOI8-R and KOI8-U,
and the East Asian multi-byte pages — which is what makes the list as long as FAR's without a
table per page in this repository.

**The list.** `Shift+F8` in the viewer, which is FAR's own key there (on a panel `Shift+F8`
deletes). It is a list one types into, the page the file is being read as is marked, and:

| Key | What it does |
| --- | --- |
| `Enter` | read the file as that page, and say so in the status line |
| `Ins` | mark the page as one `F8` walks, or unmark it. A marked page is shown with a `*` and comes first in the list |
| `F4` | give the page a name of your own, opening on what it is called now; an empty answer gives it back the program's name |

**`F8` walks the marked ones.** The list is thirty-odd pages long and nobody cycles through
thirty; marking three is what makes one key enough. With nothing marked, `F8` walks the five
the program handles itself, which is what it did before any of this existed.

**Written down.** A `[[codepage]]` block holds the two answers — whether `F8` walks a page and
what to call it — and *Save setup* writes them (SPEC 07). An id is the WHATWG name
(`windows-1251`, `KOI8-R`), because an id has to survive the list being reordered; reading one
is generous about how it was typed, so `cp1251`, `CP-1251` and the bare `1251` are the same
page.

**A file one comes back to.** `[viewer] remember` (on by default, and a row in *Viewer
settings*) makes a file open where it was left: the offset of the first row, the code page it
was being read in, the mode, the wrap, and the ten numbered bookmarks. It is written in the
state file, newest first and capped at five hundred and twelve files, and each answer is put
back on its own — a file remembered in a mode this build no longer has still opens at the
offset it was left at. The state is taken when the viewer closes, whichever of the dozen keys
closed it, and at exit for the file still on the screen.

Somebody who reads log files wants the end of every one rather than where they stopped last
week, which is why it is a setting and not a rule.

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
encoding, "jump to this file in the panel" (`Ctrl+F10`: the viewer closes and the cursor is on
the file — in the panel's folder when a row there is that file by its whole path, otherwise
in the file's own folder, read first), copy selection (`Ctrl+C`) including
hex-mode byte ranges.

### 1.5b Chrome (`Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S`)

Three rows of chrome the reader may take away one at a time, FAR's keys for each: the key bar
(`Ctrl+B`), the top line with the name and the position (`Ctrl+Shift+B`), and a scrollbar down
the right edge (`Ctrl+S`), which is the one that is off until asked for. Each row taken away is
a row of content gained, which is the whole reason for the keys — one long line, or a wide hex
dump, wants the window.

`Ctrl+B` was one of three spellings of page-up here, a vi habit, and `PgUp` and `Ctrl+Up` still
page up: FAR's key goes to FAR's feature and nothing is lost. What a key does is for the file
in front of the reader and is never written back; `[viewer]` (SPEC 10 §3.6b) is what a file
opens with.

The scrollbar says *where* in the file the reader is, not how much of it is on screen: how many
bytes a screenful holds is not a fixed fraction of a file — a wrapped line, a hex dump and a
rendered Markdown page each answer differently — so the thumb is one row at the percentage the
top line already shows.

### 1.6 Compiled files: disassembly and decompilation

Two different things, and the viewer is honest about which one it is doing.

**Disassembly — in process, no external tool.**

| What | How |
| --- | --- |
| Recognise the file | magic bytes: ELF, PE, NE, LE/LX, TE, Mach-O (incl. universal), WebAssembly, Java class, `.pyc`, `ar` archive, or raw |
| Read its structure | `object` (MIT/Apache-2.0): architecture, bitness, entry point, sections, function symbols |
| Decode instructions | `iced-x86` (MIT) for x86 and x86-64, Intel syntax, exact instruction lengths; `yaxpeax-arm` (0BSD) for AArch64, where every instruction is four bytes |
| Row format | `address  bytes  mnemonic operands`, with a `<symbol>:` heading where a function starts |
| Bad bytes | x86: one `(bad)` line per byte and the sweep resyncs one byte on, so data between functions does not hide the code after it. AArch64: one `(bad)` line per 32-bit word, because a fixed-width instruction set has no sync to lose and a byte-at-a-time resync would turn one unallocated encoding into four lines of noise |
| Scrolling | down decodes the next instruction; up uses the trail of offsets we came down through, because backward decoding of a variable-length instruction set can only guess — on AArch64 it is four bytes back, which is arithmetic |
| Huge files | ELF and PE keep their section tables at the *end*, so a prefix cannot be parsed. Above a 64 MiB limit (a debug build of a Rust program is routinely 100 MB) the viewer sweeps **raw bytes** with the host architecture and the status line says `raw x86-64 sweep (over the 64 MiB parse limit)` |
| Branch targets | asked of the decoder's own operands, never of the formatted text. On AArch64 the displacement in the encoding is turned into the address it names, because `b $+0x14` is what the bytes say and `b 401014h` is what the reader wants — and it is what `Enter` then follows |
| Other architectures | 32-bit ARM, RISC-V, MIPS, PowerPC and the rest are not decoded in process: the error names the external-tool route rather than pretending |

**Reading an architecture and writing it are two questions.** `Arch::disassemblable` answers
the first and `Arch::assemblable` the second, and the second is the shorter list: the assembler
of §2.8 is x86 only. On an ARM binary `F9` in the hex pane says so and writes nothing, which is
the only safe answer — a dialog that fell back to x86 there would put four bytes of nonsense
into somebody's binary and show them a plausible listing of it.

**Bytecode — in process, because it is a table not an analysis.** Where a format carries its
own symbols, there is nothing to guess, so we read it ourselves:

| Format | What `F5` produces |
| --- | --- |
| `.class` | a `javap -c`-style listing: class file version, flags, fields, and every method's bytecode with the constant pool resolved into the comments (`invokevirtual #17 // java/io/PrintStream.println:(Ljava/lang/String;)V`), absolute branch targets, `tableswitch`/`lookupswitch` with their cases, `wide` applied to the instruction it widens, and exception handlers |
| managed PE (.NET) | an `ildasm`-flavoured listing: the CLI header and metadata root, the `#~` table stream walked as far as MemberRef, methods grouped by type, tiny and fat method bodies, and IL with tokens resolved to names — `ldstr` literals come out of the `#US` heap as text |
| `.wasm` | nothing: a module is bytecode, so a raw sweep would be nonsense. `F5` says there is no disassembler for WebAssembly and points at the configuration, where a `[[decompile]]` rule names `wasm2wat`. Printing one in process cost 1,318,704 bytes of binary — 7.3% of it, five times what the x86 and AArch64 decoders cost together — for the format fewest readers open, and the tool that does it properly is the one WebAssembly's own authors ship |

The class-file parser is written here rather than taken as a dependency: the format is small
and completely specified (JVMS §4), and the interesting part — resolving the constant pool into
readable operands — is work no parser crate does for us. The .NET reader stops where honesty
does: signatures, generics and custom attributes are not parsed, and a token it cannot name
prints as `MemberRef(7)` rather than a guess.

Note on `CAFEBABE`: it is both a Java class file and a Mach-O universal binary. They are told
apart by what follows — a class file's major version is at least 45, a fat Mach-O's
architecture count is small.

**Decompilation — the specialists, called not reimplemented.** Turning machine code back into
C is a research project (Ghidra, RetDec, ILSpy); FinalCommander runs the installed one and
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

**Colours (0.31.0).** The hex view and the disassembly are coloured from the theme's syntax
keys, which every theme already defines, under `ui.syntax` like the source colouring. In hex:
the offset in `syntax.type`; each byte by what it is — `00` in `syntax.comment`, the printable
ones in `syntax.text`, control bytes in `syntax.number`, the high half in `syntax.preproc`,
`FF` in `syntax.keyword` — and the character column's letters in `syntax.string` with its dots
dim. In the disassembly: the address in `syntax.type`, the bytes dim, the mnemonic by what it
does to the flow — jumps, calls, returns and traps in `syntax.preproc`, `nop` and its kin dim,
`push`/`pop` in `syntax.function`, the rest in `syntax.keyword` — then registers in
`syntax.string`, numbers in `syntax.number`, the size words (`qword ptr`, `short`) dim, and a
`<symbol>` in `syntax.function`, which a label row carries in bold. The runs are read off the
row's fixed columns (`binary_runs`), so a row the viewer lays out differently is simply left
plain rather than coloured wrong.

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
| Strings of note | the interpreter (`/lib64/ld-linux-x86-64.so.2`), the `rpath`, the SONAME, the build id; since 0.38.0 also the ABI note (`Linux 3.2.0 or later`), a Go build id, the systemd package note, the `.gnu_debuglink` file and CRC, and the compilers `.comment` names |
| PE: about the image (0.38.0) | the linker, OS and subsystem versions, the checksum, `DllCharacteristics` as mitigations (ASLR, DEP, CFG…); **Version** — `VS_FIXEDFILEINFO` and every `StringFileInfo` string (company, description, product, copyright, original name) and the translations; **Manifest** — the requested execution level and DPI awareness; **Debug** — the CodeView PDB path, GUID and age, a reproducible-build entry; **Signature** — each `WIN_CERTIFICATE`, the names its certificates carry and the signing time, or that the image is not signed |
| Mach-O: about the build (0.38.0) | the UUID, the platform with the oldest OS and the SDK (`LC_BUILD_VERSION` or the older `LC_VERSION_MIN_*`), the tools, the source version, whether the text is encrypted, each dylib's current and compatible version, the `LC_RPATH` entries; **Signature** — the CodeDirectory's identifier, team, flags (ad-hoc, hardened runtime) and hash, the entitlements' size, and the CMS certificates' names, or that it is not signed |
| Office documents (0.39.0) | OOXML (`docProps/core.xml` and `app.xml`: title, subject, author, keywords, comments, category, last saved by, revision, created, modified, printed; pages, words, characters, lines, paragraphs, slides, notes, editing time, company, manager, template, the application and its version) and OpenDocument (`meta.xml`: the same from `dc:`/`meta:` elements and `meta:document-statistic`), the kind from the parts present or `mimetype`. The zip is read only as far as that: the end-of-central-directory record from the last 64 kB, the directory, and the one or two entries inflated with the `flate2` the viewer already has, at most 1 MB each; a zip with neither part is not a document |
| Pictures and sound (0.38.0) | JPEG: size, encoding, density, comment and EXIF — camera, lens, date taken, exposure, aperture, ISO, focal length, orientation, and the GPS position as its own row; PNG: size, colour, density, time, `tEXt`/`zTXt`/`iTXt` text (inflated, up to 16 MiB), embedded EXIF, animation; WebP (0.40): size, encoding, animation, EXIF, XMP; **AI generation** (0.40, `aigen.rs`): ComfyUI (the API graph followed back from `KSampler` / `SamplerCustomAdvanced`), A1111/Forge `parameters` or EXIF user comment, Fooocus and SwarmUI JSON, InvokeAI `invokeai_metadata`, NovelAI, and C2PA manifests (`caBX`, WebP `C2PA`, JPEG APP11) — `Enter` on a `RowKind::Text` row opens the text as a derived view, JSON pretty-printed; `file.strip_metadata` (`strip.rs`) rewrites PNG chunks, WebP chunks (keeping VP8X flags true) and JPEG segments without decoding, in three modes (all, AI only, all but camera EXIF and copyright, with EXIF rebuilt from whitelisted IFD0/Exif tags), the original copied to `fcmd-undo/` for `Undo::Restore`; GIF: size; MP3: ID3v2.2–2.4 (title, artist, album, year, track, genre, comment, cover) or ID3v1; FLAC: the stream, its length and the Vorbis comments; Ogg Vorbis and Opus: the stream and the comments |

A signature is **read, not verified**: the rows name the certificates and say "not verified",
because checking a chain needs the trust store of the machine that will run the file, and a
pane that said "valid" from here would be saying something it does not know. Every reader of
these parts takes a bounded slice — 16 MB of resources, 1 MB of a version block, a signature
or a certificate table, 512 kB of the start of a picture or a sound — and stops at the first
length that does not fit.

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
`zip`); and the older families after them, which are there now:

- **NE**, the 16-bit New Executable of Windows 1.0 to 3.11 and OS/2 1.x. Its header and its
  segment table, whose entries are the one place in these families where a file offset falls
  out of the header arithmetic — a sector number shifted by `ne_align` — so those rows are
  what `Enter` follows.
- **LE** and **LX**, the Linear Executables: OS/2 2.0 and later, and the DOS extenders and
  Windows 9x VxDs. The two share this header as far as the object table, and differ in how a
  page becomes a file offset — `LX` shifts a page's own offset, `LE` numbers its pages from
  one data base — so the objects are listed with their sizes and permissions and the *tables*
  are what `Enter` follows. A page decoded against the wrong one of the two layouts would
  send a reader to a plausible wrong place, which is worse than sending them to the table.
- **TE**, the Terse Executable that UEFI firmware is built out of: a PE with the stub, the
  signature and most of the optional header cut off, so it has a magic of its own (`VZ`) and
  no `MZ` in front of it. `StrippedSize` is the whole of the arithmetic — the section headers
  still carry the offsets the PE had, so a file offset is that offset less what was removed
  plus this header's own forty bytes. There is no separate TE64: the header always carries a
  64-bit image base, and `Machine` is what says x86-64 from IA-32.

Then the files that are **not programs** but are still a tree of records with a length on each,
which is the same walk and the same question — what is this, and where inside it is anything:

- **ISO base media** (MP4, MOV, M4A, M4V, 3GP, HEIF/AVIF): a tree of boxes. The brands, the
  movie header's timescale and duration, every track with its handler, its codec, its stored
  size or its sample rate, the language, the iTunes-style tags of an `.m4a`, and the top-level
  boxes with one level inside `moov` as the rows `Enter` follows. A film is three gigabytes of
  `mdat` and this reads sixteen bytes per box plus the `moov`.
- **Matroska** (MKV, WebM, MKA): EBML, whose ids and lengths are variable-width integers. The
  `DocType`, the segment's `TimecodeScale` and `Duration`, the title and the writing
  application, every `TrackEntry`, and the segment's children as navigation — with the clusters
  listed once and then counted, because a two-hour film has tens of thousands of them. An
  element of unknown length, which is how a stream still being written says so, stops the walk
  at that level instead of the file.
- **RIFF** (AVI, WAVE, WebP): the ancestor of both. AVI's main header and one group per stream
  with the codec taken from the *format* chunk, because `strh`'s handler is zero in half the
  files ever written; WAVE's format and how long its data plays; WebP's canvas, which is the
  one picture size our own decoder cannot give.
- **PDF**: the index is at the end, may be a chain of them — one per incremental save — and
  since 1.5 is itself a deflated object whose catalogue lives inside *object streams*. So this
  follows `startxref`, reads either the classic table or the stream, inflates what it must
  (undoing the PNG predictor), and reports the version, the page count, the first page's paper,
  the `/Info` fields, what `/Encrypt` allows, and how many revisions the file carries. The pages
  are rows `Enter` follows; an object packed into a stream sends `Enter` to the stream that
  holds it, because that is where its bytes are. **When the index does not lead anywhere the
  file is read for `N G obj` and that is believed instead**, which is what a repair tool does
  and what a truncated download or a hand-edited file needs. An encrypted file's strings are
  ciphertext and are shown as `(encrypted)` rather than as rubbish.

What a container reader deliberately does *not* do is decode a frame, and what the PDF reader
does not do is pull out the text: the first wants a video codec — a C library and a licence
this project has refused — and the second wants the content stream's operators and the font's
`ToUnicode` map, which is a typesetting engine that still gets a scanned page wrong. Both have
an honest answer: a film's frames come from `ffmpeg` run as a program (§1.15), and a PDF's
text from a `[[decompile]]` rule with `pdftotext` (SPEC 06 §5b), chosen by mask and run on
`Shift+F6`.

What none of the three executable families gets is the equivalent of the PE reader's import and export walks:
those want a second format each — a stream of counted strings, a stream of entry bundles —
and nothing here can be tested against a real file of them.

### 1.9 Following a branch

In the disassembly pane, `Enter` on an instruction whose operand is an address — a `call`, a
`jmp`, a conditional branch, a `lea` of a string — moves to it; `Backspace` comes back. The
stack of places jumped from is shown at the foot of the pane, so a walk into a function has a
way home. `Alt+F8` already takes an offset and learns to take a virtual address too (`0x1B50`
means the file offset; `va:0x401B50` the loaded address), because a disassembly listing names
addresses, not offsets.

Where the structure pane has read a symbol table, the operand carries the name
(`call 0000001F80 <fcmd_main>`); where it has not, the address stands alone. The viewer never
invents a name.

### 1.10 Raw devices

A block device is a file: on Linux and macOS the viewer and the hex pane open `/dev/sdb` or a
partition with no special case at all. Two things do need care, and they are the whole feature.

**The size.** `stat` reports zero for a device, so a viewer that believes it shows an empty
screen over a terabyte of disk. It is measured instead, by seeking to the end — which the
kernel answers for a block device without reading anything, and which needs no `ioctl` and no
C dependency (`fcmd_core::device::length`). What cannot be measured that way — a character
device, a pipe — is reported as an unknown length rather than as an empty file. A directory
listing never measures: opening every entry of `/dev` to list it could block on a tape or a
modem, so the panel shows the metadata's zero and the size appears when the device is opened.

**Writing.** A device opens **read-only**, and the status line names it and the size it
measured. The refusal is not a policy but the shape of the save: an atomic save writes a new
file beside the old one and renames it over the top, and there is no such move onto a disk.
Writing the bytes *out* to a file is allowed and is how a device is dumped — `F12` on a
marked block, or `F2` to a name of its own.

**Windows** (shipped 0.19.0) is where a device stops looking like a file, in three ways, and
each one is handled in exactly one place:

| What is different | Where it is answered |
| --- | --- |
| The name. There is no `/dev`: a disk is `\\.\PhysicalDrive0` and a volume is `\\.\C:`, names the filesystem does not list, so they are typed rather than arrived at from a panel | `fcmd_platform::is_device_path` |
| `metadata` on such a name *fails* — it asks the filesystem about something that is not in it — and a seek to the end answers nothing. The length is asked of the device: `IOCTL_DISK_GET_LENGTH_INFO`, with the drive geometry as a second try for a device that only implements that | `fcmd_platform::geometry` |
| The handle refuses a read that does not start on a sector boundary or is not a whole number of sectors long — `ERROR_INVALID_PARAMETER`, for a call that works on any file. So the sector size comes back with the length, and every read is widened to the sectors around what was asked for | `fcmd_core::device::read_at` |

The viewer and the hex pane both read through that one function, so they cannot disagree about
it, and the arithmetic is tested against an ordinary file: reading *through* the sector path
has to give exactly the bytes reading without it gives, at every offset and every length. A
device that will not say how long it is reads as a length nobody knows rather than as empty.

Opening a raw device on Windows needs Administrator rights, and the refusal says so instead of
leaving the system's own "Access is denied" to be puzzled over. The `unsafe` is in
`fcmd-platform` with everything else that needs it, which is the rule the workspace is built
on.

### 1.13 Base64 (`Alt+B`)

A certificate, a private key, an attachment pulled out of a mail, a `data:` URI pasted into a
file: all of them are bytes somebody wrote down in sixty-four printable characters, and all of
them are unreadable until something decodes them. The viewer is where one looks at a file one
did not write, so this is where the undoing belongs.

`Alt+B` decodes the file and shows what came out, keeping the path in the title. The bytes
decide how they are drawn: text when they read as text, the hex pane when they do not, which
is the common case — what people write down in base64 is usually something that was never
text. The status line says how many bytes came out, because that figure is how one knows the
decode was the right one.

Three shapes are read, because all three turn up: the bare payload, with or without padding
and with or without line breaks; **PEM armour**, where `-----BEGIN …-----` and `-----END …-----`
are skipped and the payload between them is decoded; and a **`data:` URI**, where everything up
to and including `;base64,` is a label rather than data. Both alphabets are accepted — the
standard one ending `+/` and the URL-safe one ending `-_` — because refusing the second would
mean refusing every token a web application ever issued, and one file is never both.

**A file that is only mostly base64 is not decoded.** The refusal names the byte that stopped
it and where it is, which is the one thing worth knowing: decoding as far as the first bad
character would produce bytes nobody wrote. The whole file is decoded at once, because base64
is only meaningful whole — a window into the middle of it decodes to bytes offset by whatever
the window cut through — so there is a limit (32 MiB of payload, 24 of bytes), above which
the file is not one somebody is reading by eye.

It is the one view that *replaces* the file rather than drawing it another way, so the key
does not toggle: the viewer now holds the decoded bytes, and `Esc` and opening the file again
are the way back, as they are from a disassembly. One undoing per file: a decoded view is not
decoded again.

### 1.14 Pictures (`F3` on a picture)

A `.png`, a `.jpg`, a `.gif` or a `.bmp` opens as the picture. That is what somebody pressing
`F3` on a screenshot came for, and `F4`'s wheel goes on to the dump and the hex for the times
they came for the bytes instead — the file is still a file, and nothing about it is hidden.

**Half blocks, and why those first.** A terminal cell is about twice as tall as it is wide, so
a cell holding `▀` with the top half in the foreground colour and the bottom in the background
is two pixels that are each very nearly square. A window of `cols × rows` cells is therefore a
canvas of `cols × 2·rows` pixels. It works in **every** terminal there is, which is the reason
to do it this way before anything else: the graphics protocols — kitty's, iTerm2's, sixel —
put real pixels on the screen and are worth having, but each works in some terminals and not
others, and a picture one cannot see at all is worse than one made of blocks. They are
§1.14a, and half blocks stay underneath them as what is drawn whenever they cannot be.

- **The file decides, not the name.** PNG, JPEG, GIF and BMP are recognised by their first
  bytes. A `.txt` that is really a PNG opens as the picture and a `.png` that is really a
  JPEG opens as the JPEG, because a magic number cannot lie and a name can.
- **Scaled down, never up.** A photograph is fitted into the window keeping its shape; a
  sixteen-pixel icon is drawn at sixteen by eight cells rather than blown up to fill the
  screen, which would be a grid of coloured squares that says nothing. Centred, as every
  picture viewer draws one.
- **The scaling averages.** Each drawn pixel is the mean of the source rectangle it covers,
  so the text in a screenshot stays legible where picking one source pixel per target would
  drop half the strokes.
- **What is transparent shows the viewer's own background through it**, mixed in the
  proportion the pixel is transparent, so a PNG with a cut-out does not come out on a square
  of whatever colour happened to be stored under the alpha.
- A picture is decoded **whole** — there is no first screenful of a PNG — so the limits are
  on the file (256 MiB) and on the picture (50 megapixels), and a file above either says so
  instead of being read. A file that is damaged says what the decoder said.
- Nothing scrolls: the picture is already the size of the window. The keys are not broken,
  there is simply nowhere to go.

The decoding is `image` with `default-features = false` — the four formats a commander
actually opens, no research formats, no `rayon`, no C. The drawing is ours.

### 1.14a Real pixels: kitty, iTerm2, sixel

Where the terminal can take a picture as pixels, the picture mode sends it one. Three
protocols cover what is in use: **kitty's graphics protocol** (kitty, Ghostty, Konsole),
**iTerm2's inline images** (iTerm2, WezTerm, VS Code's terminal) and **sixel** (foot, mlterm,
contour, `xterm -ti 340`, Windows Terminal from 1.22).

**The setting.** `viewer.pixels = "auto" | "kitty" | "iterm" | "sixel" | "off"`, `auto` by
default; a row of the viewer settings dialog (SPEC 10 §3.6) and of the settings browser.

**`auto` never asks the terminal.** Asking means writing a query and reading the answer off
the keyboard's own channel: a terminal that does not answer leaves the program waiting, and
one that answers late types its answer into whatever has the focus by then. So `auto` reads
what the terminal exported before the program started and says yes only to what is known to
draw what it is sent (`fcmd_tui::pixels_from_env`):

| Environment | Protocol |
| --- | --- |
| `TMUX`, `STY` or `ZELLIJ` set, or `TERM` starting `screen` or `tmux` | none — a multiplexer draws the screen itself |
| `KITTY_WINDOW_ID` set, or `TERM=xterm-kitty` | kitty |
| `TERM=xterm-ghostty`, or `TERM_PROGRAM=ghostty` | kitty |
| `TERM_PROGRAM=iTerm.app` or `WezTerm` | iTerm2 |
| `TERM` starting `foot` or `contour`, or `mlterm` | sixel, and only when the size of a cell is known |
| anything else | none |

Deliberately not decided: Windows Terminal (it draws sixel from 1.22 and exports no version),
xterm (sixel only as a VT340), VS Code (iTerm2's images only with a setting of its own on),
Konsole (kitty's protocol, but how much of it is not told by `KONSOLE_VERSION`).
Each is a value of the setting away, and a wrong `auto` there would be a blank where a
picture was.

**The size of a cell.** kitty and iTerm2 are told how many cells an image covers and scale it
themselves; sixel is drawn pixel for pixel, so the size of a cell in pixels decides whether a
picture fits the cells left for it. In order: `viewer.cell_width` × `viewer.cell_height` when
both are set; ten by twenty for sixel in Windows Terminal, which draws sixel on a VT340's
cells whatever its font; what the system reports for the terminal's window (`TIOCGWINSZ`,
answered by the kernel — not a terminal query; nothing on Windows); eight by sixteen. It is
read again when the window changes size.

**Scaling and encoding** (`fcmd_view::pixels`). The picture is fitted to the window by the
same two rules as the half blocks — down keeping its shape, never up — at the cells' size in
pixels, and drawn onto a canvas of exactly the whole cells it covers, centred. For kitty and
iTerm2 the canvas is a PNG with transparent padding, compressed quickly rather than well. For
sixel it is composited over the window's own colour and encoded by our own encoder: a palette
of 256 chosen by median cut over a five-bit-a-channel histogram, Floyd–Steinberg error
diffusion onto it, run-length encoding, raster attributes stating the size. A picture with
256 colours or fewer keeps exactly those and is not dithered. The encoder is deterministic:
the same picture is the same bytes. All of this runs on a worker (`Request::Pixels`,
`Event::Pixels`), keyed by the picture, the window, the cell and the protocol, and is done
once per key.

**On the wire** (`fcmd-tui`). kitty: `ESC _ G a=T,f=100,i=<n>,c=<cols>,r=<rows>,C=1,q=2,m=<more> ;
<base64> ESC \` in pieces of at most 4096 characters — `q=2` so the terminal sends nothing
back, `C=1` so the cursor does not move — and `a=d,d=I,i=<n>` to delete. iTerm2: `ESC ] 1337 ;
File=inline=1;size=<bytes>;width=<cols>;height=<rows>;preserveAspectRatio=0 : <base64> BEL`.
Sixel: `ESC P 0;1;0 q "1;1;<w>;<h> … ESC \`.

**Living with the cell buffer.** A picture is not a cell and the diff-based painter knows
nothing of it. Each frame carries a list of *placements* — the cells, the protocol, the
picture's number, the bytes — handed to `Term::draw_with` beside the buffer:

- The cells under a placement are **blank cells of the window's colour** in the buffer. They
  are painted like any others, the picture is written after them, and since they do not
  change between frames the diff never writes over it. A picture is sent once, not once a
  frame.
- A placement that is **gone or changed** is taken off: kitty is told to delete the image;
  for sixel and iTerm2 the cells it covered are forgotten from the previous frame, so the
  diff writes every one of them again, which is what erases it. Where another picture takes
  exactly the same cells — the next frame of a film — the cells are not written in between.
- A placement whose cells **were written this frame** is sent again for sixel and iTerm2,
  where writing a cell erases what was drawn in it. kitty keeps its images on a layer of
  their own and needs nothing.
- After `invalidate()` — a resize, another program on the screen — every picture is taken
  off and sent again, and leaving the terminal deletes kitty's.
- **Anything drawn across a picture puts it back in half blocks.** While a frame is drawn,
  the cells of a placement hold a private mark; a dialog, a menu or its shadow replaces or
  recolours some of them, and if at the end one cell of a placement is not as it was marked
  the whole frame is drawn again with no placements. An image lies on top of the text and
  would cover what was put there; half blocks lie under it.
- Sixel and iTerm2 leave the cursor below the image, and a cursor pushed off the last row
  scrolls the screen: for them a picture never takes the last row.

**Half blocks are always there.** They are what is drawn with no protocol, while a picture's
pixels are being encoded, under a dialog, and in every test and every screenshot — drawn in
the cells the pixels will take, so a picture sharpens where it stands rather than changing
size. The film's previous frame stays up while the next is encoded, so stepping and playing
do not blink through them.

Tested against a real terminal: sixel in xterm (VT340 mode). kitty's and iTerm2's are tested
by the bytes they are sent.

### 1.15 Films: frames through `ffmpeg`

`F3` and the quick view on a video show a frame of it. fcmd decodes no video — the codecs
carry patents, the complete libraries are LGPL or GPL, and no decoder in Rust is ready — so
the frame is asked of the **`ffmpeg` on the machine, run as a program and never linked**, by
the rule every external program follows (SPEC 02 §14): found on the `PATH`, listed in the
external programs panel, and named by the viewer when it is missing.

- **What is a film.** A container the structure pane reads (§1.8) — ISO base media, Matroska,
  AVI — with a picture track. The first bytes only say "perhaps" (`media::may_be_film`); the
  tracks say, and they are read on the worker (`media::film`), which is also where the
  running time and the codec on the status line come from. An `.m4a` is a container of sound
  and is not a film; other containers (MPEG-TS, FLV, ASF) are not recognised yet.
- **One frame**: `ffmpeg -hide_banner -loglevel error -nostdin -ss <t> -i <file> -frames:v 1
  -an -sn -vf scale='min(W,iw)':'min(H,ih)':force_original_aspect_ratio=decrease -f image2pipe
  -vcodec png -`. The seek comes before the input, so the last minute is as quick as the
  first; the frame is scaled by `ffmpeg` to the window it will be drawn in, in pixels, and
  never up; fifteen seconds is the limit, after which the program is stopped. What comes
  down the pipe is a PNG, and from there a frame is a picture (§1.14, §1.14a).
- **Where it opens**: a tenth of the way in, in the viewer and the quick view alike.
- **Stepping**: `PgDn`/`→` and `PgUp`/`←` move by a tenth of the running time, `Home` and
  `End` go to the first frame and to the last one worth asking for (a hundredth of the film,
  at most a second, before its end — the very end has no frame). Steps add up before their
  frames arrive: three quick presses are three tenths. A film that does not say how long it
  is steps by ten seconds.
- **The status line** leads with `time / duration  codec width×height`.
- **Playing** (`Space`, and `Space` again to pause): `ffmpeg -re … -vf fps=8,scale=… -f
  image2pipe -vcodec png -`, a stream of PNGs cut where each ends, eight a second, without
  sound. `-re` makes `ffmpeg` keep the film's own time, the worker sends one event a frame,
  and the loop keeps no clock. It stops at the end of the film, on a step, on `F4`, and when
  the viewer closes. A preview, not a player.
- **An answer for a question that was replaced is dropped.** Every ask has a number; the
  viewer keeps the newest, the workers share it, and a worker fetching an older frame stops
  its `ffmpeg` and says nothing.
- **Without `ffmpeg`** the viewer says so on the status line, with what the film is, and the
  file stays what it opened as — `F11` still reads the container. The quick view says
  nothing: it would say it for every film the cursor passed. A frame `ffmpeg` could not give
  is an error with `ffmpeg`'s own first line in it.
- Only a file on this disk: a film on a server is fetched whole by `F3` first, as every file
  is, and the quick view does not fetch.

### 1.11 Table mode: the tabular formats

A commander opens a `.csv`, a `.jsonl` and a `.dbf` every day, and all three read as what they
are not: a wall of commas, a wall of braces, a wall of hex. What they want is one mode, not
three viewers — the screen is the same in every case (a header, columns, a cursor on a row),
and what differs between the formats is only how a record is read out of the bytes.

So: a `Table` mode beside `text`, `dump`, `hex`, `disasm` and `markdown`, and behind it a
reader trait that answers three questions and nothing else — what the columns are called, what
a column holds, and what is in the rows from *n* to *n + k*. The viewer keeps what it already
has: `F7` searches, `Alt+F8` goes to a row, `+`/`-` step to the next file, the chrome keys take
a row back, and `F2`/`Shift+F4` leave the mode for the source, because the source is sometimes
the thing one came to look at.

**Shipped in 0.6.0: the mode, CSV/TSV and JSONL. In 0.7.0: DBF, read a window at a time.** `Shift+F3` in the viewer, and a file whose
name says it is tabular opens that way (`ui.table`, on by default, with the same key as the way
back to the source). **What opens by itself and what the key opens are not the same test**: a
name that says `csv`, `tsv`, `tab`, `jsonl`, `ndjson` or `dbf` opens as a table, because what a
file opens as has to be predictable, while `Shift+F3` will make a table of anything that reads
as one — tab-separated columns in a `.txt` are as common as in a `.tsv`, and a log laid out in
columns has no extension that says so. The key refuses only what has no table in it: a file
whose first bytes are not text, or one that does not come apart into two columns or more. The
same key goes back to the source, so a wrong guess costs one keystroke. The heading and its rule are drawn first and *stay* while the rows scroll
under them, because a table whose heading scrolled away is a wall of numbers again; `←`/`→`
move the window sideways and stop where the last column ends rather than scrolling into empty
space, `Ctrl+←`/`Ctrl+→` jump a whole column (the unit a table has and a wall of text has
not), `Home` and `End` go to the first and last screenful of *rows*, `Ctrl+F3` sorts by the
column the view is on and `Ctrl+F7` filters the rows; the status line says the format, the delimiter that was guessed, the
shape of the table (`rows × columns`) and what was left out. A cell wider than forty columns is
cut with a `…` — one 4 kB cell must not push every other column off the screen — and the
horizontal scroll reads the rest. A search finds what is on the screen, so a value is found in
its cell, as in a laid-out Markdown document.

**Sorting and filtering, and not editing.** `Ctrl+F3` sorts the rows by the column the view is
on — the same column `Ctrl+←`/`Ctrl+→` walks to — ascending, then the other way round, then back
to the order the records are in the file: a sort with no third press is a sort with no way out.
`Ctrl+F7` asks what a row has to hold and keeps only those, and an empty answer gives them all
back. `Ctrl+F3` because that is the key that sorts in the panels; `Ctrl+F7` because `F7` is the
key that looks for something, and this is the other thing one does with what one is looking for.

Both are **an index and nothing else**: one record number per visible row, built from the
records and thrown away to go back. Nothing is written, the records are untouched, and every
row keeps its own offset — so a search, a bookmark and `Alt+F8` still mean the record rather
than its place in the sorted list. A column whose every cell reads as a number sorts as
numbers, so a column of sizes does not put 1000 before 9; anything else sorts as text, with the
same natural comparison the panels sort names with. The status line says what the view is
doing — `↑name`, `↓size`, `/tor` — because a table sorted by a column one cannot see sorted is
a table one mistrusts, and a sort of a file the mode read only the start of says *that* out
loud as well. Both walk every record to read the key out of it, so above `MAX_VIEW` records
(200 000) they refuse and say so rather than reading a million records for a keystroke: a table
that large is one to query rather than to sort in a viewer.

**A cursor, marks and a copy (0.38.0).** `↑`/`↓` move a cursor over the rows and the window
follows it; `Ins` and `Shift+↓`/`Shift+↑` mark the cursor's row and move on, as in a panel, and
`Ctrl+C`/`Ctrl+Ins` put the marked rows — or the cursor's — on the clipboard as tab-separated
text with the heading first, the form a spreadsheet pastes into cells, quoting a cell that holds
a tab, a newline or a quote. Marks are kept by record number, so a sort or a filter does not
move them onto other records, and the copy follows the order on screen.

**The heading is guessed, not assumed (0.38.0).** For delimited text the first record is a
heading unless what is under it says otherwise: per column, where every cell below is a number a
first cell that is one votes for data and one that is not votes for a heading; where the cells
below share one length a first cell of another length votes for a heading; an empty first cell,
or one whose value recurs below, votes for data. No vote either way is a heading, which is what
nearly every export writes. Without one the columns are `#1`, `#2`… and the note says `no
heading`; `Ctrl+H` turns the guess round and reparses. JSON Lines and dBase name their fields
and have nothing to turn. The delimiters guessed are `,` `;` tab `|` and `^`, in that order on
a tie.

**Editing a cell stays out**, and that is the decision rather than a gap: a viewer that writes
is an editor, and `F4` and the hex pane are where writing lives. Writing a cell back would also
be three jobs rather than one — a line rewritten for CSV, fixed-length records and a memo file
beside them for DBF, and nothing honest at all for a file that was read only as far as the
limit.

**The text formats are read whole**, up to a limit (`TABLE_LIMIT`, sixteen megabytes), exactly
as the Markdown render is and for the same reason — a column's width cannot be known from the
part of the file on screen. Above the limit the file opens as text, which is the viewer's usual
promise.

**A DBF is not**, and that is the shape the plan above asks for: its header says how many
records there are, how long one is and where they start, so the columns and their widths are
known without reading a single record and row *n* is one multiplication and one read. Nothing
but the window on screen is ever touched, so a two-gigabyte table opens as fast as a log does
and the size limit does not apply to it. One `Records` enum carries the difference — `Read`
for the formats that were parsed up front, `Dbf` for the one that seeks — and everything that
draws asks the table for the cells of a row rather than reaching into a vector.

What the DBF reader does with the field types: `C` is text with the padding off, `N` and `F`
are the numbers as written, `D` is `YYYYMMDD` shown as `YYYY-MM-DD` (and empty where the date
was never set), `L` is `true`/`false`, `I`/`Y`/`B`/`O` are the binary integer, currency and
double, `T` is a Julian day with milliseconds since midnight. The numeric
types sit on the right of their column and everything else on the left, because a column of
numbers is read down its last digit. A **deleted** record is shown rather than skipped — a
table whose deleted rows vanish is one whose row count does not add up, and finding them is
half of why anybody opens a DBF — and the `*` that says so has a **column of its own**, in
front of the fields, rather than being stuck on the first cell: in a column of numbers `*1`
reads as a number nobody can tell from `1`. The status line's `rows × columns` counts the
fields and not that column. The header's language byte picks the text encoding, and the DOS
code pages are read as DOS code pages: a table written in Germany under DOS carries `0x0F`
and is in code page 437, where reading it as Windows-1252 turns `temporäre` into `tempor„re`.
437 is the viewer's own table, the one it has had since it learned to show a DOS text file;
850, 852 and the other DOS tables `encoding_rs` does not carry fall back to the Windows table
that agrees with them on the accented letters, which is what a field holds. A **memo** field shows what the `.dbt` or `.fpt`
beside the table holds, on one line — a memo is written with newlines in it and a cell has one
line to give. Opening that second file is the *caller's* job, because what a path means — on
disk, inside an archive, on a server — is not the viewer's to decide: the application looks
for `.dbt` then `.fpt`, in either spelling, beside a table on disk. Three formats, each
recognised by what it holds rather than by its name: FoxPro's `.fpt` (block size big-endian
at offset 6, a block opening with its type and length), dBase IV's `.dbt` (block size at 20,
a block opening `FF FF 08 00` and its length), and dBase III's `.dbt` (blocks of 512, the
text running to the first `1A`). Where there is no memo file — a remote table whose bytes
were fetched without it, a `.dbt` that was lost — the cell says `memo:42`, the block number,
which is what there is to say. A picture or an object field says the same: a cell is no place
for either. A header that claims more records than
the file can hold is a truncated table: what is there is read and the status line says `cut`.

**The readers, in the order they are worth building.**

| Reader | What it is | The hard part |
| --- | --- | --- |
| CSV / TSV | delimiter-separated text, RFC 4180 quoting | guessing the delimiter (`,`, `;`, tab, `|`) from the first kilobytes and *saying* which was guessed; a quoted field may hold the delimiter and a newline, so a row is not a line |
| JSONL / NDJSON | one JSON object per line | the columns are the union of the keys of the first *N* records, in first-seen order, and a record that has a key the header does not shows it in an overflow column rather than being silently cut |
| DBF | dBase III/IV and FoxPro: a typed header, then fixed-length records | **Shipped in 0.7.0**, windowed: the field types, the deleted-record flag (shown, not skipped), the header's code page. The memo files beside it (`.dbt`, `.fpt`) followed in 0.7.2 — all three formats, each recognised by what it holds |

**Reading a window, not a file**, wherever the format allows it — which is the promise the
viewer makes everywhere else. DBF is the best case: a fixed record length and a count in the
header mean row *n* is one seek, so a two-gigabyte table opens as fast as a log does. JSONL is
nearly as good: a line index is built as one scrolls and kept, so going to the end is a seek
and a scan rather than a parse of everything before it. CSV is the exception and it is the
Markdown exception again — columns cannot be laid out from the part of the file on screen — so
the widths are decided from a sample (the first *N* kilobytes, configurable) and the status
line says the widths are a sample rather than a measurement. A row wider than the sample
suggested is not re-laid out: it is marked, because a table whose columns moved while one
scrolled would be worse than one that admits its guess.

**What this mode is not.** It is not a query tool and not a spreadsheet: no editing, no
sorting, no filtering in the first version — those are the obvious next asks and each wants an
answer about writing the file back, which a viewer does not have. Parquet and SQLite stay out
too, and for a stated reason rather than an oversight: Parquet means a columnar stack (arrow)
that is larger than this whole program, and SQLite is a different shape of question — several
tables and a query language — which belongs to a plugin with its own panel.

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
or hex, never block; results are cached by (path, size, mtime, geometry). Quick-view panel uses
the same pipeline at lower priority. The block-character renderer this promised is built and is
§1.14 — it is not a preview rule but a view of its own, because a picture one opened with `F3`
is the file and not a preview of it. The kitty, iTerm2 and sixel protocols are §1.14a, and a
video's frames §1.15.

## 2. Editor (`F4`)

**Implemented today** (M1, `fcmd-edit`): open and save with the encoding, EOL, BOM and
final-newline of the original preserved (`Eol::detect`, UTF-8/UTF-16LE/BE/CP1252/CP437);
`Shift+F4` for a new file; atomic save through `name.fcmd-tmp` + rename with the permissions
carried over; save-as; reload; refusal to overwrite a file that changed on disk without a
confirmation, which also offers to reload it or to see what changed (§2.2j); read-only files
opened and marked as such. Editing: insert, newline with
auto-indent, backspace, delete, delete to end of line, delete word left, tab expansion, indent
and outdent, delete / duplicate / move line, undo and redo grouped per command and per burst
of typing with a depth that is a setting (§2.2g), stream blocks
(`Shift+arrows`) and **column blocks** (`Alt+Shift+arrows`) with copy / cut / paste (a column
paste pads short lines), select all, search and replace as text or as a regular expression,
with whole words, capture references, *preserve style* and a list of every matching line
(§2.2h), go to line or `line:col`, bookmarks 0–9, smart home, and horizontal scrolling with tabs and wide
characters counted in display columns. Since 0.31.0, `Alt+B` to the matching bracket, `Alt+J`
to join the next line, `Alt+Shift+S` to sort the block's lines, `Alt+Shift+U` to drop the
ones that repeat an earlier line (the first of each stays where it is, so the block need not be
sorted), `Alt+Shift+R` to reverse them and `Alt+Shift+N` to number them from 1, right-aligned
to the widest, and `Alt+Shift+T` (or `editor.trim_trailing` at every save) to take trailing
blanks off — each one undo step. A stream block that ends at column 0 of a line does not
include that line. Since 0.41.0, `Shift+F8` converts the end of line, the byte order mark and
the code page (§2.2i).
**Still to come**: the rope (today's buffer is a `Vec<String>` of lines — enough for the
256 MiB open limit and far simpler to keep correct; it will be swapped for a rope when
streaming edits of larger files land), persistent-block mode, multi-caret, replace inside a
block only, macros, the buffer list (§2.3) and the external editor (§2.4).

### 2.1 Buffer

- Rope (`ropey`-style) with line index, undo tree, and a dirty-region tracker for rendering.
- Opens files up to the configured limit (default 256 MiB) fully; above it, read-only "huge
  file" mode with chunked editing refused and an explanation.
- Encoding on open = viewer's detection; on save: keep original encoding and EOL by default
  (FAR's `KeepEditorEOL`), with explicit conversion commands (§2.2i). BOM preserved unless
  changed.
- Atomic save: write `name.fcmd-tmp` in the same directory, copy permissions/owner where
  possible, fsync, rename. Option for in-place save for special files (`/proc`, sockets,
  hardlink-sensitive paths).
- External-change detection: if the file changed on disk while open, a save offers
  overwrite / reload / diff / cancel (§2.2j). Keeping the buffer is *Cancel*, and save-as is
  `Shift+F2` from there rather than a fifth button.

### 2.2 Editing model

| Area | Behaviour |
| --- | --- |
| Blocks | stream blocks (`Shift+arrows`) **and column blocks** (`Alt+Shift+arrows`) with the FAR semantics: typing replaces the block optionally, `Alt+I`/`Alt+U` indent/outdent, persistent-block mode copies/moves a block to the cursor (`Ctrl+Shift+P`/`Ctrl+Shift+M`) |
| Multi-caret | derived from a column block or `Ctrl+D`-style "add next occurrence" (M3) |
| Undo | grouped by typing burst and by command; configurable depth; redo; undo survives save — §2.2g, which also says why the burst is bounded by what was done and not by a 300 ms clock |
| Clipboard | the system clipboard through the terminal (`OSC 52`, SPEC 06 §11) — a block copied or cut goes on it as well as on the editor's own, which is what `Ctrl+V` pastes from, because what the terminal holds cannot be read back; still owed: a ring of the last 10 blocks |
| Indentation | detect per file (spaces vs tabs, width), auto-indent, smart home, trailing-space trim on save (opt-in), `.editorconfig` support |
| Lines | duplicate, delete, move up/down, join, sort selected lines, unique, reverse, number |
| Search/replace | shared engine with the viewer, plus **replace with capture groups**, count, a list of every match, and "preserve style" (the capitals of the replaced token — FAR's feature, re-implemented from the described behaviour) — §2.2h. Still owed: replace-in-selection, and the separator half of preserve style (`foo_bar` → `foo-bar` conventions) |
| Bookmarks | 0..9 per file, persisted in `history/editor.toml` along with the cursor position |
| Syntax | shipped — see §2.5. Bracket matching and a rainbow option are still to come |
| Big edits | search/replace across a whole file streams through the rope without materialising copies |
| Macros | record/play keystroke macros, scoped to the editor, saved by name |

### 2.2c `.editorconfig`

The `.editorconfig` files from the file's folder up to the first with `root = true` are read
when a local file opens in the editor (`fcmd-edit::editorconfig`), the nearer file and the
later section winning, with the format's globs (`*`, `**`, `?`, `[…]`, `{a,b}`, `{1..3}`).
What they say wins over the settings for that file only:

* `indent_style` — spaces or tabs for what `Tab` types; `indent_size` / `tab_width` — the width.
* `trim_trailing_whitespace` — over `editor.trim_trailing`, at every save.
* `end_of_line` and `insert_final_newline` — for a new or empty file only. An existing file
  keeps the endings it has, so opening one never turns into a change to every line.
* `charset` is not applied: the code page is detected from the bytes (§2.1), and a file whose
  bytes disagree with its `.editorconfig` is shown as it is.

### 2.2d Completing a word, and the character list

* `Alt+/` (`editor.complete`): the word before the cursor — letters, digits, `_` — finished with
  a longer word of the buffer that starts with it, the nearest line first (this one, then one
  above, one below, outwards), each word once. Pressed again with the cursor where the last one
  left it, the next candidate replaces it; after the last, the prefix as typed comes back. One
  undo step each. No dictionary: the file is the vocabulary.
* `Alt+Shift+C` (`editor.charmap`): a list over the editor of Latin-1, punctuation, currencies,
  arrows, mathematics, box drawing and blocks, geometric shapes and Greek, each row the
  character and its `U+` code, filtered by typing either. `Enter` inserts at the cursor.

### 2.2e Taking out lines by pattern

`Alt+Shift+D` (`editor.delete_matching`): a regular expression, and every line of the block —
or of the file, with no block — that matches it goes; a leading `!` inverts it, keeping only
the matching lines (vim's `:g/re/d` and `:v/re/d`, FAR's issue #10). One undo step; a file
left with nothing is one empty line; the status line says how many went.

### 2.2f Replacements kept and recent

`Alt+Shift+F7` (`editor.replace.templates`, FAR's issue #282) lists the replacements kept by
name — `[[editor.replace]]` blocks with `name`, `find` and `with` — and then the last thirty
pairs `Ctrl+F7` ran, newest first, kept with the dialogs' field histories in the state file.
`Enter` runs one on the file as `Ctrl+F7` would; `Ins` on any row asks for a name and writes it
into the settings; `Del` on a kept one removes it.

### 2.2g Undo: bursts, commands and the depth

One `Ctrl+Z` takes back one *step*, and a step is one of two things.

- **A command**: everything one command changed, however many edits it is made of — a
  replace-all, a pasted column block, a sorted block, a deleted line.
- **A burst**: keys of one kind pressed one after the other. There are three kinds — a
  character typed (`Tab` and a `Ctrl+Q` literal included), `Backspace`, and `Del` — and a key
  joins the burst before it only when it is of the same kind *and* the cursor is exactly where
  that burst left it. So a word typed is one step; typing and then rubbing out are two;
  `Backspace` and then `Del` are two.

Anything else ends a burst: any cursor movement (even away and back), `Enter`, any command,
a search that moves the cursor, an undo or a redo, and typing over a block (the block's
removal is a step of its own). FAR groups the same way.

SPEC drafts said "300 ms": a pause ending a burst. It is not implemented, on purpose. The
event loop is input-driven and keeps no timer (see `AGENTS.md`), a clock would make undo
depend on how fast one types, and the boundaries above already fall where a pause usually
does. A burst is therefore bounded by what was done, never by when.

The mechanism lives in `fcmd-edit`: the editor decides whether a key *may* continue a burst
(its kind and the cursor), the buffer decides whether the burst is still there to continue —
any edit recorded outside a burst's own key closes it, so a command can never be folded into
the typing before it. Redo puts the cursor where the last key of the burst left it. Text
typed after an undo empties the redo list as soon as it is a step.

`editor.undo_depth` (default 1000, 1 to 100 000) is how many steps a buffer keeps; the oldest
are forgotten first. It is read when a file is opened and survives a reload from disk. Undo
survives a save.

### 2.2h Search and replace in the editor

`F7` (`editor.search`) is the viewer's dialog (§1.4) with a third button, **All matches**;
`Ctrl+F7` / `F8` (`editor.replace`) is a dialog of its own: *Search for*, *Replace with*,
*As* (text or regular expression), *Case sensitive*, *Whole words only*, *Preserve style*,
and the buttons **Replace all**, **Replace next**, **Cancel**. Both open on what was last
looked for and how; the how is kept per open file. A search works line by line: no hit spans
two lines.

- **Whole words**: a hit counts only if the character right before it and the one right after
  it are not word characters (a letter or digit of any script, or `_`). It is decided on the
  hit's neighbours, not by wrapping the pattern in `\b…\b`, which would demand a word
  character at each end of the hit and so never find `++` or `-x`. A hit refused for its
  neighbours is retried one character further on, so an overlapping one that stands alone is
  not lost.
- **Capture references**: when the search is a regular expression the replacement is expanded
  with the `regex` crate's syntax — `$1`, `${1}`, `${name}`, `$$` for a dollar; a reference to
  a group that does not exist is empty. In a plain search the replacement is literal, dollars
  included.
- **Preserve style** (plain searches only): the replacement takes the capitals of what it
  replaces. All lower case → lower-cased; all upper case (two letters or more) → upper-cased;
  first cased letter a capital and the rest lower, which includes a single capital letter →
  the replacement's first letter capitalised and the rest as typed; any other shape → as
  typed. FAR's own rule also carries separators over (`foo_bar` ↔ `foo-bar` ↔ `fooBar`);
  that half is not implemented.
- **Replace all** is one undo step and reports the count. **Replace next** replaces the hit
  that is selected — or finds the next one first — as one undo step, and leaves the following
  hit selected. A block marked by hand is replaced as such, with the text as typed.
- **All matches** opens a list with one row per matching line — the line number, right-aligned,
  and the line — on the first hit at or below the cursor. The list filters as every list
  does; `Enter` selects that line's first hit as a search would, so `Shift+F7` / `Alt+F7`
  continue from it; `Esc` returns to the editor untouched. No hit at all is the same
  "not found" message as a failed search.
- An empty match (`^`, `$`, `x*`) can be replaced; it is not offered again right after a
  non-empty hit, the rule the regex engine itself iterates by.

The replacements of §2.2f run with the options the dialog was last set to.

### 2.2i Conversions: end of line, byte order mark, code page (`Shift+F8`)

A save writes what was read (§2.1). `Shift+F8` (`editor.convert`) is the one place that
changes it: a list of three groups separated by rules — *End of line* LF / CRLF / CR, *Byte
order mark* written / not written, and one row per code page, favourites first, in the order
of §1.12 — with the row that describes the file now marked in each group and the cursor on the
first of them. The list filters as every list does. `Enter` converts and returns to the
editor; `Esc` returns with nothing changed.

- **A conversion is an edit that touches no line.** It changes what the next save writes,
  marks the buffer modified, and is one undo step of its own — never joined to the typing
  before or after it — recorded with the whole of how the file was written before, so `Ctrl+Z`
  gives back end of line, mark and page together. Nothing is written until `F2`. A read-only
  buffer refuses it like any other edit; a row that is already true says so and records
  nothing.
- **End of line**: every line gets the chosen terminator. A file with mixed endings has no
  row marked, and any of the three is a change — including the majority one, which the status
  line was already showing.
- **Byte order mark**: only UTF-8, UTF-16LE and UTF-16BE have one; asking for it in any other
  page is refused with a message.
- **Code page**: the text in memory is Unicode and is not touched; the page is what it is
  encoded in at the next save. A page that cannot hold every character of the text
  (`Codepage::can_encode`) is refused and nothing changes — the save never writes a `?` for a
  character it could not encode. The mark follows the page: UTF-16 gets one, UTF-8 keeps what
  it had, any other page loses it.
- The editor's status line names the result: `LF`/`CRLF`/`CR`, the page, and ` BOM` when a
  mark is written.

This is not the viewer's `Shift+F8` (§1.12), which decodes the same bytes as another page.
Reading an open file again in another page is not implemented in the editor.

### 2.2j A file changed underneath

The editor remembers the modification time of the file it opened or last wrote. `F2` on a
modified buffer whose file now has another time does not write: it asks *Changed on disk*,
with four buttons in this order — **Overwrite**, **Reload**, **What changed**, **Cancel** —
the first lit. `←`/`→`/`Tab`/`Shift+Tab` go round them, `Enter` takes the lit one, `Y` is
*Overwrite*, and `Esc`, `N` or `F10` is *Cancel*.

- **Overwrite** saves over the file as it is now.
- **Reload** is `editor.reload` (`Ctrl+R`): the file is read again, the unsaved edits and the
  undo history go, the cursor stays where it was if that line still exists.
- **What changed** opens the diff screen of §3 on two texts: on the left the file on disk,
  read as a reload would read it (BOM, detection, §1.11) and titled `name · on disk`; on the
  right the buffer as it stands, unsaved edits included, titled `name · in the editor`. Line
  endings are compared or ignored as the comparison dialog was last told, and `E` toggles it.
  The screen only shows: `F5` (merge), `F2` (write) and `F12` (screens) are refused with a
  message, since a merge would have to change either the buffer behind its undo history or a
  file somebody else has just written. `Esc`, `F10` or `F3` closes it and **asks the same
  question again**, on its first button. Either side larger than the diff's limit (§3), or a
  file that can no longer be read, says so and leaves the question open.
- **Cancel** returns to the editor with the buffer untouched and still modified.

A fetched copy of a remote file (SPEC 04) asks its own question when the server's
copy changed; that one still has two buttons.

### 2.2b Syntax highlighting

`fcmd-syntax` is a scanner, not a parser, and deliberately: it knows how a language writes a
comment, a string and a number, plus the words anyone would point at and call keywords. That
colours real files correctly and fits on a page, where a grammar per language would be a
compiler front end and a build-time dependency per language on top.

The unit is a line and the state carried from the one before it — a block comment, an
unterminated string, a Markdown fence. That is what makes both callers possible:

- The **editor** keeps the state each line starts in and extends it as the screen asks for
  lines further down, throwing it away from the first line an edit touched. Closing a comment
  uncolours the lines below it in the same frame, and a file of a million lines costs the
  scan of what is on screen plus whatever is above it, once.
- The **viewer** reads back at most a megabyte above the first row on screen to know what that
  row starts inside, and remembers where it stopped, so scrolling down reads each line once.
  A block comment opened within that megabyte — a form's `{ … }` of binary data, a licence in
  `/* … */` — keeps colouring the rows under it after its first line has scrolled off; one
  opened further up is not seen, which is the price of opening a two-gigabyte log instantly.
- **Block guides.** For Pascal and the forms written in it, the viewer draws a thin line down
  each open block, as the Delphi IDE does: from the line that opens it (`begin`, `try`,
  `case`, `repeat`, `asm`, a `class` or `record` declared with a body; `object`, `inherited`,
  `inline` and `item` in a form) to its `end` or `until`, in the column where that line starts,
  one per nested block, only through blank cells and never in the text that is copied. The
  colour is `syntax.guide`; wrapped lines have none, a row there not being a line.
  The **editor** draws the same guides in its text area, for the same two syntaxes and under
  the same `ui.syntax` switch, from the same reading of the words (`fcmd_view::guides`). It
  works them out for the lines on screen only, reading back above the first of them as far
  as the viewer does (256 KiB of text) for the blocks already open, and starting the scanner
  in the state the colouring has for that line, so a comment opened further up still hides
  its words. The answer is kept until an edit touches a line in the window or above it — an
  edit below the window, a cursor movement or a redraw costs nothing — and a change of tab
  width or of the window asks again. Columns are display columns with tabs expanded, shifted
  by the horizontal scroll; a guide whose column has scrolled off is not drawn. A marked
  block is drawn over the guides, and with *Show spaces and tabs* on a guide takes the place
  of the `·` in its cell.

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

### 2.5b Search and replace in hex mode

The keys are the text editor's, because they are the same commands (`editor.search`,
`editor.search.next`, `editor.search.prev`, `editor.replace`): `F7` searches, `Shift+F7` and
`Alt+F7` repeat the last search forwards and backwards, `Ctrl+F7` or `F8` replaces.

| Thing | Behaviour |
| --- | --- |
| The query | the dialog of §1.4: **text**, encoded through the buffer's code page (UTF-8 unless set otherwise — a binary does not say what its strings are written in, and UTF-8 is ASCII for everything ASCII), with the case-sensitive box; **hex** bytes with spaces anywhere and `??` for any byte (`DE AD ?? EF`); or a **regular expression** over the bytes |
| Where it starts | forwards at the cursor, or one byte past it when the cursor is already on a match; backwards, a match has to end at the cursor or before it |
| What is found | becomes the block, with the cursor on its first byte and the row scrolled into view |
| The ends of the file | a search that runs off one end starts again from the other, and the status line says that it did |
| Large files | the bytes are read through the piece table a window of 256 KiB at a time, the windows overlapping by at least the pattern's length less one, so a match lying across the edge of two is found and the file is never loaded whole. The search runs where the key was pressed, as the viewer's does (§1.4) |
| The replacement | typed the way the query was: hex digits when the search is in hex (no wildcards; an empty line deletes what was found), text through the code page otherwise. A regular expression's replacement is literal — no capture references |
| Lengths | a replacement as long as the match overwrites it. Any other length moves everything after it, so it is done only in insert mode (`Ins`); in overwrite mode it is refused with the two lengths, and a replace-all in which any one match is of another length is refused whole |
| One at a time | *Replace* in the dialog goes to each match from the cursor on and asks: **Replace**, **All** (this one and every one after it), **Skip**, **Cancel**. The question is drawn over the bytes, with the match marked behind it. It does not come round from the start |
| All at once | *All* in the dialog replaces every match in the file and says how many |
| Undo | each confirmed replacement is one step; a replace-all is **one** step however many matches it had. All of its matches point at the same stretch of typed bytes, so a hundred thousand replacements cost the bytes of one |
| Read-only | a read-only buffer and a device are searched, and refuse the replacement by name |

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
assembled as 32-bit code on a 64-bit machine, and an architecture this cannot write (AArch64,
which §1.6 *reads*) is refused by name rather than assembled as x86 — and what comes back names the bytes, the
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
| an address | `mov eax, [rbx]`, `mov [rbx+8], rax`, `add eax, [rbx+rcx*4+8]`, `mov eax, [0x401000]`, `lea rdi, [rip+0x100]` |
| an address and a number | `mov dword [rbx], 1`, `cmp byte ptr [rsi], 0x2e`, `and qword [rsp+8], 0x0f` |
| an address through the stack or a branch | `push qword [rbx]`, `pop qword [rbx]`, `jmp qword [rbx]`, `call qword [rip+0x20]` |

A number is whatever the calculator of §2.9 reads: `0x2a`, `2Ah`, `0b101010`, `42`, `0x10+6`.
A branch is written as the address to arrive at, not as a displacement, and the shortest form
that reaches it is chosen unless `short` or `near` says otherwise.

**Addresses** are written the way a disassembly prints them, because retyping a line one has
just read is the whole job: `[base]`, `[base+disp]`, `[base+index*scale+disp]`, a bare `[disp]`,
and `[rip+disp]` with the displacement counted from the end of the instruction. The pieces may
be written in any order (`[8+rbx]` is `[rbx+8]`), a scale belongs to a register and only there,
and every number inside the brackets goes through the calculator of §2.9.

How wide the thing at an address is has to be known before anything is written, and there are
only two ways to know: the register on the other side of the instruction says so, or a size word
in front of the brackets does — `byte`, `word`, `dword`, `qword`, with masm's optional `ptr`.
When neither says, the instruction is refused with that sentence rather than guessed at: `mov
[rbx], 1` could write one byte or eight, and the difference is somebody's binary. A size word
that contradicts the register is refused too.

What is still **not** assembled: segment prefixes (`fs:[0x30]`), a 32-bit address inside 64-bit
code (the `0x67` prefix), far pointers, and the floating-point and vector instruction sets. Each
is refused by name rather than guessed at, and the hex pane is still there for bytes nobody can
spell. This is not a compiler and does not pretend to be one.

The encoding rules that are easy to get wrong are the ones the tests pin down by their bytes:
`rsp` and `r12` as a base need a SIB byte, `rbp` and `r13` as a base need a displacement even
when it is zero, an index with no base carries a 32-bit displacement whether one wrote it or
not, `rsp` cannot be an index at all, and `push`, `pop` and an indirect branch move a machine
word without a `REX.W` to say so. `[rip+disp]` is the one form that does not round-trip as
text, because the disassembler resolves it to the address it lands on — which is what one wants
to read; the bytes are checked directly instead.

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

## 3. Diff: two files side by side (`Ctrl+Shift+D`)

Two files, two columns, and what differs marked — the screen a reader of two versions of the
same file wants, and the one gap a commander coming from Midnight Commander or Krusader notices
at once. `Compare by content` (§SPEC 03 §11) answers *which of these hundred pairs differ*; this
answers *and what, in this pair, is different*, which is a different question and needs a screen
rather than a selection.

**Which two files.** The one under each panel's cursor. Not a dialog: the question is already
answered by where the cursors are, which is how the comparison commands beside it in the same
menu work too. A cursor on `..` or on a directory says so instead of opening an empty screen.

**What it will not read.** A file on a server or inside an archive says so by name rather than
fetching a copy behind the reader's back: the whole of both files has to be in memory to line
them up, and a silent download of two hundred megabytes over SFTP is not what `Ctrl+Shift+D`
looks like it will do. The limit is 16 MiB a side (`fcmd_view::diff::MOST`) and a file over it
is refused with its size, because the alignment below is quadratic in the worst case and a
screen that hangs is worse than one that declines.

**The algorithm** is in `fcmd-core::diff`, with no dependency: patience diff — the lines that
occur exactly once on each side are the anchors, the longest rising sequence of them is kept,
and each region between two anchors is solved the same way again. A region with no unique line
left and at most 400 lines (`EXACT`) is finished with a proper longest-common-subsequence
table, which is where the quadratic cost is bounded; a larger anchorless region is reported as
one removal and one addition, which is what it is. Patience rather than Myers because the
answer is the one a reader expects — a moved function reads as moved, not as a thousand
alternating lines — and because the whole of it is a hundred lines one can read and verify.
Inside a changed pair of lines, `diff::inside` narrows the difference to the part that actually
changed, on character boundaries, and that part is drawn in the selection's colours: the word
diff is the eye's shortcut to the one character that matters.

**What is ignored.** Three questions, each a keystroke and each lining the files up again at
once, so one watches the answer change: `W` blanks, `C` case, `E` line endings. Line endings
start from `[compare] ignore_eol`, so the screen and the folder comparison agree about what
"the same" means; blanks and case start off, because they are a question one asks of the diff
in front of one rather than a setting one keeps.

**The screen.** Two equal columns with a rule between them, the left the *before* and the right
the *after*. Each side has five columns of line number, then the sign a unified diff would give
the line — `-` on the left of a removed or changed row, `+` on the right of an added or changed
one — then the text; a line that exists on one side only has a number on that side and nothing
on the other. The colours are the diff's own (`syntax.removed` and `syntax.added`, 0.39.0): red
on the left side of a row that differs, green on the right, the changed part in the selection's
colours on top. The foot line says which hunk of how many is under the cursor, the lines
removed and added (a changed row counts one of each), and the keys. `Alt+↓` and
`Alt+↑` are the hunks, which is the key one presses forty times; the plain arrows, `PgUp`,
`PgDn`, `Home` and `End` are the rows, `←` and `→` scroll both columns sideways together, and
`Tab` changes which column is the one being read. `F1` opens the written chapter, `Esc`, `F3`
and `F10` close. Two files that are the same open all the same — one wants to see that they
are — and the screen says so rather than leaving the reader to scroll for a difference that is
not there.

**Taking a difference across.** `F5`, which is Midnight Commander's merge key: the focused
column takes the other one's version of the hunk at the top of the screen, so `Tab` chooses
which way it goes and `Alt+↓` chooses which hunk. Whole lines, because a hunk is whole lines
— a row only the other side has is inserted, a row only this side has goes, a row both have
with different text is replaced — and the incoming lines are re-ended to match the file they
are going into, so a Unix line dropped into a DOS file does not become the one bare newline in
it. A hunk one side has no lines in still has a place to go: where it goes is counted from the
rows before it rather than read off a line number it has not got. On a row both files agree
about, `F5` says there is no difference here rather than taking the next one by surprise.

Nothing reaches a disk until `F2`. The side that was merged into is marked with a `*` beside
its name, `F2` writes the marked side or sides — beside the original and renamed over it, as
everything else in the program replaces a file — and a side that was written is no longer
marked, so a second `F2` has nothing to do and says so. A side that refused keeps its mark.
`Esc` with something unwritten says so and takes the next `Esc` as the answer: this screen has
no dialog of its own, and a warning one can walk past deliberately is better than a question
one has to answer twice an hour.

**It is a screen**, so `F12` lists it with the viewers and editors and one steps away from it
and back.

Still to come, and deliberately not in the first cut: a unified view, which is a second layout
of the same rows; a diff of a file against the copy on disk, which is the editor's question;
and a byte comparison of two binaries with the first differing offset and hex context, for
which the pair is simply too large to line up as lines.
