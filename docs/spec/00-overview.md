# SPEC 00 — Overview, goals, non-goals

## 1. What we are building

**FinalCommander** (`fcmd`) is a cross-platform orthodox file manager written in Rust:
two panels, a command line, a function-key bar, keyboard-first, with an internal viewer and
editor, a virtual filesystem layer (archives, SFTP, S3, …), a job engine for file operations,
and an extension layer (Lua + WebAssembly).

The interface is **strictly TUI**: a character grid in a terminal, local or over SSH, inside
tmux or a bare console. There is no GUI build, no GPU canvas, no desktop-toolkit backend — not
as an option, not later. Anything that cannot be expressed in cells, colours and keystrokes is
out of scope, and the few places where terminals do offer more (inline image protocols,
kitty keyboard reporting, OSC 52 clipboard) are used as *terminal* features behind capability
detection, never as a second UI.

Model: FAR Manager for the interaction grammar and plugin ambition, Total Commander and
Krusader for the tool set, Midnight Commander for Unix integration, yazi for the async engine
and preview pipeline. Licensing basis: `docs/01-licensing.md`.

## 2. Goals

- **G1 — Same muscle memory everywhere.** One binary, identical keys and behaviour on Linux,
  macOS and Windows. Keymap profiles (`far`, `mc`, `tc`, `nc`, `vi`) for people arriving from
  another commander.
- **G2 — Never block the UI.** Directory listing, sizing, hashing, previews, copying: all off
  the UI thread. 100k-entry directories scroll at full speed; a stalled network mount never
  freezes the panels.
- **G3 — Operations you can trust.** Explicit conflict handling, verified copies, preserved
  metadata, a visible job queue, resumable/cancellable work, and no silent data loss.
- **G4 — Scriptable and extensible.** Every action is a named command; keys, menus, macros,
  Lua and plugins all drive the same vocabulary. A sandboxed WASM ABI so plugins outlive
  internal refactors.
- **G5 — Honest and local.** No telemetry, no accounts, no background network traffic.
  Config is human-readable TOML in a documented location; portable mode supported.
- **G6 — One file.** The product is a **single executable**. Every resource it needs is
  compiled in: themes, keymap profiles, all seven language catalogues, the help. It runs with
  an empty disk around it — no installer, no asset directory, no data files to deploy, nothing
  to break when a path moves.
  The only directory FinalCommander ever looks for is **`plugins/`**, and it is optional.
  User settings, when there are any, are **one file** (`fcmd.toml`) plus **one state file**
  (`fcmd-state.toml`) beside it — themes and keymaps are blocks inside that file, not a tree.
  Budget: < 20 MB binary, cold start < 50 ms, no runtime dependencies for the core features.

## 3. Non-goals

- Not an Explorer/Finder replacement with a single-pane shell-extension UX.
- No cloud sync service, no accounts, no "AI assistant" in the panel.
- No bundled proprietary blobs; no feature that requires a non-free component to work
  (RAR *write*, for example, is simply out of scope).
- **No installer and no asset tree.** Nothing is read from `/usr/share`, no theme or language
  files are deployed next to the binary, and a missing config file is a normal state, not a
  degraded one. A release is one executable (plus, if you want them, plugins in one folder).
- **No GUI.** No GTK/Qt/GPU backend, no "windowed mode", no web front end. The TUI is the
  product; a graphical file manager is a different program.
- Not a terminal multiplexer, though it embeds PTYs.

## 4. Users we design for

1. **The FAR/TC veteran on a new OS.** Wants the keys to work and the archive to open as a
   folder. Success = never reaches for the mouse, never reads the manual.
2. **The sysadmin over SSH.** Wants a working commander inside a 80x24 tmux pane on a jump
   host, with SFTP panels and a real shell underneath.
3. **The developer.** Wants fast navigation in huge repos, grep with jump-to-match, git-aware
   colouring, an editor that is good enough for a quick fix, and a scriptable interface.
4. **The migrant from mc.** Wants `mc` keys, subshell behaviour and `Ctrl+O`.

