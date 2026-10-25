# Contributing

Thanks for looking. M1 shipped and the work is in **M2** (see `docs/spec/09-roadmap.md`):
the program is a daily driver, and what is left is parity with FAR and Midnight Commander
feature by feature.

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
4. **No `unsafe`** outside `fcmd-platform` (which does not exist yet).
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

## What a PR needs

- A behaviour change needs a test. UI changes get a **render snapshot** test (build a `Buffer`,
  draw into it, assert on `to_text()`); key handling gets a table-driven test; anything touching
  the filesystem gets a `tempfile`-based test, including the failure path.
- Keep the diff to one concern. A chunk from the roadmap table is a good PR size.
- User-visible strings go through `fcmd-i18n` (`crates/fcmd-i18n/i18n/<lang>/fc.ftl`) — all seven
  languages, or English plus a note in the PR so a translator can finish it.
- Nothing may block the UI thread: filesystem and network work goes through a request to the
  runner, never a blocking call inside a key handler.

## Translations

Add or fix a language in `crates/fcmd-i18n/i18n/<tag>/fc.ftl`. The test suite fails if any
catalogue is missing a message id, so a half-finished translation cannot ship silently.

## Cutting a release

1. Put the changes under a new heading in `CHANGELOG.md` (`## [0.0.2] — YYYY-MM-DD`): the
   release notes are read from that section, so what is written there is what people see.
2. Bump `version` in the workspace `Cargo.toml` (one place: every crate inherits it), then
   `cargo build` so `Cargo.lock` follows, and commit.
3. Tag and push: `git tag -a v0.0.2 -m "0.0.2" && git push origin v0.0.2`. Creating the
   release from the web interface works too — the workflow fills in the archives either way.
   The publish step needs the workflow token to be allowed to write: Settings → Actions →
   General → Workflow permissions → *Read and write*.
4. `.github/workflows/release.yml` builds `linux-x86_64`, `linux-x86_64-static` (musl),
   `macos-universal` (Intel + Apple Silicon in one binary) and `windows-x86_64`, runs
   `fcmd --version` on each, and publishes the archives with `SHA256SUMS`.

Run the workflow by hand from the Actions tab (`Run workflow`) to build the artifacts without
publishing anything — useful after touching the workflow itself.
