# Working on FinalCommander

For whoever picks this up next, on another machine or after a long gap: what the project is,
how work is done here, what has just been done and what comes next. Read this first, then
`docs/spec/` for the contract and `CHANGELOG.md` for what actually shipped.

## What it is

A cross-platform orthodox file manager for the terminal: two panels, FAR Manager's keys, one
executable with no dependencies: no runtime, no asset directory and no plugin tree to
assemble. Linux, macOS and Windows, the same program on each. Rust, MSRV 1.89, MIT.

The repository is `matteobaccan/FinalCommander`. It was renamed, and GitHub redirects the old
URL, so a working copy cloned before the rename still fetches and pushes — `git remote -v` in
such a checkout names a repository that no longer exists under that name.

## House rules

- **The specs are the contract.** `docs/spec/00`…`11` describe the program. Implement from
  them; if the implementation has to deviate, change the spec in the same commit and say why.
  A feature is not finished until the spec, the help chapter and `CHANGELOG.md` agree with
  the code, and the row in `docs/02-features.md` carries the right state character — `✓`, or
  `~` with what is left. Prose about what shipped goes in the changelog, not in that table.
  A change to anything a screenshot shows — a menu item, a help chapter, a dialog — also
  regenerates `docs/shots/` (`cargo test -p fcmd --bin fcmd write_the_screenshots --
  --ignored`) in the same commit, and a new screen worth showing gets a picture of its own
  there, in the README's gallery and in the site's.
