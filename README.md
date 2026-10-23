# SupremeCommander

A cross-platform **orthodox file manager** (two panels, command line, function keys,
keyboard-first) written in Rust — in the tradition of Norton Commander, FAR Manager,
Total Commander, Midnight Commander and Krusader, with a modern engine underneath.

> **Status: milestone M1 in progress.** The specs are written and the skeleton runs: two
> panels, navigation, selection, sorting, view modes, quick search and filter, themes,
> keymap profiles and seven interface languages. File operations, the viewer and the editor
> are the next chunks — see `docs/spec/09-roadmap.md`.

## Try it

```sh
cargo run --release -- /etc /tmp     # left panel, right panel
```

```
┌────────── /home/mb/project ──────────┐┌──────────────── /tmp ────────────────┐
│..                      UP--          ││..                      UP--          │
│src/                  FOLDER          ││cargo-installXXXXXX/  FOLDER          │
│docs/                 FOLDER          ││scmd-build/             FOLDER          │
│Cargo.toml             412 B 2026-10-2││notes.md              1.2 K 2026-10-2│
│README.md             2.00 K 2026-10-2││                                      │
└─2 files, 2.40 K  [name]──────────────┘└─1 files, 1.20 K  [name]──────────────┘
 Cargo.toml  412 B  2026-10-23 17:42
1Help   2Menu   3View   4Edit   5Copy   6RenMov 7MkDir  8Delete 9PullDn 10Quit
```

What works today:

- **File operations**: `F5` copy, `F6` move/rename, `F7` create directory, `F8` delete — on the
  selection or on the item under the cursor, with the destination prefilled from the passive
  panel. Recursive trees, conflict dialogs (overwrite / all / skip / skip all / cancel), error
  decision points (retry / skip / skip all / cancel), a progress overlay with a per-file and a
  total bar, `Esc` to cancel, and both panels refreshed afterwards. A same-filesystem move is a
  rename, not a copy. A copy is written to `name.scmd-part` and renamed only when complete, so an
  interrupted operation never leaves a file that looks finished.
