# Contributing

Thanks for looking. The program is a daily driver, and what is left is parity with FAR and
Midnight Commander feature by feature (`docs/spec/09-roadmap.md` has the roadmap,
`docs/BACKLOG.md` the gaps with their reasons).

## The short version: open an issue, not a pull request

**Please do not open a pull request with new code.** Say what is missing or wrong in an
**issue** — there is a form for each — and it will be built for you, as a whole: the spec
section, the help chapter, the changelog entry, the tests on three platforms and the strings
in seven languages. That last part is most of the work, and a patch to the code alone cannot
carry it.

There is a licensing reason as well, and it is the harder one: far2l, Midnight Commander,
Krusader and Double Commander are GPL, this program is MIT, and code that arrives in a pull
request cannot be unseen afterwards. A description of a behaviour carries no such risk —
"FAR does this, it is called that, here is the screen" is the most useful thing you can send.

Welcome without any code: **bug reports** (what you did, what happened, what you expected,
`fcmd --version`, your platform), **missing features** (which program you know it from and
what it is called there), **keymaps and themes** (TOML, not code — paste one into an issue;
`fcmd --export-theme <id>` prints one to start from), and **translations** (a corrected line
from `crates/fcmd-i18n/i18n/<lang>/fc.ftl`, in an issue).

The rest of this file is the standard everything here is held to. It is written for whoever
does the building — today the maintainer — and it is also what to expect of the code you are
reading.

`AGENTS.md` is the short version of everything below, plus where the work stands today and
what is queued next. Read it first if you are picking the project up rather than fixing one
thing.

## Ground rules

1. **The specs in `docs/spec/` are the contract.** Implement from them. If the implementation
   has to deviate, change the spec in the same commit and say why.
2. **Licensing.** This project is MIT (inbound = outbound: your contribution is MIT too).
   - FAR Manager (BSD-3-Clause) may be studied and partially reused — keep its notice and add
     an entry to `THIRD-PARTY.md`.
   - far2l, Midnight Commander, Krusader, Double Commander are **GPL**: read them to understand
     behaviour, never copy or translate their code. A PR whose diff mirrors a GPL file's
     structure will be rejected.
   - Total Commander and other proprietary commanders: observe them as a user, never decompile.
   - No GPL/LGPL dependency in the default feature set; see `docs/01-licensing.md` §5.
3. **Strictly TUI.** No GUI toolkit, no GPU backend, no "windowed mode". Terminal features
   (inline images, kitty keyboard protocol, OSC 52) are capability-detected extras.
4. **No `unsafe`** outside `fcmd-platform`, which is where the calls `std` does not make
   live — each one with a `SAFETY:` line saying why it holds.
5. **New dependencies** need one line of justification in the PR description.

## Working on it

The minimum Rust version is **1.89** (`russh`, the netbox's SSH library, needs it); stable is
what CI uses.

```sh
cargo test --workspace          # everything, including the render snapshots
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -- /etc /tmp          # two panels, left and right
cargo test -p fcmd-net          # includes an SSH server started in the test process
```

Useful non-interactive modes while developing:

```sh
cargo run -- --check-keymap     # validate and print the resolved bindings
cargo run -- --list-commands    # the command vocabulary
cargo run -- --list-themes      # built-in + external themes, with their source
cargo run -- --export-theme tron    # a [[theme]] block to paste into fcmd.toml
cargo run -- --paths                # config file, state file, plugins directory
cargo run -- --dump-config      # the effective configuration
```

## What a change needs

- A behaviour change needs a test. UI changes get a **render snapshot** test (build a `Buffer`,
  draw into it, assert on `to_text()`); key handling gets a table-driven test; anything touching
  the filesystem gets a `tempfile`-based test, including the failure path. A test must never
  write out a path: the same `VPath` renders with a backslash on Windows.
- One concern at a time. A line from the backlog is a good size.
- User-visible strings go through `fcmd-i18n` (`crates/fcmd-i18n/i18n/<lang>/fc.ftl`) — all seven
  languages, because the test suite fails on a message id that is missing from one of them.
- Nothing may block the UI thread: filesystem and network work goes through a request to the
  runner, never a blocking call inside a key handler.
- A feature is not finished until the spec, `docs/02-features.md`, the help chapter and
  `CHANGELOG.md` agree with the code.

## Translations

Add or fix a language in `crates/fcmd-i18n/i18n/<tag>/fc.ftl`. The test suite fails if any
catalogue is missing a message id, so a half-finished translation cannot ship silently.

## Cutting a release

**No tag is pushed by hand.** `.github/workflows/tag.yml` watches `main` for a version bump,
waits for CI to be green on that very commit, and creates the tag itself; a tag pushed by hand
skips that check, which is the one thing it exists for.

1. Put the changes under a new, dated heading in `CHANGELOG.md` (`## [0.10.0] — 2026-10-28`):
   the release notes are read from that section, so what is written there is what people see.
2. Bump `version` in the workspace `Cargo.toml` (one place: every crate inherits it), then
   `cargo build` so `Cargo.lock` follows.
3. Regenerate the manual page — `./target/debug/fcmd --man > docs/fcmd.1` — commit, and push
   to `main`. `tag.yml` notices the bump and the dated heading and takes it from there; if CI
   is not green within its window it says so and refuses to tag, which is the correct outcome.
4. `.github/workflows/release.yml` then builds `linux-x86_64`, `linux-x86_64-static` (musl),
   `macos-universal` (Intel + Apple Silicon in one binary) and `windows-x86_64`, runs
   `fcmd --version` on each, and publishes the four `FinalCommander-*` archives with
   `SHA256SUMS`. The publish step needs the workflow token to be allowed to write:
   Settings → Actions → General → Workflow permissions → *Read and write*.

Run `release.yml` by hand from the Actions tab (`Run workflow`) to build the artifacts without
publishing anything — useful after touching the workflow itself. The site is the same shape:
`pages.yml` builds on every push and publishes only through `workflow_dispatch` with
`publish: yes`.