## 5. Interaction contract (non-negotiable)

1. The function-key bar always shows what `F1..F12` do *right now*, including modifier rows —
   **all twelve of them, in twelve equal boxes**, with the ones this screen or this modifier
   has no use for drawn empty. The bar is read sideways: `F5` is the fifth box along wherever
   one is in the program, and a bar that left out its unused keys would put `F7` where `F5`
   had been, which is how a reader learns to distrust it.
2. Every destructive action asks once, with the exact count and target shown, and has a
   `--yes`-equivalent setting for people who hate it.
3. Any operation longer than 300 ms gets a progress UI; any operation longer than 2 s is
   movable to the background without losing state.
4. `Esc` always backs out. `Ctrl+C` never kills the app while a job runs; it cancels the job.
5. The panel under the cursor never lies: after an operation the affected panels are refreshed
   and the cursor stays on a sensible item.
6. No modal dialog without a keyboard path; no feature reachable only by mouse.
7. Errors are shown with the OS error, the path, and an action (retry / skip / skip all /
   cancel). Never a bare "operation failed".

## 6. Quality gates (enforced in CI)

| Gate | Target |
| --- | --- |
| Cold start to first paint | ≤ 50 ms (Linux, release, warm cache) |
| Listing 100 000 files | first paint ≤ 120 ms, full sort ≤ 400 ms |
| Input-to-paint latency while scrolling | ≤ 16 ms p99 |
| Memory for 100 000-entry panel | ≤ 60 MB RSS |
| `cargo clippy -- -D warnings`, `cargo fmt --check` | clean |
| `cargo deny` (licences + advisories) | clean |
| `unsafe` outside `fcmd-platform` | forbidden by lint |
| Snapshot tests of the rendered screen | all platforms |
| Fuzz targets (path parsing, archive readers, keymap parser) | no new crashes |

## 6b. The help, and the manual page

`F1` opens a book with chapters. Half of them are prose, written under `docs/help/` and
compiled into the binary so the program carries its own manual and a release is still one
file; the other half — the keys, every command with the keys that reach it, every setting with
its value — is **generated at the moment it is opened**, from the keymap as loaded and the
configuration as it stands. A written list of keys is wrong the day after it is written, and a
help page that lies is worse than no help page.

The prose carries no tables and is wrapped by the renderer rather than by hand: the same page
has to read on an eighty-column terminal and on a wide one. A test refuses a table in a help
page for that reason.

The manual page, `docs/fcmd.1`, is generated from the command line's own options by
`fcmd --man`, with the sections `clap` cannot know about — files, keys, exit status — written
once underneath. The file in the repository is that output and a test fails when the two
differ, so a flag cannot exist without a line in the page, and a release that forgets to
regenerate it does not build.

## 7. Milestones

| M | Theme | Done when |
| --- | --- | --- |
| **M1** | Daily driver | two panels, navigation, selection, copy/move/delete/mkdir with conflicts and progress, command line + user screen, viewer (text), config/keymap, the six built-in themes plus external theme loading, far+mc profiles, Linux/macOS/Windows builds |
| **M2** | mc/FAR parity | tabs, quick view/info/tree panels, filters, find files, archives (zip/tar/7z) read+write, job queue, editor, multi-rename, compare dirs, checksums, PTY subshell, trash, help system |
| **M3** | Commander parity | full VFS (SFTP/FTP/WebDAV/S3), connection manager, previews, syntax highlighting, macros, Lua API, WASM plugin ABI, diff viewer, synchroniser, elevated ops |
| **M4** | Beyond | multi-panel/windowed layout *inside the TUI*, plugin registry, native plugin ABI, special VFS (process/registry/git), headless CLI, split engine/UI over ssh |

Specs: `01-architecture`, `02-panels`, `03-file-operations`, `04-vfs`, `05-viewer-editor`,
`06-tools`, `07-config-theming`, `08-plugins`, `09-roadmap`, `10-menus-and-dialogs`.
What is specified but not built yet is listed, item by item, in `../BACKLOG.md`.
