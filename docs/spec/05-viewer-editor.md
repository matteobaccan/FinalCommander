# SPEC 05 — Viewer and editor

## 1. Viewer (`F3`)

### 1.1 Modes

| Mode | Purpose | Notes |
| --- | --- | --- |
| `text` | decoded text, wrap off/char/word | arbitrary line length, tabs expanded per config |
| `dump` | raw bytes as characters, fixed width | for binaries where text decoding lies |
| `hex` | offset + hex + ASCII/decoded pane | bytes-per-row adjustable (`Alt+←/→`), 16-aligned jumps |
| `preview` | rendered content (image, markdown, PDF page, media info, archive listing) | pluggable, see §1.6 |

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

### 1.5 Navigation extras

Go to offset / line / percent (`Alt+F8`, `:`), bookmarks 0..9 (set `Ctrl+Shift+N`, go
`Alt+N`), position undo (`Ctrl+Z`), next/previous file in the panel (`+`/`-`) keeping mode and
encoding, "jump to this file in the panel" (`Ctrl+F10`), copy selection (`Ctrl+C`) including
hex-mode byte ranges.

### 1.6 Preview pipeline

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

### 2.1 Buffer

- Rope (`ropey`-style) with line index, undo tree, and a dirty-region tracker for rendering.
- Opens files up to the configured limit (default 256 MiB) fully; above it, read-only "huge
  file" mode with chunked editing refused and an explanation.
- Encoding on open = viewer's detection; on save: keep original encoding and EOL by default
  (FAR's `KeepEditorEOL`), with explicit conversion commands. BOM preserved unless changed.
- Atomic save: write `name.sc-tmp` in the same directory, copy permissions/owner where
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
| Clipboard | OS clipboard via `sc-platform` (X11/Wayland/macOS/Windows, OSC 52 fallback over SSH); internal ring of the last 10 blocks; append-to-clipboard (`Ctrl+Shift+C`) |
| Indentation | detect per file (spaces vs tabs, width), auto-indent, smart home, trailing-space trim on save (opt-in), `.editorconfig` support |
| Lines | duplicate, delete, move up/down, join, sort selected lines, unique, reverse, number |
| Search/replace | shared engine with the viewer, plus **replace with capture groups**, replace-in-selection, count, and "preserve style" (match the case/separator convention of the replaced token — FAR's feature, re-implemented from the described behaviour) |
| Bookmarks | 0..9 per file, persisted in `history/editor.toml` along with the cursor position |
| Syntax | tree-sitter grammars loaded from `grammars/` (prebuilt, feature-gated), fallback to a regex-based highlighter; bracket matching, rainbow option |
| Big edits | search/replace across a whole file streams through the rope without materialising copies |
| Macros | record/play keystroke macros, scoped to the editor, saved by name |

### 2.3 Multiple files

Buffer list (`F12`), next/previous buffer, "open all selected files from the panel", recent
files with positions, and a modified-indicator in the title. Closing with unsaved changes asks
per file with a "save all" action.

### 2.4 External editor

`$VISUAL`/`$EDITOR` or a per-mask association (`assoc.toml`) can replace the internal editor
for chosen types; the panel waits for the process and refreshes afterwards. `Alt+F4` always
flips internal↔external (FAR's rule).

## 3. Diff viewer (M3)

Side-by-side and unified views of two files (or the two panels' cursor items, or a file against
its on-disk version), with word-level intra-line diff, synchronised scrolling, next/previous
change, copy-block-to-other-side (producing a normal job), ignore-whitespace/case/EOL options,
and binary comparison (first differing offset, hex context). Backed by a Myers/patience diff
implementation in `sc-core`.
