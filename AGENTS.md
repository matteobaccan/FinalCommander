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

1. Write the section in `CHANGELOG.md` under a dated heading: `## [0.5.0] — 2026-11-02`.
   The release notes are read from that section.
2. Bump `version` in the workspace `Cargo.toml` (one place — every crate inherits it) and run
   a build so `Cargo.lock` follows.
3. Commit and push to the development branch. `.github/workflows/tag.yml` notices the bump and
   the dated heading, **waits for CI to be green on that very commit**, and creates the tag;
   `release.yml` then builds `fcmd-linux-x86_64-<v>.tar.gz`, `-static-`,
   `fcmd-macos-universal-<v>.tar.gz`, `fcmd-windows-x86_64-<v>.zip` and `SHA256SUMS`.
   No tag is pushed by hand.

**GitHub Pages must not publish by itself.** `pages.yml` builds the site on every push and
keeps it as the run's artifact; publishing happens only through `workflow_dispatch` with
`publish: yes`, by hand, deliberately.

## Where the work is right now

0.4.0 is released. Unreleased on the development branch, in order:

1. **The filesystem watch** — a local panel re-reads itself when its directory changes
   (`notify`, one thread, 300 ms debounce, `panel.auto_refresh`). SPEC 02 §2.6.
2. **The rename to FinalCommander** — crates, binary, configuration, plugin environment,
   release assets and pictures. Old configuration file names are still read and written, with
   one line on the startup screen saying so.
3. **The extension in a column of its own** in the full view, as FAR draws it.
4. **`Ctrl+O` takes the panels off** instead of opening a read-only screen: the command line,
   the menus and the keys all keep working, and `PgUp`/`PgDn` page through the output.
5. **The detailed view draws FAR's dates** (written, made, last read) and drops the extras
   before the name becomes unreadable.
6. **`fcmd-platform`**, the crate the rules always named: free space (on Windows too now),
   the device's size and allocation unit, and the machine's memory. It feeds the panel frame,
   the information panel (device, memory, slack) and the detailed view's allocated column.
7. **The keymap table swept** against the bindings the program resolves: `Ctrl+P`,
   `Shift+F10`, `Ctrl+W` and the editor's delete-word-right built, the rest of the rows
   corrected to what is true.

`main` was created from that work and is where it goes on.

## The queue, as the author's review left it

In the order they were raised, all from a side-by-side comparison with FAR on Windows:

- **View modes.** The extension column in `full` and the `detailed` view's dates and
  allocated column are done. What is left: the `wide` layout (the author has a screenshot of
  FAR's) and the file-owners mode.
- **Menu mnemonics.** The mechanism ships (`&` in a label marks the letter, the dropdown
  draws it highlighted), but only four labels carry a marker. The letters for the rest are
  the author's call.
- **The `F4` keybar label in the viewer** — a question to the author that is still open.
- **Tabular viewers** (dbf, csv, tsv, jsonl) are on the roadmap, not started.

One rule that came out of that review and holds everywhere: **`Alt+<letter>` belongs to the
quick search**, so nothing else may take it (`Ctrl+Shift+N/I/T` moved for this reason), and
the first `Enter` of a quick search both accepts it and acts on the row.
