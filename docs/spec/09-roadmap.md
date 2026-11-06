# SPEC 09 — Roadmap

Sized in issue-level chunks so M1 can start immediately. Each chunk is independently
reviewable and testable.

## M1 — Daily driver (the walking skeleton, then the keys)

| # | Chunk | Depends on |
| --- | --- | --- |
| 1 | workspace skeleton: crates, CI (fmt/clippy/deny/test on 3 OS), release profile, `fcmd --version` | — |
| 2 | `fcmd-core`: `VPath`, `Entry`, `Listing`, `Selection`, `Sort`, `Totals`, predicate evaluator | 1 |
| 3 | `fcmd-tui`: backend trait, cell buffer, diff renderer, input decoding + capability detection | 1 |
| 4 | `fcmd-ui`: panel widget, key bar, status line, message/confirm dialog, input dialog | 2,3 |
| 5 | `fcmd-vfs`: trait + local provider (streaming list, stat, read/write, rename, remove) | 2 |
| 6 | panel behaviour: navigation, cursor, scrolling, enter/parent/root, history, re-read | 4,5 |
| 7 | view modes brief/full + sorting (all keys, reverse, dirs-first, natural) | 6 |
| 8 | selection: `Ins`, ranges, masks `+`/`-`/`*`, select all/none/invert, totals | 6 |
| 9 | fast find / quick search in panel | 6 |
| 10 | `fcmd-cmd`: command registry, keymap loader, scope stack, chords, `--check-keymap` | 2 |
| 11 | keymap profiles `far` + `mc`, learn-keys dialog, `keys.local.toml` | 10 |
| 12 | `fcmd-config`: discovery, TOML load/merge/validate, `--dump-config`, hot reload | 2 |
| 13 | ✅ `fcmd-jobs`: job model, planner, executor, progress, conflict dialog; copy/move/delete/mkdir | 5,12 |
| 14 | command line + `%`-macros + user screen (`Ctrl+O`) + run command | 6,12 |
| 15 | ✅ `fcmd-view`: viewer with wrap, windowed reader, encodings, plain/regex/hex search, hex and dump modes, follow, bookmarks | 5 |
| 21 | ✅ `fcmd-edit`: line buffer with encoding/EOL/BOM preserved, atomic save, undo, stream and column blocks, search/replace, go to line, bookmarks; `F4`/`Shift+F4` in the TUI | 5,12 |
| 22 | ✅ `fcmd-net` (netbox): SFTP provider, site list, connection pool, host-key policy, transfers through the job engine; `Ctrl+Shift+N`, remote panels, `sftp://` in `Ctrl+L` | 5,12,13 |
| 23 | ✅ menu bar (`F9`) with FAR's five menus, settings dialogs (System/Panel/Interface/Confirmations), languages, themes, drives, folder history and sort-mode lists, `Save setup` (SPEC 10) | 4,10,12 |
| 16 | `fcmd-platform` M1 slice: long paths, trash, clipboard, drives/mounts, FS watch | 1 |
| 17 | ✅ theming: theme loader, seven built-in themes (`commander`, `far`, `black`, `white`, `matrix`, `tron`, `blade`), colour degradation | 3,12 |
| 18 | ✅ packaging: a tag builds linux-gnu, linux-musl (static), macOS universal and windows-msvc, with checksums and release notes from the changelog | 1 |
| 19 | snapshot test harness + keymap tests + job property tests + perf budget check | all |
| 20 | ✅ docs: README, help pages generated from `docs/`, man page | all |

**Done so far**: every chunk, the help and the man page (20) since 0.1.0 among them, except one
part of 19 — the performance budget checks in CI, which are what M1 still owes. `docs/BACKLOG.md`
lists every menu entry that is in the bar but not yet behind a feature.

Exit criteria: the authors use it instead of `mc`/FAR for a week without reaching for the old
tool for anything in M1 scope; all quality gates green on three platforms.

## Release plan

M1–M4 say what the thing becomes; this says what the next few tags contain. The order is by
what is missed most, not by what is easiest, with one exception: anything that unblocks two
later items goes first. Sizes are honest estimates of focused work, not of calendar time.

