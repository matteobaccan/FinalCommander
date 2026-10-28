# Working on FinalCommander

For whoever picks this up next, on another machine or after a long gap: what the project is,
how work is done here, what has just been done and what comes next. Read this first, then
`docs/spec/` for the contract and `CHANGELOG.md` for what actually shipped.

## What it is

A cross-platform orthodox file manager for the terminal: two panels, FAR Manager's keys, one
executable of about five megabytes with no runtime, no asset directory and no plugin tree to
assemble. Linux, macOS and Windows, the same program on each. Rust, MSRV 1.89, MIT.

It shipped as **SupremeCommander** with the command `scmd` up to 0.4.0; since then it is
**FinalCommander**, command `fcmd`, and the repository is `matteobaccan/FinalCommander`. The
releases published before the rename still carry the old name in their archives, and GitHub
redirects the old repository URL to the new one.

## House rules

- **The specs are the contract.** `docs/spec/01`…`10` describe the program. Implement from
  them; if the implementation has to deviate, change the spec in the same commit and say why.
  A feature is not finished until the spec, `docs/02-features.md`, the help chapter and
  `CHANGELOG.md` agree with the code.
- **`main` is the branch**, and the release machinery watches it (`tag.yml` also watches the
  development branches, for work that is cut elsewhere).
- **Commits are authored by Matteo Baccan** (`matteo.baccan@gmail.com`) and carry no
  co-author or generated-by trailers. The message says what changed and *why* it changed, in
  prose, the way the ones already in `git log` do. No tooling is named anywhere in the
  repository — not in commits, not in code, not in comments, not in documentation.
- **English in the repository, Italian with the author.**
- **Nothing half-done is reported as done.** If a part is blocked, finish everything else and
  say plainly what was left and why.
- **No `unsafe`** outside `fcmd-platform`, where every call carries a `SAFETY:` line; no GUI
  toolkit, no GPL dependency. A new dependency needs a line of
  justification where it is declared and an entry in `THIRD-PARTY.md`.

## The gate, before every commit

```sh
cargo fmt --all
cargo test --workspace --all-targets      # ~1 400 tests, 22 binaries
cargo clippy --workspace --all-targets -- -D warnings
```

All three must be clean. `cargo deny check` is the fourth (licences and advisories, policy in
`deny.toml`); it runs in CI and is worth installing locally when touching dependencies.

CI (`.github/workflows/ci.yml`) runs four jobs on every push: `fmt + clippy`, `test` on
ubuntu/macos/windows, `cargo-deny`, and a release build on all three. Each platform's job can
fail alone — a test that writes to a fixed `/tmp` path or assumes `/` separators is the usual
cause on Windows, and a type that is not the same width on every Unix is the usual one on
macOS (`statvfs` counts blocks in 32 bits there and 64 on Linux).

**A test must never write out a path.** `assert_eq!(copied, "/home/mb/docs")` passes here and
fails on Windows, where the same `VPath` renders with a backslash; build the path the test
expects from the panel's own (`cwd.join("docs")`). This has cost a red Windows job and a
refused tag once already.

Anything with a `#[cfg]` in it is worth type-checking against the other platforms before
pushing, which costs seconds and saves a red build:

```sh
rustup target add x86_64-pc-windows-msvc aarch64-apple-darwin
cargo check --target x86_64-pc-windows-msvc -p fcmd-platform --all-targets
cargo check --target aarch64-apple-darwin  -p fcmd-platform --all-targets
```

(The whole workspace will not `check` for Windows from here — `ring` wants a C toolchain for
that target — but the crates that carry the `#[cfg]`s will.)

## Where things are

| Crate | What it owns |
| --- | --- |
| `fcmd-core` | paths (`VPath`), entries, listings, sorting, selection, filters, compare |
| `fcmd-vfs` | the `Vfs` trait and the local provider; archives, net and S3 plug in behind it |
| `fcmd-cmd` | the command vocabulary and the keymaps (`keymaps/far.toml` and friends) |
| `fcmd-config` | `fcmd.toml`, state, macros, the user menu, `Paths::discover` |
| `fcmd-i18n` | the seven Fluent catalogues (`i18n/<lang>/fc.ftl`) |
| `fcmd-ui` | widgets, themes, the drawing primitives |
| `fcmd-tui` | the terminal backend: input decoding, raw mode, painting |
| `fcmd-jobs` | the file operation engine: plan, execute, settle |
| `fcmd-view` / `fcmd-edit` | the viewer (text, dump, hex) and the editor |
| `fcmd-arc` / `fcmd-net` / `fcmd-s3` | archives, SFTP/FTP/WebDAV, object storage |
| `fcmd-find` / `fcmd-syntax` / `fcmd-term` / `fcmd-plug` | find files, syntax, the PTY, plugins |
| `fcmd-platform` | the only crate allowed `unsafe`: free space, cluster size, memory, owners, hard links, processes, sockets, installed programs, volumes — one module per subject, each dispatching to `windows.rs` on Windows |
| `fcmd-site` | the static site generator — not part of the program, `fcmd` does not link it |
| `fcmd` | the binary: the app, the panels, the rendering, the event loop |

