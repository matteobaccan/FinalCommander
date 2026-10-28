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
cargo test --workspace --all-targets      # ~1 300 tests, 21 binaries
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
| `fcmd-platform` | the only crate allowed `unsafe`: free space, cluster size, memory, per platform |
| `fcmd-site` | the static site generator — not part of the program, `fcmd` does not link it |
| `fcmd` | the binary: the app, the panels, the rendering, the event loop |

Documentation: `docs/spec/` (the contract), `docs/02-features.md` (the feature inventory with
its sources), `docs/03-keymap.md` (every key, comparing FAR/MC/TC), `docs/BACKLOG.md` (what is
deliberately not built yet, with reasons), `docs/help/` (the in-program help, which the site
and `F1` both read), `docs/shots/` (SVG screenshots, hand-edited text — each `<text>` run's
character count must match its x-position list).

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

**0.7.2 is released.** What is in it, and in the three before it:

- **0.6.0** — `[[highlight]]` groups (colour by mask, size, age, attributes or kind, from the
  configuration); the find dialog's size condition; the system clipboard through the terminal
  with `OSC 52` (`Ctrl+Ins` names, `Alt+Shift+Ins` paths, and the editor's block), which also
  works over `ssh`; pasted text reaching the command line, a dialog row, the editor and a
  terminal instead of only a prompt; `Ctrl+8`, the file-owners view; a letter for every menu
  row in six languages; table mode in the viewer (`Shift+F3`) for CSV, TSV and JSON Lines.
- **0.7.0** — the dBase reader for table mode, read a window at a time; the dialog that edits
  the `[[highlight]]` groups (`F9 → Options → Files highlighting`); `Alt+F11`, the files that
  have been read or edited, persisted in the state file.
- **0.7.1** — what the author found by using 0.7.0: the key bar always draws all twelve keys
  (and three editor keys that worked and were never written down); the hex dump widens in
  blocks of eight; the table mode's arrows, `Ctrl+`arrows, `Home` and `End`; `Shift+F3` makes
  a table of anything that reads as one rather than going by the name; the DBF's numbers to
  the right, its deletion flag in a column of its own, its DOS code pages read as DOS code
  pages; the command line's history on `↑`/`↓`; and a command's output arriving while it
  runs rather than all at the end.
- **0.7.2** — the memo file beside a dBase table (`.dbt`, `.fpt`, all three formats); the sort
  groups on `Shift+F11`, from `[[sort_group]]` blocks; an arrow ends a quick search and leaves
  the cursor where it found; the name drawn in letters above what the program says on the
  command line.

## The queue, as the author's review left it

In the order they were raised, all from a side-by-side comparison with FAR on Windows. The
view modes, the menu letters and the tabular viewers are done; what is left is waiting on the
author:

- **The change-drive dialog** (`Alt+F1`/`Alt+F2`) is to look *exactly* like FAR's. The author
  sent a screenshot of ours; what is missing is one of FAR's own, because the column layout
  cannot be reconstructed from memory without guessing. **Asked, not answered.**
- **The `wide` view's layout** — same thing: the author has a FAR screenshot of it.
- **The `F4` keybar label in the viewer** — a question to the author that is still open.
- **The menu letters** are chosen by a rule now (first free word initial, then any free
  letter), in all six languages written in an alphabet, with a test keeping them unique. If
  the author wants FAR's own letters instead, the rule is one function and the catalogues are
  where the markers live.

## What is worth doing next, in rough order

Everything here is unblocked and specified; the backlog has the reasons.

1. **`descript.ion`** (`Ctrl+Z`, description columns, the information panel) — a whole small
   feature, and the one FAR habit this program has none of. It touches the job engine,
   because a description has to follow a copy, a move, a rename and a delete.
2. **The screens list and background screens** (`F12`): several viewers, editors and jobs
   open at once. The job engine already runs more than one; the screen stack is what is
   missing, and `Mode` holding one screen is what has to change.
3. **Custom columns** — the column grammar of SPEC 02 §3, which is also what the `wide` view
   wants.
4. **Sort criteria chains** (`sort.chain = ["size", "name"]`, SPEC 02 §4): the sort groups
   shipped in 0.7.2 and the chain is the half that is left. `Sort` is `Copy` and a `Vec` of
   keys would take that away, so the shape to pick is a small fixed array of tie-breakers.
5. **A dialog for the sort groups**, as `F9 → Options → Files highlighting` edits the
   colouring groups: the blocks can only be written by hand today.

One rule that came out of that review and holds everywhere: **`Alt+<letter>` belongs to the
quick search**, so nothing else may take it (`Ctrl+Shift+N/I/T` moved for this reason), and
the first `Enter` of a quick search both accepts it and acts on the row.