| Release | Theme | What is in it | Size | Why here |
| --- | --- | --- | --- | --- |
| ~~**0.0.8**~~ | The hex editor (the Hiew track) | **Shipped.** Hex mode of the editor with a patch list and an atomic save, block operations (`F3` marks, `F12` operates), the assembler for the instructions a patcher types, a calculator, search by instruction pattern, and raw devices on Linux and macOS. SPEC 05 §2.5–§2.9. What was left out on purpose: a keyed cipher over a block and devices on Windows, both on the backlog with their reasons. Memory operands in the assembler came later, in 0.18.0. | A release of its own | Deliberately in the editor, not the viewer: the viewer's read-only contract is what lets it seek through 40 GB |
| ~~**0.1.0**~~ | Macros, plugins, and the screens | **Shipped.** The macro engine (record, assign to a key, two play modes, a list), the screens (`F12`, `Ctrl+Tab`) with the background counter, plugin commands (`F11`) run as processes with a JSON protocol, the help as a book whose reference chapters are generated from the live keymap, and the manual page generated from the command line's own options. What is left for the plugin tiers above: a plugin that *adds* a column, a preview or a filesystem, and one that is asked a question rather than told to run. | The macro engine was the large one | Macros need everything else to be a command first, which is why they were last rather than first |
| **0.37.0** | The documentation, all of it | README, help (en + it), site, manual page, specs and the documents under `docs/`, read against the program: every key and every setting named is checked against the keymaps and `fcmd-config`, the sentences an older release made true are rewritten, and the README gains a key table for people coming from FAR, Total Commander and mc | A release of its own, no features | Thirty-six releases of features left sentences behind them; the survey of November 2026 is where the next ones come from |
| **0.38.0** | Git, table mode, metadata | Git beyond the panel it has, all of it in panels: branches and switching, what is to commit and what is staged, stage/unstage, commit, push, pull, a diff, a file's log. Table mode: a header row detected, `^`, copying the selected rows. Metadata as far as each format goes, executables included. A written study of what the AI could do next | A release, larger than most | Asked for by the author after the survey; the rest of the survey's gaps are in `docs/BACKLOG.md` |

0.0.7 was a fix release for a hang in 0.0.6's tree panel, which is why the Hiew track is
0.0.8 rather than 0.0.7.

Running alongside: **mouse support** (0.4.0), the **OS clipboard** through `OSC 52` (0.6.0),
the **trash**, **descriptions** (0.7.3), the **settings browser** (0.16.0), and **AArch64
disassembly** (0.18.0) have all gone out. What is still running alongside is the packaging work
(`.deb`, `.rpm`, Homebrew, Scoop, AUR), which SPEC 11 costs channel by channel. Since 0.1.0 a
release is cut when there is something worth having in it rather than to a plan written in
advance, and `CHANGELOG.md` is the record of what each one held.

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

**Where it stands** (read against `docs/02-features.md` for 0.37.0): almost all of it is in.
Still open, with the row that tracks each — the journal that lets a copy resume after a crash
(3.7, 11.6; the queue itself shipped in 0.31.0), more than one file open in the editor (6.1),
the selection kept by session restore (9.7), the `tc` and `nc` keymap profiles (only `far` and
`mc` ship), a stacked layout (1.2), icons (1.22), opening the item in the desktop's own file
manager (8.8), and the packages.

## M3 — Commander parity and beyond the terminal's usual limits

**Headline: the rest of netbox** (SFTP and the connection manager landed in M1) — FTP/FTPS/WebDAV/S3 in one first-party plugin, plus the connection
manager and keyring; then rclone bridge · nested containers and
write-back · previews (images via kitty/sixel, markdown, PDF, media) · **table mode for the
tabular formats** (CSV/TSV and JSONL first, then DBF — §1.11 of the viewer spec) · syntax highlighting
(tree-sitter) in viewer and editor · diff viewer · synchroniser · macros · Lua API · WASM
plugin host + WIT interfaces + capability grants · elevated helper · embedded terminal tabs ·
fuzzy goto · ACL/xattr/ADS editors · accessibility mode · `vi` profile · config import from
far/tc/mc/dc.

## M4 — Ambition

Multi-panel / windowed layout **inside the TUI** (the DOS Navigator idea: several panels and
floating windows drawn in the character grid) · plugin registry and `fcmd plugin` CLI ·
native plugin ABI + optional TC packer-plugin bridge on Windows · special VFS (process list,
registry, services, git) · headless CLI (`fcmd copy --queue`) · Android/iOS and web experiments ·
remote `fcmd` protocol (run the engine on the server, the UI locally — far2l's best trick).

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

## How to contribute

1. Pick a chunk from the table; the spec section it implements is linked in the issue.
2. Specs are the contract: if the implementation needs to deviate, the PR updates the spec in
   the same commit.
3. Every user-visible behaviour needs a snapshot or keymap test; every job behaviour needs a
   failure-injection test.
4. No new dependency without a justification line; no `unsafe` outside `fcmd-platform`.