Documentation: `docs/spec/` (the contract), `docs/02-features.md` (the feature inventory with
its sources), `docs/03-keymap.md` (every key, comparing FAR/MC/TC), `docs/BACKLOG.md` (what is
deliberately not built yet, with reasons), `docs/help/` (the in-program help, which the site
and `F1` both read), `docs/shots/` (SVG screenshots, which are **generated, never hand-edited** —
`cargo test -p fcmd --bin fcmd write_the_screenshots -- --ignored` rewrites all of them from
the renderer's own cell buffer, and everything in them is fixed on purpose so that running it
twice produces the same bytes). Both the README and the site read those files, so a change to
the drawing means regenerating them in the same commit.

## How the program is put together

One `mpsc::channel::<Event>` in `crates/fcmd/src/main.rs`. The app is a state machine that
answers `handle_event`/`handle_input` and queues `Request::*`; the loop drains the requests
each frame and hands them to worker threads or the tokio runtime, which answer with `Event`s.
Nothing blocks the UI thread, ever — a filesystem or network call inside a key handler is a
bug. The loop is input-driven and keeps no timer: anything that must happen on its own
(the filesystem watch, a job's progress) sends an `Event` from a thread of its own.

## Conventions that the tests enforce

- **i18n**: every user-visible string is a Fluent id in all seven catalogues. One test fails
  if an id is missing from a language, another if an id is never used. Plurals use the Fluent
  selector (`{ $count -> [one] … *[other] … }`).
- **Tests are sentences**: `a_junction_to_a_folder_sorts_with_the_folders`. A UI change gets a
  render-snapshot test (draw into a `Buffer`, assert on `to_text()`); anything touching the
  filesystem gets a `tempfile` test including the failure path; key handling gets a table.
- **Comments say why**, not what. The code in this repository is commented as prose for a
  reader who was not there; match it.
- **Portability**: `std::env::temp_dir()`, never a literal `/tmp`. Windows runs the same tests.

## Releases

1. Write the section in `CHANGELOG.md` under a dated heading: `## [0.7.0] — 2026-10-28`.
   The release notes are read from that section.
2. Bump `version` in the workspace `Cargo.toml` (one place — every crate inherits it) and run
   a build so `Cargo.lock` follows.
3. Commit and push to the development branch. `.github/workflows/tag.yml` notices the bump and
   the dated heading, **waits for CI to be green on that very commit**, and creates the tag;
   `release.yml` then builds `FinalCommander-linux-x86_64-<v>.tar.gz`, `-static-`,
   `FinalCommander-macos-universal-<v>.tar.gz`, `FinalCommander-windows-x86_64-<v>.zip` and
   `SHA256SUMS`. The archives carry the project's name; the executable inside each is `fcmd`.
   No tag is pushed by hand.

**GitHub Pages must not publish by itself.** `pages.yml` builds the site on every push and
keeps it as the run's artifact; publishing happens only through `workflow_dispatch` with
`publish: yes`, by hand, deliberately.

## Where the work is right now

**0.10.0 is written and waiting for its tag.** 0.8.0 is the last version with a tag and a
release on GitHub; 0.9.0 and 0.10.0 are both in `main` with their CHANGELOG sections, and the
tag never happened because the Actions queue on a private repository sat for longer than
`tag.yml` waits for green CI. The repository is public now, so the runners are free and the
queue moves; what is left is to touch `CHANGELOG.md` so `tag.yml` fires again. 0.9.0's notes
were folded into 0.10.0 when its own tag timed out, which is why the changelog goes from
0.10.0 straight to 0.8.0.

What is in the last few, newest first:

- **0.10.0** — the comparison the plugin everybody uses has (`AdvCmpEx`): a dialog of what
  counts as a difference (case, size, time to a tolerance, contents, contents only when the
  time differs, a first-fragment read, line-endings-only, selected only, stop at the first,
  newer only), **recursive through the subdirectories** with the answer as a panel of marked
  rows rather than a selection, and **find duplicates** under a directory by content. The
  change-drive dialog gained `F9` for its own switches and filled columns on Windows
  (`GetDriveTypeW`, `GetVolumeInformationW`, `WNetGetConnectionW`), and the prompt gained `$+`.
- **0.9.0** (folded into 0.10.0's notes) — the change-drive dialog as FAR draws it, over the
  panel it is about, with `Ctrl+1..9`, `Ctrl+R` and FAR's separators; `F3` on the process
  list as a real process card, on Windows too; a **socket list** as a panel of its own with
  the process holding each socket; a panel of **installed programs** with its card and its
  own uninstall command put on the command line unrun; the **prompt as a format** with the
  git branch in it (`$p$r$g `, FAR's letters, `%GITBRANCH%`); and the NetBox session editor
  with the pages the plugin's own dialog has.
- **0.8.0** — the author's own list, from FAR screenshots: the last two view modes (`Ctrl+9`
  links, `Ctrl+0` alternative full), so all ten of FAR's are there; `F4` in the viewer turns
  a wheel through text, dump, hex, document and table; the `wide` view fits its columns to
  the names it has; three panels that are not drives in `Alt+F1`/`Alt+F2` — NetBox (the
  stored sessions), the process list, the temporary panel — and text that can be marked out
  on the command line with `Shift` and an arrow.
- **0.7.3** — **file descriptions** (`descript.ion`), the one FAR habit the program had none
  of: read in whatever code page they are in, shown by `Ctrl+6` and `Ctrl+7`, in the
  information panel and in the status line, written by `Ctrl+Z`, and carried by a copy, a
  move, a rename and a delete. With them a sort criteria chain (`panel.sort_chain`).
- **0.7.2** — the memo file beside a dBase table (`.dbt`, `.fpt`, all three formats); the sort
  groups on `Shift+F11`, from `[[sort_group]]` blocks; an arrow ends a quick search and leaves
  the cursor where it found; the name drawn in letters above what the program says on the
  command line.
- **0.7.1** — what the author found by using 0.7.0: the key bar always draws all twelve keys;
  the hex dump widens in blocks of eight; the table mode's arrows, `Ctrl+`arrows, `Home` and
  `End`; the command line's history on `↑`/`↓`; and a command's output arriving while it runs
  rather than all at the end.
- **0.7.0** — the dBase reader for table mode, read a window at a time; the dialog that edits
  the `[[highlight]]` groups (`F9 → Options → Files highlighting`); `Alt+F11`, the files that
  have been read or edited, persisted in the state file.
- **0.6.0** — `[[highlight]]` groups; the find dialog's size condition; the system clipboard
  through the terminal with `OSC 52`, which also works over `ssh`; pasted text reaching the
  command line, a dialog row, the editor and a terminal; `Ctrl+8`, the file-owners view;
  table mode in the viewer (`Shift+F3`) for CSV, TSV and JSON Lines.

**All ten of FAR's view modes and all nine of its sort keys are there** (`Ctrl+1..0`,
`Ctrl+F3..F11`), the last two sort keys — by description and by owner — since 0.10.0's
`Unreleased` section. What the view modes still owe is the column grammar: FAR's ten modes are
each re-definable, ours are ten fixed lists.

## What the author's reviews have asked for, and where it went

Every one of these came out of a side-by-side comparison with FAR on Windows, and the plugins
the author actually uses are the reference for each: `FarGroup/FarManager` for the core,
`michaellukashov/Far-NetBox` for the connections, `FarPlugins/AdvCmpEx` for the comparison,
`FarPlugins/UnInstall` for the installed programs. Read the plugin's own sources and language
files rather than guessing what its dialog says.

- ~~The change-drive dialog, the `wide` view's layout, the `F4` keybar label~~ — 0.8.0.
- ~~The dialog FAR actually draws for the drives, and the panels that are not drives~~ —
  0.9.0, from FAR's own `diskmenu.cpp`: over its own panel, `Ctrl+1..9`, `Ctrl+R`, FAR's
  separators, and `F9` for the switches.
- ~~The NetBox session dialog's pages, the process card, the sockets, the installed
  programs, the prompt's git branch~~ — 0.9.0.
- ~~The comparison, with the options `AdvCmpEx` has, and into the subdirectories~~ — 0.10.0.
- **The menu letters** are chosen by a rule (first free word initial, then any free letter),
  in all six languages written in an alphabet, with a test keeping them unique. If the author
  wants FAR's own letters instead, the rule is one function and the catalogues are where the
  markers live. Still open, and the one thing here waiting on an answer rather than on work.

## What is worth doing next, in rough order

Everything here is unblocked and specified; the backlog has the reasons.

1. **Custom columns** — the column grammar of SPEC 02 §3: a user's own column set per view
   mode, which is what makes the ten modes a setting rather than a list, and what FAR's own
   `Column types` / `Column widths` strings describe.
2. **The screens list and background screens** (`F12`): several viewers, editors and jobs
   open at once. The job engine already runs more than one; the screen stack is what is
   missing, and `Mode` holding one screen is what has to change.
3. **A dialog for the sort groups**, as `F9 → Options → Files highlighting` edits the
   colouring groups: the blocks can only be written by hand today.
4. **Descriptions through a provider** — inside an archive, on a server — and carried out of a
   search result, which are what SPEC 02 §13 still owes.

One rule that came out of that review and holds everywhere: **`Alt+<letter>` belongs to the
quick search**, so nothing else may take it (`Ctrl+Shift+N/I/T` moved for this reason), and
the first `Enter` of a quick search both accepts it and acts on the row.
