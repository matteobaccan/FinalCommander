# Contributing

Thanks for looking. The project is in milestone **M1** (see `docs/spec/09-roadmap.md`): the
specs are written, the skeleton runs, and most features are still ahead.

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
4. **No `unsafe`** outside `sc-platform` (which does not exist yet).
5. **New dependencies** need one line of justification in the PR description.

## Working on it

```sh
cargo test --workspace          # everything, including the render snapshots
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -- /etc /tmp          # two panels, left and right
```

Useful non-interactive modes while developing:

```sh
cargo run -- --check-keymap     # validate and print the resolved bindings
cargo run -- --list-commands    # the command vocabulary
cargo run -- --list-themes      # built-in + external themes, with their source
cargo run -- --export-theme tron > ~/.config/sc/themes/mine.toml
cargo run -- --dump-config      # the effective configuration
```

## What a PR needs

- A behaviour change needs a test. UI changes get a **render snapshot** test (build a `Buffer`,
  draw into it, assert on `to_text()`); key handling gets a table-driven test; anything touching
  the filesystem gets a `tempfile`-based test, including the failure path.
- Keep the diff to one concern. A chunk from the roadmap table is a good PR size.
- User-visible strings go through `sc-i18n` (`crates/sc-i18n/i18n/<lang>/sc.ftl`) — all seven
  languages, or English plus a note in the PR so a translator can finish it.
- Nothing may block the UI thread: filesystem and network work goes through a request to the
  runner, never a blocking call inside a key handler.

## Translations

Add or fix a language in `crates/sc-i18n/i18n/<tag>/sc.ftl`. The test suite fails if any
catalogue is missing a message id, so a half-finished translation cannot ship silently.
