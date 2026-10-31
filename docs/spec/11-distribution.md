# SPEC 11 — How the program reaches a machine

Every other spec says what the program does once it is running. This one says how it gets
there. It exists because the first thing anybody asks about a file manager they have not heard
of is not what it does — it is `apt install` *what?*, and the answer today is "download an
archive from a web page", which is the answer that loses the argument.

**Status: a proposal.** §2 is what exists. Everything from §3 on is written to be decided
against, not built from. Each channel is costed in three columns that matter more than the
feature list: what it costs once, what it costs *at every release forever*, and what breaks if
nobody keeps it up.

## 1. The promise that constrains all of it

SPEC 00 §3 lists, among the non-goals, a promise that is the whole pitch:

> **No installer and no asset tree.** Nothing is read from `/usr/share`, no theme or language
> files are deployed next to the binary, and a missing config file is a normal state, not a
> degraded one. A release is one executable (plus, if you want them, plugins in one folder).

A reader could think packaging contradicts that. It does not, and the distinction is worth
stating once because every section below depends on it:

- **What the file is** — one executable, self-contained, which runs from a USB stick, from
  `/tmp`, from a home directory on a machine where one has no privileges at all. This does not
  change. Ever.
- **How the file arrives** — a browser, `curl`, `scp`, or a package manager. This is free to
  change, and today it is poorer than it needs to be.

So the rule for every channel: **a package may deliver the binary, the man page and the
licence, and nothing else.** A package that creates `/usr/share/fcmd/themes` has broken the
promise even if the program still runs; the program must behave identically whether it was
installed by a package manager or copied by hand, and it must keep running after the package
is removed, if the file was copied elsewhere.

That rule also decides a question that comes up with every distribution: **no post-install
script.** Nothing to run, nothing to configure, nothing to register.

## 2. What exists today

Roadmap item 18, shipped. A tag builds four archives and publishes them:

| Archive | What is in it |
| --- | --- |
| `FinalCommander-linux-x86_64-<v>.tar.gz` | glibc build |
| `FinalCommander-linux-x86_64-static-<v>.tar.gz` | musl, static — runs on any distribution |
| `FinalCommander-macos-universal-<v>.tar.gz` | one binary for Intel and Apple Silicon |
| `FinalCommander-windows-x86_64-<v>.zip` | msvc build |
| `SHA256SUMS` | a checksum per archive |

The executable sits at the root of the archive, so unpacking leaves `fcmd` where one is rather
than one directory down. There is no installer and nothing asks for privileges.

What is *not* there: any way to say "install it" in one line, any way for a machine to learn
that a new version exists, and any signature beyond the checksum file (§6).

## 3. The channels

### 3.1 `cargo install fcmd` — and the nineteen crates behind it

**What it gives.** One line for anybody who has Rust, and a presence on crates.io, which is
where a Rust programmer looks first. The name is free: `fcmd` and `finalcommander` were both
unclaimed when this was written (checked 2026-10-31).

**What it costs, and this is the part that is not obvious.** `crates/fcmd` depends on eighteen
sibling crates, and only `fcmd-site` is marked `publish = false`. Publishing the binary means
**publishing nineteen crates**, because cargo refuses to publish a crate whose dependency is a
bare path. The root `Cargo.toml` today says:

```toml
fcmd-arc = { path = "crates/fcmd-arc" }
```

with a comment above it stating the convention: a crate is named by `{ workspace = true }` and
never "by spelling out a path and a version of its own". Publishing requires exactly that —
`{ path = "…", version = "0.27.0" }` on all eighteen — so the first cost of this channel is
**reversing a documented convention**, and the second is that every release publishes nineteen
crates in dependency order, each one irrevocable: a version on crates.io can be yanked but
never deleted or replaced.

It also takes nineteen names on a shared namespace forever, for libraries nobody outside this
repository has a reason to depend on.

**The alternative worth weighing.** Mark all eighteen `publish = false` and publish nothing —
`cargo install --git` already works and needs no namespace at all:

```
cargo install --git https://github.com/matteobaccan/FinalCommander fcmd
```

It is longer to type and invisible on crates.io. That is the whole trade.

**What it does not give, either way.** `cargo install` compiles from source: it wants a
toolchain and several minutes, which is the opposite of "copy one file onto a machine". It
also installs the binary and nothing else — no man page. It is a channel for programmers, not
the answer to `apt install what?`.

### 3.2 `.deb` and `.rpm` as release assets

**What it gives.** `dpkg -i` / `rpm -i` on a machine that has no tap, no AUR, no Nix — which
is most servers. It is the cheapest honest answer to the question that opens this spec.

**How.** `cargo-deb` (MIT, 2.4M downloads) and `cargo-generate-rpm` (MIT, 409k) read what is
already in `Cargo.toml` and need a handful of metadata lines. Two steps in `release.yml` on
the Linux job, two more artifacts beside the archives.

**What it costs per release.** Nothing: the tag already builds the Linux binary, and these
package the file that was just built.

**A note on the rules.** Neither tool is a dependency of the program — they run in CI and
nothing of theirs ends up in the executable. So no `THIRD-PARTY.md` entry and nothing for
`cargo deny` to judge. This is true of every tool in this spec and is the reason packaging is
cheaper here than it looks.

**What goes in.** `/usr/bin/fcmd` and `/usr/share/man/man1/fcmd.1.gz`. Nothing else, per §1.
The package must depend on nothing — the musl build has no dependencies at all and is the
honest thing to package, even on Debian.

**What it does not give.** Discovery. Nobody finds a `.deb` on a releases page; they find it
because they already came here.

### 3.3 Homebrew

**What it gives.** `brew install fcmd` on macOS and on Linux, and this is how a macOS user
actually installs anything.

