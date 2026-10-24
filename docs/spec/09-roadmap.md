# SPEC 09 — Roadmap

Sized in issue-level chunks so M1 can start immediately. Each chunk is independently
reviewable and testable.

## M1 — Daily driver (the walking skeleton, then the keys)

| # | Chunk | Depends on |
| --- | --- | --- |
| 1 | workspace skeleton: crates, CI (fmt/clippy/deny/test on 3 OS), release profile, `scmd --version` | — |
| 2 | `scmd-core`: `VPath`, `Entry`, `Listing`, `Selection`, `Sort`, `Totals`, predicate evaluator | 1 |
| 3 | `scmd-tui`: backend trait, cell buffer, diff renderer, input decoding + capability detection | 1 |
| 4 | `scmd-ui`: panel widget, key bar, status line, message/confirm dialog, input dialog | 2,3 |
| 5 | `scmd-vfs`: trait + local provider (streaming list, stat, read/write, rename, remove) | 2 |
| 6 | panel behaviour: navigation, cursor, scrolling, enter/parent/root, history, re-read | 4,5 |
| 7 | view modes brief/full + sorting (all keys, reverse, dirs-first, natural) | 6 |
| 8 | selection: `Ins`, ranges, masks `+`/`-`/`*`, select all/none/invert, totals | 6 |
| 9 | fast find / quick search in panel | 6 |
| 10 | `scmd-cmd`: command registry, keymap loader, scope stack, chords, `--check-keymap` | 2 |
| 11 | keymap profiles `far` + `mc`, learn-keys dialog, `keys.local.toml` | 10 |
| 12 | `scmd-config`: discovery, TOML load/merge/validate, `--dump-config`, hot reload | 2 |
| 13 | ✅ `scmd-jobs`: job model, planner, executor, progress, conflict dialog; copy/move/delete/mkdir | 5,12 |
| 14 | command line + `%`-macros + user screen (`Ctrl+O`) + run command | 6,12 |
| 15 | ✅ `scmd-view`: viewer with wrap, windowed reader, encodings, plain/regex/hex search, hex and dump modes, follow, bookmarks | 5 |
| 21 | ✅ `scmd-edit`: line buffer with encoding/EOL/BOM preserved, atomic save, undo, stream and column blocks, search/replace, go to line, bookmarks; `F4`/`Shift+F4` in the TUI | 5,12 |
| 22 | ✅ `scmd-net` (netbox): SFTP provider, site list, connection pool, host-key policy, transfers through the job engine; `Alt+N`, remote panels, `sftp://` in `Ctrl+L` | 5,12,13 |
| 23 | ✅ menu bar (`F9`) with FAR's five menus, settings dialogs (System/Panel/Interface/Confirmations), languages, themes, drives, folder history and sort-mode lists, `Save setup` (SPEC 10) | 4,10,12 |
| 16 | `scmd-platform` M1 slice: long paths, trash, clipboard, drives/mounts, FS watch | 1 |
| 17 | theming: theme loader, `far-classic` + `mono`, colour degradation | 3,12 |
| 18 | ✅ packaging: a tag builds linux-gnu, linux-musl (static), macOS universal and windows-msvc, with checksums and release notes from the changelog | 1 |
| 19 | snapshot test harness + keymap tests + job property tests + perf budget check | all |
| 20 | docs: README, help pages generated from `docs/`, man page | all |

**Done so far**: chunks 1–19 and 21–23, including the operation engine (13), the viewer (15),
the editor (21), the netbox (22) — the three priority components of SPEC 08 §8 are all in the
binary — the menu bar with its settings dialogs (23), and the command line with the user screen
(14, shipped in 0.0.3). What M1 still owes: the generated help and man pages (20), the trash,
and the performance budget checks in CI. `docs/BACKLOG.md` lists every menu entry that is in the
bar but not yet behind a feature.

Exit criteria: the authors use it instead of `mc`/FAR for a week without reaching for the old
tool for anything in M1 scope; all quality gates green on three platforms.

## Release plan

M1–M4 say what the thing becomes; this says what the next few tags contain. The order is by
what is missed most, not by what is easiest, with one exception: anything that unblocks two
later items goes first. Sizes are honest estimates of focused work, not of calendar time.

