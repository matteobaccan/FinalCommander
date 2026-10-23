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
│docs/                 FOLDER          ││sc-build/             FOLDER          │
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
  rename, not a copy. A copy is written to `name.sc-part` and renamed only when complete, so an
  interrupted operation never leaves a file that looks finished.
- **Panels**: `Tab` / `Ctrl+U`, arrows / `PgUp` / `PgDn` / `Home` / `End` / `Enter` /
  `Ctrl+PgUp` / `Backspace` / `Ctrl+\` with history (`Alt+←/→`), `Ins` and `+` / `-` / `*` /
  `Ctrl+A` selection by mask, `Ctrl+F3..F7` sorting (press twice to reverse), `Ctrl+1/2` view
  modes, `Ctrl+H` hidden files, `Alt+I` live filter, quick search by typing, `Space` for a
  recursive directory size, `Ctrl+=` to sync the other panel.
- **The rest**: `Alt+T` cycles themes, `F1` is a help page generated from the live keymap,
  `F10` quits.

Not yet: `F3` view and `F4` edit (the viewer and the editor are the next milestone), trash,
wipe, a background job queue, archives and network panels. Anything unimplemented says so
plainly instead of pretending.

Non-interactive modes (used by CI and by scripts):

```sh
sc --list-themes      # built-in and external themes, with where each came from
sc --list-commands    # the command vocabulary every key, menu and plugin binds to
sc --list-languages   # en it de fr es ru zh
sc --check-keymap     # validate a keymap and print the resolved bindings
sc --dump-config      # the effective configuration
sc --export-theme tron >> ~/.config/sc.toml   # a ready-to-paste [[theme]] block
sc --paths            # where the config file, the state file and plugins/ are
sc --list-plugins     # what is in the plugins directory
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

Everything on disk is optional and amounts to: one settings file (`sc.toml`, which also holds
your themes and keymaps), one state file beside it, and one `plugins/` directory — the only
directory the program ever reads.

There is no GUI build and there will not be one: the graphical commanders are a crowded field,
and the open slot is a terminal commander with a modern engine.

## Themes and languages

Six themes are compiled in — `commander` (the classic blue, default), `black`, `white`,
`matrix`, `tron`, `blade` — and `[[theme]]` blocks in `sc.toml` are read at startup,
hot-reloaded, may `inherit` a built-in one, and replace a built-in with the same id.

The interface ships in English, Italian, German, French, Spanish, Russian and Simplified
Chinese (`ui.language = "auto"` follows the OS locale); CJK double-width layout is tested.

## Layout

```
crates/sc-core     paths, entries, listings, sorting, selection   (no dependencies of ours)
crates/sc-vfs      the Vfs trait + the local provider
crates/sc-jobs     the operation engine: plan, execute, settle; conflicts and progress
crates/sc-cmd      command vocabulary, keys, keymap profiles (far, mc)
crates/sc-config   TOML configuration, discovery, overrides
crates/sc-i18n     fluent catalogues for seven languages
crates/sc-ui       cell buffer, colours, themes, panel and chrome widgets
crates/sc-tui      the terminal backend: input decoding and diffed painting
crates/sc          the binary: CLI, state machine, event loop
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