- **Panels**: `Tab` / `Ctrl+U`, arrows / `PgUp` / `PgDn` / `Home` / `End` / `Enter` /
  `Ctrl+PgUp` / `Backspace` / `Ctrl+\` with history (`Alt+←/→`), `Ins` and `+` / `-` / `*` /
  `Ctrl+A` selection by mask, `Ctrl+F3..F7` sorting (press twice to reverse), `Ctrl+1/2` view
  modes, `Ctrl+H` hidden files, `Alt+I` live filter, quick search by typing, `Space` for a
  recursive directory size, `Ctrl+=` to sync the other panel.
- **Viewer** (`F3`): text, dump and hex on files of any size — position is a byte offset, so
  opening a 40 GB log costs one read and `End` costs one seek. Encoding detected (UTF-8,
  UTF-16 with or without a BOM, CP1252, CP437) and switchable with `F8`; `F2` cycles
  no-wrap / character / word wrap; `F4` hex, `Shift+F4` the next mode; `F7` search (plain,
  regex or `4D 5A ?? 00` hex patterns, hits highlighted, `n`/`N` for the next and previous);
  `Alt+F8` jumps to an offset, `0x`-offset or `NN%`; `Shift+F` follows a growing file like
  `tail -f`; `Ctrl+Z` undoes a jump. `hjkl`, `g`/`G`, `/` and `:` work too.
- **Inside compiled files**: `F5` shows the code, whatever the file is. Native binaries (ELF,
  PE, Mach-O, raw) are disassembled with sections and symbols read via `object` and x86/x86-64
  decoded with `iced-x86`, with `<symbol>:` headings, scrolling by whole instructions, and a
  raw sweep (announced in the status line) for files over the 64 MiB parse limit, because a
  debug build is routinely 100 MB. **Bytecode is read in process**: a `.class` file becomes a
  `javap`-style listing with the constant pool resolved into the comments, a managed PE becomes
  an `ildasm`-style IL listing with metadata-resolved tokens and `ldstr` literals, and a
  `.wasm` becomes its text format. `F6` decompiles:
  WebAssembly becomes its text format in process, and anything else is handed to an installed
  tool through a `[[decompile]]` rule — `javap`, `ilspycmd`, `python -m dis`, `objdump`,
  `retdec` — whose output is then viewable, searchable text. A real decompiler is a research
  project: we call the good ones instead of shipping a bad one.
- **The rest**: `Alt+T` cycles themes, `F1` is a help page generated from the live keymap,
  `F10` quits.

Not yet: `F4` edit (the editor is next), trash, wipe, a background job queue, archives and
network panels. Anything unimplemented says so plainly instead of pretending.

Non-interactive modes (used by CI and by scripts):

```sh
scmd --list-themes      # built-in and external themes, with where each came from
scmd --list-commands    # the command vocabulary every key, menu and plugin binds to
scmd --list-languages   # en it de fr es ru zh
scmd --check-keymap     # validate a keymap and print the resolved bindings
scmd --dump-config      # the effective configuration
scmd --export-theme tron >> ~/.config/scmd.toml   # a ready-to-paste [[theme]] block
scmd --paths            # where the config file, the state file and plugins/ are
scmd --list-plugins     # what is in the plugins directory
```

## Why

The best commanders are either Windows-only (FAR Manager, Total Commander), GPL forks of
30-year-old C++ (far2l, Midnight Commander), or tied to a desktop toolkit (Krusader,
Double Commander). Nothing in Rust covers the full orthodox feature set: the existing Rust
file managers are single-pane browsers (yazi, joshuto, xplr) or very young GTK4 apps.

Goal: **one executable** — themes, keymaps, all seven languages and the help compiled in,
nothing to install and no asset tree — the same keys on Linux, macOS and Windows, a
**strictly TUI** interface that works over SSH and in tmux, an operation engine that never
blocks or loses data, and a sandboxed plugin ABI whose first three clients are the viewer, the
editor and netbox (the network panel).

Everything on disk is optional and amounts to: one settings file (`scmd.toml`, which also holds
your themes and keymaps), one state file beside it, and one `plugins/` directory — the only
directory the program ever reads.

There is no GUI build and there will not be one: the graphical commanders are a crowded field,
and the open slot is a terminal commander with a modern engine.

## Why the command is `scmd` and not `sc`

`sc` is taken three times over: on Windows `sc.exe` in `System32` is the Service Control
Manager, on Unix `sc` is the classic spreadsheet calculator, and the crates.io name belongs to
a syscall crate. A commander that promises the same keys on three platforms cannot fight a
system command for its own name, so the binary, the config file and the crates are all
`scmd`. Please do not "fix" it back.

## Themes and languages

Six themes are compiled in — `commander` (the classic blue, default), `black`, `white`,
`matrix`, `tron`, `blade` — and `[[theme]]` blocks in `scmd.toml` are read at startup,
hot-reloaded, may `inherit` a built-in one, and replace a built-in with the same id.

The interface ships in English, Italian, German, French, Spanish, Russian and Simplified
Chinese (`ui.language = "auto"` follows the OS locale); CJK double-width layout is tested.

## Layout

```
crates/scmd-core     paths, entries, listings, sorting, selection   (no dependencies of ours)
crates/scmd-vfs      the Vfs trait + the local provider
crates/scmd-jobs     the operation engine: plan, execute, settle; conflicts and progress
crates/scmd-view     the viewer: windowed reads, encodings, text/dump/hex, search
crates/scmd-cmd      command vocabulary, keys, keymap profiles (far, mc)
crates/scmd-config   TOML configuration, discovery, overrides
crates/scmd-i18n     fluent catalogues for seven languages
crates/scmd-ui       cell buffer, colours, themes, panel and chrome widgets
crates/scmd-tui      the terminal backend: input decoding and diffed painting
crates/scmd        the binary: CLI, state machine, event loop
```

## Documents

| Document | Content |
| --- | --- |
| [`docs/00-landscape.md`](docs/00-landscape.md) | survey of ~50 commanders: language, licence, what we take from each |
| [`docs/01-licensing.md`](docs/01-licensing.md) | **can we rewrite FAR? yes** — licence analysis (FAR is BSD-3-Clause), what we may and may not reuse, clean-room rules |
| [`docs/02-features.md`](docs/02-features.md) | the complete feature inventory (11 areas, ~130 features) with target milestones |
| [`docs/03-keymap.md`](docs/03-keymap.md) | every key of FAR / mc / Total Commander / Krusader side by side, our default keymap, the command vocabulary, and the terminal limitations that shape it |
| [`docs/spec/00-overview.md`](docs/spec/00-overview.md) | goals, non-goals, interaction contract, quality gates |
| [`docs/spec/01-architecture.md`](docs/spec/01-architecture.md) | crate layout, threading, rendering, paths, errors, testing |
| [`docs/spec/02-panels.md`](docs/spec/02-panels.md) | panel state, listing, view modes, sorting, filters, tabs, layout |
| [`docs/spec/03-file-operations.md`](docs/spec/03-file-operations.md) | job engine, conflicts, copy semantics, queue, multi-rename, compare/sync |
| [`docs/spec/04-vfs.md`](docs/spec/04-vfs.md) | the `Vfs` trait, capabilities, providers (archives, SFTP, S3, …), nesting |
| [`docs/spec/05-viewer-editor.md`](docs/spec/05-viewer-editor.md) | viewer (text/hex/dump/preview), editor (column blocks, undo, syntax), diff |
| [`docs/spec/06-tools.md`](docs/spec/06-tools.md) | find, grep, disk usage, user menu, associations, shell/PTY integration, help |
| [`docs/spec/07-config-theming.md`](docs/spec/07-config-theming.md) | TOML config, keymap profiles, themes, i18n, config import from other commanders |
| [`docs/spec/08-plugins.md`](docs/spec/08-plugins.md) | command bus, Lua API, WASM plugin ABI (WIT), native ABI, macros |
| [`docs/spec/09-roadmap.md`](docs/spec/09-roadmap.md) | M1→M4 with issue-sized chunks, risks, contribution rules |

## Licence question, short version

- **FAR Manager is BSD-3-Clause** → we may study it, port parts of it (with its notice), and
  build a permissively licensed Rust commander on its model. The only restriction that bites is
  the no-endorsement clause: we must not use the authors' names to promote this project.
- **far2l, Midnight Commander, Krusader, Double Commander are GPL** → behavioural reference
  only, never copy or translate their code into this project.
- **Total Commander is proprietary** → no code, no resources, no decompiling. Its *key layout
  and feature set* are ideas and interfaces, not protected expression (CJEU C-406/10,
  Lotus v Borland, Google v Oracle), so a `tc` keymap profile is fine.

**This project is MIT licensed** (see [`LICENSE`](LICENSE)); details and the inbound-compatibility
rules in [`docs/01-licensing.md`](docs/01-licensing.md).

## Non-goals

No telemetry, no accounts, no cloud service, no bundled proprietary blobs, no GTK/Qt dependency
in the core, and no attempt to be an Explorer/Finder replacement.

---

Documents are written in English so the project is contributable; discussion happens in
English and Italian.