**Two routes.** A **tap** of one's own (`matteobaccan/tap`) is a second repository with one
Ruby file, under our control, available the day it exists — the cost is the user typing
`brew tap matteobaccan/tap` once. **Homebrew core** needs no tap but has rules to pass
(notability, a stable release history) and a review, and its formula is then maintained by
people who are not us.

**What it costs per release.** The formula names a version and a checksum, so it must be
updated at every release. This is a job for the release workflow, not a human: the tag pushes
a commit to the tap. Doing it by hand is how a tap ends up two versions behind.

### 3.4 Scoop, and WinGet

**Scoop** is a JSON manifest in a bucket, the Windows analogue of a tap, and the same shape of
work: our own bucket is ours and immediate; the official `extras` bucket wants a pull request.

**WinGet** reaches further — it is in Windows itself — and costs a pull request to
`microsoft/winget-pkgs` per release, with a manifest and a review. It is the only channel here
that expects something from Microsoft at every version.

Both want the `.zip` that already exists and its checksum, so there is nothing new to build.

### 3.5 AUR

**What it gives.** `yay -S fcmd` for Arch users, who are disproportionately the people who use
an orthodox file manager.

**What it costs.** A `PKGBUILD` in a git repository AUR hosts, pushed over SSH. Two packages
are conventional: `fcmd-bin` (unpacks the release archive, seconds to install) and `fcmd`
(builds from source). `fcmd-bin` alone is enough and is a dozen lines.

**What it costs per release.** A version and a checksum in the `PKGBUILD`, and a push —
automatable from the tag like the Homebrew tap.

### 3.6 Nix

A derivation in nixpkgs, or a flake in this repository. The flake is cheap and serves the
people who would ask for it; nixpkgs is a pull request and a maintainer commitment.
Low priority, named here so the question has an answer.

### 3.7 What we are deliberately not doing

- **Flatpak and Snap.** They are for graphical applications. Both sandbox the filesystem,
  which for a *file manager* is not a wrinkle but a contradiction, and both would deliver a
  program that cannot see the files it exists to manage without holes punched through the
  sandbox.
- **Debian, Fedora and Ubuntu proper.** Not a technical job: it needs a Debian Developer to
  sponsor the package and months of process, and afterwards the version in the distribution
  lags ours by a release cycle. Worth doing when somebody asks for it, and not before.
- **An MSI or a Windows installer.** An installer for a program that is one file is theatre.
  Scoop and WinGet cover the people who want a command to type.
- **A curl-to-shell installer.** `curl … | sh` teaches a habit worth not teaching, and the
  thing it would save is one `tar xzf`.

## 4. What a package carries

| File | Where | Note |
| --- | --- | --- |
| `fcmd` | `/usr/bin` | the musl build on Linux: no dependencies to declare |
| `fcmd.1` | `/usr/share/man/man1` | generated by the hidden `--man` flag; `docs/fcmd.1` is in the repository and regenerated at every release |
| `LICENSE` | `/usr/share/doc/fcmd` | MIT |

Nothing else. No configuration file is installed: SPEC 07 §1 has the themes, keymaps, language
catalogues and help compiled in, so the program is fully usable with nothing else on disk, and
a missing config file is a normal state.

**Shell completions are missing and belong here.** There are none today. `clap` generates them
for bash, zsh, fish, PowerShell and Elvish from the parser that already exists, which is a
hidden `--completions <shell>` flag and a few lines; a package is the natural place to install
them, and it is the one thing in this list the program cannot currently produce. Deciding
against completions is fine; shipping packages that are expected to carry them and do not is
the thing to avoid.

## 5. Version checking

Not proposed, and written down so the question is settled rather than reopened: **the program
does not phone home.** No version check, no update notice, no network traffic the user did not
ask for. A package manager is the thing that knows about new versions, which is a further
argument for §3.2 to §3.5 and not for a feature inside the binary.

## 6. Trust

Today a reader gets `SHA256SUMS`, which proves the file was not corrupted in transit and
proves nothing about who built it.

Two steps worth considering, in order:

1. **Signing the checksums.** `minisign` or `cosign` with a key in the repository's secrets.
   Cheap, and it turns "this file is intact" into "this file came from the release workflow".
2. **Reproducible builds.** Rust is close to reproducible but not free of it; this is a real
   piece of work, worth it only if somebody is actually verifying.

macOS notarisation and Windows Authenticode both cost money per year and both want an identity
a person owns. Without them macOS warns on first run and SmartScreen may warn on Windows. This
is a real friction and the only one on the list that cannot be solved with code.

## 7. Proposed order, and what has to be decided

| | Channel | Why there | Decision needed |
| --- | --- | --- | --- |
| 1 | `.deb` + `.rpm` as assets | cheapest, serves servers, no per-release cost, no account anywhere | none — it can be built |
| 2 | Homebrew tap | the way macOS installs things; a tap is ours and immediate | tap now or wait for core? |
| 3 | AUR `fcmd-bin` | the audience most likely to want this program | who holds the AUR account |
| 4 | Scoop bucket | symmetry on Windows | our bucket or a PR to `extras` |
| 5 | crates.io | visible to Rust programmers | **nineteen crates and a reversed convention** (§3.1), or `--git` and nothing published |
| 6 | Signing | turns the checksums into provenance | whether anybody is verifying |
| 7 | WinGet, Nix, distributions | each reaches further and costs more per release | on request |

Three questions are the author's and not mine to answer: **which accounts exist** (crates.io,
AUR, a tap repository), **whether crates.io is worth nineteen permanent names**, and **how much
per-release automation is acceptable** — every channel from 2 on is a file somewhere that says
a version, and a channel nobody updates is worse than a channel that does not exist, because a
stale formula installs an old program and says nothing is wrong.