| Release | Theme | What is in it | Size | Why here |
| --- | --- | --- | --- | --- |
| **0.0.8** | The hex editor (the Hiew track) | **Hex mode of the editor** with a patch list and an atomic save, **block operations** (`F3` marks, `F12` operates), the **assembler**, a **calculator**, **search by instruction pattern**, and **raw devices**. SPEC 05 §2.5–§2.8 and `docs/02-features.md` §5b. | A release of its own | Deliberately in the editor, not the viewer: the viewer's read-only contract is what lets it seek through 40 GB |
| **0.1.0** | Macros, plugins, and the screens | **Macro engine** (record and play per area, two play modes, a list) on top of the command vocabulary, the **screens list** (`F12`) and the background-screen counter, the **plugin host API** so a plugin can do more than carry data, the **help system** generated from `docs/`, and the **man page**. | The macro engine is the large one | Macros need everything else to be a command first, which is why they are last rather than first |

0.0.7 was a fix release for a hang in 0.0.6's tree panel, which is why the Hiew track is
0.0.8 rather than 0.0.7.

Running alongside, each going out with whichever release it is ready for: **mouse support**, the
**OS clipboard** (`scmd-platform`), the **trash**, **descriptions** (`descript.ion`), the
**settings browser** for the ~50 keys we have, **AArch64 disassembly**, and the packaging work
(`.deb`, `.rpm`, Homebrew, Scoop, AUR).

Two things are explicitly *not* on this list, and `docs/BACKLOG.md` says why: Netware module
formats, and anything that needs reverse engineering a proprietary program rather than reading
its published description.

## M2 — mc/FAR parity

**Headline: what the viewer and the editor still owe** (the first two of the three priority
components, SPEC 08 §8, landed in M1 — see SPEC 05 for what each one does not do yet), then: tabs · quick view / info / tree panels · filters + highlight groups · custom columns ·
find files + grep + panelize · disk usage · archives (zip/tar/7z read+write, pack/unpack
dialogs) · temp panel · job queue panel + journal/resume · multiple editor buffers + OS clipboard ·
multi-rename ·
compare dirs · checksums/split/combine · attributes & permissions dialogs · links · trash/wipe ·
PTY subshell with cwd sync · user menu · associations · histories with pinning · hex/dump
viewer modes + regex/hex search · help system · session restore · `tc`/`nc` keymap profiles ·
`.deb`/`.rpm`/Homebrew/Scoop/AUR packages.

## M3 — Commander parity and beyond the terminal's usual limits

**Headline: the rest of netbox** (SFTP and the connection manager landed in M1) — FTP/FTPS/WebDAV/S3 in one first-party plugin, plus the connection
manager and keyring; then rclone bridge · nested containers and
write-back · previews (images via kitty/sixel, markdown, PDF, media) · syntax highlighting
(tree-sitter) in viewer and editor · diff viewer · synchroniser · macros · Lua API · WASM
plugin host + WIT interfaces + capability grants · elevated helper · embedded terminal tabs ·
fuzzy goto · ACL/xattr/ADS editors · accessibility mode · `vi` profile · config import from
far/tc/mc/dc.

## M4 — Ambition

Multi-panel / windowed layout **inside the TUI** (the DOS Navigator idea: several panels and
floating windows drawn in the character grid) · plugin registry and `scmd plugin` CLI ·
native plugin ABI + optional TC packer-plugin bridge on Windows · special VFS (process list,
registry, services, git) · headless CLI (`scmd copy --queue`) · Android/iOS and web experiments ·
remote `scmd` protocol (run the engine on the server, the UI locally — far2l's best trick).

## Risks and how we de-risk them

| Risk | Mitigation |
| --- | --- |
| Terminal key limitations make the FAR keymap unusable | capability detection + kitty protocol + chord fallbacks + learn-keys, all in M1 (§9 of `03-keymap.md`) |
| Scope explosion (FAR has 30 years of features) | the feature matrix is milestone-tagged; M1 ships a *complete* small thing, not a broken big one |
| Async + UI complexity | one rule: UI thread owns state, everything else is messages; enforced by the crate graph |
| Archive/VFS edge cases eat months | one conformance suite run against every provider; unsupported capability is a clear message, not a bug |
| Data loss bugs | job journal, write-to-temp-then-rename, verify option, property tests with injected failures, no delete before a verified copy |
| Plugin ABI churn | WIT-versioned WASM interfaces from the start; native ABI deferred to M4 |
| Licence contamination | `01-licensing.md` rules + `cargo deny` + PR review checklist |
| Single-maintainer bus factor | small crates, documented specs, generated help/docs, no hidden tribal knowledge |

## How to contribute (once M1 starts)

1. Pick a chunk from the table; the spec section it implements is linked in the issue.
2. Specs are the contract: if the implementation needs to deviate, the PR updates the spec in
   the same commit.
3. Every user-visible behaviour needs a snapshot or keymap test; every job behaviour needs a
   failure-injection test.
4. No new dependency without a justification line; no `unsafe` outside `scmd-platform`.
