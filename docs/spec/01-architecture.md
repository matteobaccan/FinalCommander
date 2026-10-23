# SPEC 01 — Architecture

## 1. Crate layout (cargo workspace)

```
scmd-core        domain types: Entry, Meta, Path abstraction, Selection, SortKey, Filter, errors
scmd-vfs         Vfs trait + providers (local, archive, sftp, ftp, webdav, s3, temp, trash)
scmd-jobs        operation engine: plan/execute/settle, conflict policy, progress (queue and
               journal still to come)
scmd-ui          immediate-mode widgets on top of a cell buffer: panel, dialog, menu, keybar
scmd-tui         the only backend: terminal I/O, input decoding, capability detection
scmd-view        viewer: text/dump/hex, encodings, search (preview pipeline still to come)
scmd-edit        editor: line buffer (rope later), column blocks, undo, syntax (tree-sitter)
scmd-cmd         command vocabulary, keymap, macro engine, command palette
scmd-config      TOML config, themes, profiles, portable-mode discovery, migration
scmd-plugin      Lua (mlua) + WASM (wasmtime) hosts, capability-scoped plugin API
scmd-platform    the only crate allowed `unsafe`: OS specifics (ADS, xattr, trash, elevation,
               long paths, FS watch, PTY, clipboard, drives/mounts)
scmd-i18n        fluent catalogues, pluralisation, width-aware formatting
scmd           the binary: wiring, CLI, session, panic/crash handling
```

Dependency rule: `scmd-core` depends on nothing of ours; `scmd-ui` draws into a cell buffer and
knows nothing about the terminal; only `scmd-tui` talks to a terminal, and only `sc` wires them
together. The UI is strictly TUI — there is no second backend, and the cell-buffer boundary
exists for testability (render to a string and diff it), not to leave room for a GUI.
Enforced by dependency allow-lists in CI.

## 2. Process and thread model

```
main thread            ── UI loop: decode input → dispatch Command → update state → render
tokio runtime (mt)     ── VFS I/O, network, directory scans, previews, hashing
rayon pool             ── CPU-bound: sorting large listings, checksum, search in files
blocking pool          ── OS calls that can hang (stat on dead NFS, Windows drive probe)
pty threads            ── one reader thread per embedded terminal
```

- UI state is owned by the main thread. Everything else communicates by messages:
  `Event::{Input, Vfs, Job, Plugin, Watch, Tick}` on an `mpsc` channel.
- No lock is ever held across an `await` or across a frame.
- Every background task carries a `CancellationToken`; leaving a directory cancels its scan,
  its sizing and its previews.
- A frame is rendered only when state changed or a 60 Hz tick has pending animation; idle cost
  is zero CPU (blocking read on the event channel).

## 3. Rendering

- `scmd-ui` draws into a **cell buffer** (`Cell { char, fg, bg, attrs, wide_flag }`) with
  double-buffered diffing; `scmd-tui` emits only the changed runs.