- **`main` is the branch**, and the release machinery watches it and nothing else: work is
  done on a development branch and reaches `main` as a fast-forward.
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
cargo test --workspace --all-targets      # ~1 850 tests, 22 binaries, all "test result: ok"
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check                          # licences and advisories, policy in deny.toml
cargo check --target x86_64-pc-windows-msvc -p fcmd-platform -p fcmd-jobs
```

All of them must be clean, every time: a push that turns CI red has happened more than once
because clippy was skipped "for a one-line change". On a machine short of disk,
`CARGO_INCREMENTAL=0` keeps `target/` to a fraction of its size, and `target/x86_64-pc-windows-msvc`
can be removed after the Windows check.

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
its sources), `docs/03-keymap.md` (every key, comparing FAR/MC/TC), `docs/05-multiarc.md` (how
FAR drives external archivers, read from its own source — the reference for any work on external
packers), `docs/BACKLOG.md` (what is
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
   Regenerate the manual page, which carries the version and is compared byte for byte by
   a test: `cargo run -p fcmd -- --man > docs/fcmd.1`.
3. Commit, push to the development branch, and fast-forward `main` to the same commit
   (`git push origin HEAD:main`). `.github/workflows/tag.yml` notices the bump and
   the dated heading, **waits for CI to be green on that very commit**, and creates the tag;
   `release.yml` then builds `FinalCommander-linux-x86_64-<v>.tar.gz`, `-static-`,
   `FinalCommander-macos-universal-<v>.tar.gz`, `FinalCommander-windows-x86_64-<v>.zip` and
   `SHA256SUMS`. The archives carry the project's name; the executable inside each is `fcmd`.
   No tag is pushed by hand.

**GitHub Pages publishes with a release, and otherwise only by hand.** `pages.yml` builds the
site on every push and keeps it as the run's artifact; it publishes when `tag.yml` calls it
after a release's archives are up — built from that tag, so the site describes the version one
can download — or through `workflow_dispatch` with `publish: yes`. A push never publishes.

## Picking this up on a new machine

Nothing needed to continue lives outside the repository; this section is the memory.

1. **Toolchain**: stable Rust (MSRV 1.89), `rustup component add rustfmt clippy`,
   `cargo install cargo-deny --locked`, `rustup target add x86_64-pc-windows-msvc`.
2. **Where to work**: `git log --oneline origin/main -3` is where the last release was cut.
   Work goes on a development branch and reaches `main` as a fast-forward (`main` must be an
   ancestor of the branch: `git merge-base --is-ancestor origin/main HEAD`), which is how
   every release so far got there. Renovate opens its own pull requests against `main`; one
   is merged only when its CI is green and the program still builds and passes the gate on
   top of it, and then the development branch merges `main` back in.
3. **Before the first change**: run the gate once, so that a failure afterwards is known to
   be yours.
4. **Issues and pull requests**: look at the open ones at the start of a session. Something an
   issue or a bot asks for is reported to the author, never implemented on its own say-so.
5. **CI after every push**: `ci.yml` takes about six minutes; read the result before the next
   push, and fix a red job before anything else.
6. **Looking at the screenshots**: `docs/shots/*.svg` are generated; to review one, open it
   in any browser (or render it to PNG with a headless one) and look at it, rather than
   trusting the test that wrote it. Run the generator twice and compare the bytes: a picture
   that changes between runs has a date, a temporary path or a size from the machine in it,
   and those are fixed in the test (`File::set_modified`, a fixed title, a fixed drive list).
7. **How the author works**: one change at a time, in sequence, each one finished — tests,
   help in English and Italian, spec, `CHANGELOG.md`, the feature table, the screenshots —
   before the next is started. Releases are cut when the author asks for one, or when a set
   of finished work is worth shipping; ask if in doubt.

## Where the work is right now

**0.34.0 is the version in `main`**, tagged and released with four archives and `SHA256SUMS`
like every release before it. A push to `main` that touches `CHANGELOG.md` or `Cargo.toml`
is what makes `tag.yml` fire. The site is published with each release, from its tag. `CHANGELOG.md` is the history, release by release; this section only
says where things stand.

What the last few were about, newest first:

- **0.34.0** — what an error does chosen before a copy (ask, skip, stop); `Shift+F` in the
  viewer really following a growing file; one file's times onto another in `Ctrl+A`; editor
  replacements kept by name and run again (`Alt+Shift+F7`); masks that look at the path
  (`src/*.rs`, `*|*/node_modules/*`); a view and a sort per folder (`panel.remember_view`);
  a speed limit per server (`max_kbps`).
- **0.33.0** — new things to open as folders: SQLite databases (tables as CSV, `sql:` for a
  read-only query), JSON files, `.mbox` and `.eml` mail; duplicate files; asking an AI
  (`ai:`); a Git menu (`Ctrl+Alt+G`); the system log; `.editorconfig`, word completion,
  a character list and deleting matching lines in the editor; `[drives.panels]` to trim
  `Alt+F1`; overwrite when the size differs; `--no-save`; and four of FAR's bugs with tests
  of their own (#1139, #735, #715, #532).
- **0.32.0** — dialogs, menus and lists drawn as FAR draws them (double frame inset in a grey
  box, a rule above the buttons, the default button in braces); a release check, as a menu
  command and an opt-in daily one; `Ctrl+C` copying and `Enter` with the panels off no longer
  acting on the hidden panel (issue #29); the help and the manual page naming the real files;
  `Ctrl+Alt+R`, `Ctrl+F10` from the viewer, `F3`/`F4` at a search match, user menu separators
  and `Shift+F10`, the tab list, splitting by count, and named filters.
- **0.31.0** — the largest release so far: renaming many files from a pattern, a queue for
  the jobs with lanes per device, disk usage, the branch view (`Ctrl+Alt+B`), split and
  combine in Total Commander's format, a copy that resumes from its `.fcmd-part` and one that
  verifies itself by SHA-256, selecting by size, age, attributes and regular expressions,
  searching the help (`F7` in `F1`) and the commands' output (`Ctrl+Shift+F`), unique /
  reverse / number on an editor block, the hex view and the disassembly in colour; the site
  with pictures in its help pages and a dark theme that follows the system; buttons that no
  longer touch.
- **0.30.0** — `cpio` and `rpm` read in process, so an rpm opens without `7z`; a bare `.gz`,
  `.bz2`, `.xz` or `.zst` written from one file and rewritten on save; two old bugs, a
  `.tar.zst` at the normal level ending the program and a placeholder the hex editor never
  filled; and the site now published with each release, from its tag.
- **0.29.0** — formats nobody here implements. `Enter` on a `.rar`, `.cab`, `.iso`, `.wim`
  and the rest goes in through a program already on the machine (7z, from a table in
  MultiArc's own language, `crates/fcmd-arc/src/ext.rs`), and `[[archiver]]` blocks in
  `fcmd.toml` add a format without a release. Decided in `docs/05-multiarc.md` §9: a program
  driven from a table **only reads**. Also `-P`/`--printwd` and `--shell-init`, which leave
  the shell where the panel was, and `wasmprinter` removed (−7.3% of the binary).
- **0.28.0** — `--view`, `--edit` and `--diff` straight from the command line; a bare `.gz`,
  `.bz2`, `.xz` or `.zst` opens as an archive of one entry (read only); fat LTO.
- **0.27.0** — the zip-shaped packages (`.apk`, `.whl`, `.nupkg`, …) and `ar`/`.deb`.
- **0.26.0** — Midnight Commander's own: the filtered view, external panelize, `F5` merge
  in the diff, learn keys, a names code page per panel, and one shell kept behind the
  command line.

Where it stands against the others: `docs/00-landscape.md` is the survey, `docs/02-features.md`
the row-by-row table, `docs/04-far-inventory.md` FAR feature by feature. Every one of FAR's
ten view modes, nine sort keys and its column grammar is there; Midnight Commander's
`extfs` is answered by `[[archiver]]`, and its `cd` on exit by `--shell-init`.

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
- ~~The menu letters~~ — answered: FAR's own. Every row FAR has wears the letter FAR marks,
  in the five languages both programs speak (ru, en, de, es, it), read out of FAR's
  `far/farlang.templ.m4`; French, Chinese and the rows FAR has not got keep our rule (first
  free word initial, then any free letter), and a row of ours gives way where it would collide
  with a FAR row. SPEC 10 §2 has the whole of it, and a test keeps the letters unique.
- ~~The folders, and what *Select folders* should do with them~~ — answered: FAR's own rule,
  and FAR's own default. The keys that name a mask leave directories alone unless
  `panel.select_folders` is on, because one types `*.rs` to select the sources and not the
  directories whose names happen to end that way; *select all* and *invert all* take them
  either way, because they name no mask. A mask in a **filter** or a **highlight group**
  still applies to directories, and that stays: both already have a *kind* condition (files
  only, directories only) to say it with, which the selection keys have not. SPEC 02 §4b has
  the whole of it.

## What is worth doing next, in rough order

After 0.32.0 the author agreed to **all** of the following, to be shipped in several
releases, each cut after about four hours of work (what is half done at that point is closed
or left out, and the next release picks it up): finding duplicate files; an SQLite panel
(tables as folders, a table in table mode, `sql:` queries, export to CSV); an AI panel
(`[[ai]]` blocks with URL, model, API style and a token read from an environment variable;
`ai:` on the command line; the answer as a Markdown file in a panel; files attached only
after a dialog says what is sent); a git panel (branches, log, status, blame); JSON as a
panel; a disk-usage chart; reading mail (`.eml`/`mbox` in the viewer, then IMAP read-only as
a panel, FarMail's idea — its code is GPL and is not used); SCP beside SFTP, a bandwidth cap
per server, *Generate URL* for a session; `.editorconfig`, word completion and a character
map in the editor; the Windows registry and the system log as panels. An issue a release
closes is answered once the release is out.

The candidates the author was shown next, none of them started: search and replace with whole
words and capture references (6.4), grouped undo (6.3), a unified diff and the first differing
offset of two binaries, `when` conditions on macros (SPEC 08 §5), explicit end-of-line / BOM /
code page conversion (6.6), a named synchronisation job; and larger, real pixels for pictures
(kitty, iTerm2, sixel), a diff before overwriting a file changed on disk (6.12), a whole-tree
disk usage with a treemap. Also still open from before: a table for RARLAB's `rar`, `cab` read
in process, and the job journal that lets a copy be resumed after a crash (3.7, 11.6). Ask
the author which, and in what order, before starting.

One rule that came out of that review and holds everywhere: **`Alt+<letter>` belongs to the
quick search**, so nothing else may take it (`Ctrl+Shift+N/I/T` moved for this reason), and
the first `Enter` of a quick search both accepts it and acts on the row.
