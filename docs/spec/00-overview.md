# SPEC 00 — Overview, goals, non-goals

## 1. What we are building

**SupremeCommander** (`sc`) is a cross-platform orthodox file manager written in Rust:
two panels, a command line, a function-key bar, keyboard-first, with an internal viewer and
editor, a virtual filesystem layer (archives, SFTP, S3, …), a job engine for file operations,
and an extension layer (Lua + WebAssembly).

Primary UI is a **TUI** that runs in any terminal (local, over SSH, inside tmux). An optional
GUI backend (M4) renders the same character grid on a GPU canvas with its own font, true
colour and inline images — the MTN2 approach — without forking the UI code.

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
- **G6 — Small, fast, boring to install.** Single static binary, < 20 MB, cold start < 50 ms,
  no runtime dependencies for the core features.

## 3. Non-goals

- Not an Explorer/Finder replacement with a single-pane shell-extension UX.
- No cloud sync service, no accounts, no "AI assistant" in the panel.
- No bundled proprietary blobs; no feature that requires a non-free component to work
  (RAR *write*, for example, is simply out of scope).
- Not bound to a desktop toolkit: no GTK/Qt dependency in the core.
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

1. The function-key bar always shows what `F1..F12` do *right now*, including modifier rows.
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
| `unsafe` outside `sc-platform` | forbidden by lint |
| Snapshot tests of the rendered screen | all platforms |
| Fuzz targets (path parsing, archive readers, keymap parser) | no new crashes |

## 7. Milestones

| M | Theme | Done when |
| --- | --- | --- |
| **M1** | Daily driver | two panels, navigation, selection, copy/move/delete/mkdir with conflicts and progress, command line + user screen, viewer (text), config/keymap/theme, far+mc profiles, Linux/macOS/Windows builds |
| **M2** | mc/FAR parity | tabs, quick view/info/tree panels, filters, find files, archives (zip/tar/7z) read+write, job queue, editor, multi-rename, compare dirs, checksums, PTY subshell, trash, help system |
| **M3** | Commander parity | full VFS (SFTP/FTP/WebDAV/S3), connection manager, previews, syntax highlighting, macros, Lua API, WASM plugin ABI, diff viewer, synchroniser, elevated ops |
| **M4** | Beyond | GUI backend, multi-panel windows, plugin registry, special VFS (process/registry/git), headless CLI, mobile/web experiments |

Specs: `01-architecture`, `02-panels`, `03-file-operations`, `04-vfs`, `05-viewer-editor`,
`06-tools`, `07-config-theming`, `08-plugins`, `09-roadmap`.