- Unicode width via `unicode-width` + explicit east-asian-wide handling; a wide glyph occupies
  two cells with a continuation marker, so panel columns never tear (MTN2's CJK rule).
- Immediate-mode widgets: `fn panel(&mut Frame, &PanelState)`. No retained widget tree, no
  callbacks — this is what makes snapshot testing trivial (`render_to_string(80, 24)`).
- Colour: 24-bit by default, automatic degradation to 256/16/mono from terminfo and
  `COLORTERM`; themes declare colours once and the backend maps them.

## 4. Input

- `scmd-tui` decodes: legacy xterm sequences, kitty keyboard protocol (enabled when advertised),
  Win32 input mode on Windows, bracketed paste, focus events, SGR mouse, mouse wheel.
- Decoded into `KeyEvent { code, mods, kind }`; `scmd-cmd` resolves it through the active keymap
  and the current **scope stack** (dialog > editor > viewer > panel > global).
- Ambiguities (Esc vs Alt, F-keys without modifiers reported) are resolved by capability
  detection at startup plus the "learn keys" dialog, persisted to `keys.local.toml`.
- Chord support (`F12 5`, `Ctrl+x c`) so mc-style prefixes and fallbacks work.

## 5. Paths

`scmd-core::VPath` — a VFS-aware path:

```rust
pub struct VPath { scheme: SchemeId, root: Arc<str>, segments: Vec<OsString> }
```

- Local paths keep `OsString` (so non-UTF-8 names on Unix and UTF-16 oddities on Windows
  survive a round trip); display uses lossy conversion marked with a sentinel.
- Windows: long paths via `\\?\` prefixing in `scmd-platform`, UNC aware, case-insensitive
  comparison with case-preserving display.
- Archive/remote paths are `scheme://root!/inner/path`; nesting is allowed
  (`sftp://host/a.tar.gz!/b.zip!/c.txt`) with a depth limit and an explicit cache.
- No `String`-based path concatenation anywhere; a lint forbids `format!("{}/{}")` on paths.

## 6. Error handling

- `thiserror` per crate, one `ScError` at the boundary, every error carries the `VPath` and the
  raw OS error code.
- Operations never `unwrap`; the job engine turns an error into a **decision point**
  (retry/skip/skip-all/cancel) and records it in the job log.
- A panic in a plugin or a preview task never takes the app down: plugin tasks run isolated
  (WASM instance or `catch_unwind`-wrapped Lua) and the UI shows a one-line failure.
- Crash handler writes a report with the last 100 events (no file contents, no paths from
  other users) to `$STATE/crash/`.

## 7. Configuration discovery

1. `--config <dir>` wins.
2. Portable mode: a `scmd-portable` marker file next to the executable → config beside the binary.
3. Otherwise XDG (`$XDG_CONFIG_HOME`), `~/Library/Application Support`,
   `%APPDATA%`; state/cache separated (`$XDG_STATE_HOME`, `$XDG_CACHE_HOME`).

Files: **one**. `scmd.toml` holds options plus the `[[theme]]`, `[[keymap]]`, `[[filter]]`,
`[[assoc]]`, `[[menu]]` and `[[site]]` blocks (no secrets — those go to the OS keyring);
`scmd-state.toml` beside it holds histories and the last session. Themes, keymap profiles,
language catalogues and help are compiled into the executable, and the only directory ever
read is the optional `plugins/`. See SPEC 07 §1.

## 8. Testing strategy

| Level | How |
| --- | --- |
| unit | pure functions in `scmd-core`/`scmd-jobs` (sorting, filters, conflict matrix, rename patterns) |
| golden screen | render a scripted session to a text grid and diff against a snapshot (`insta`) |
| keymap | table-driven: profile + key → expected command id |
| VFS conformance | one test suite run against every provider (local, zip, tar, sftp-in-docker, s3 via minio) |
| jobs | property tests on a simulated FS with injected errors (ENOSPC, EACCES, vanishing files, symlink loops) |
| platform | per-OS integration tests in CI matrix (ubuntu/macos/windows), plus FreeBSD cron job |
| fuzz | `cargo-fuzz` on archive readers, path parser, keymap/theme parsers, viewer decoders |
| performance | criterion benches + a CI budget check on the gates in SPEC 00 §6 |

## 9. Dependency policy

Allowed core dependencies (all permissive): `crossterm`/`ratatui`, `tokio`, `rayon`,
`unicode-width`, `unicode-segmentation`, `encoding_rs`, `memchr`, `regex`, `globset`,
`serde`/`toml`, `thiserror`, `tracing`, `zip`/`tar`/`flate2`/`zstd`/`xz2`/`sevenz-rust2`,
`russh`/`suppaftp`/`reqwest-dav`/`aws-sdk-s3` (feature-gated), `mlua`, `wasmtime`,
`portable-pty`, `notify`, `trash`, `keyring`, `tree-sitter`, `fluent`.

Rules: no GPL/AGPL dependency in the default feature set; every optional backend behind a
cargo feature; `cargo deny` fails the build on a licence outside the allow-list; new
dependencies need a one-line justification in the PR.
