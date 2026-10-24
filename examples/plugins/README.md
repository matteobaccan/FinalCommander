# Example plugins

A plugin is a folder with a `plugin.toml` in it, put in the plugins directory —
`scmd --paths` says where that is, and `scmd --plugins <dir>` points somewhere else for a
try. `scmd --list-plugins` says what was found and what each one brings. `F11` in the panels
lists the commands they provide.

```sh
dir=$(scmd --paths | awk '/^macros|^plugins/ {print $2}' | tail -1)
cp -r examples/plugins/git-here "$dir"/
chmod +x "$dir"/git-here/*.sh      # the host runs it, so it has to be executable
```

Four of them, one per action of the protocol, each short enough to read in a minute:

| Folder | Command | What it shows |
| --- | --- | --- |
| `git-here` | Git: what has changed here | `status`, and a program for each system |
| `git-select` | Git: select the files that changed | `select` — the panel's selection, set from what only git knows |
| `dated-folder` | Make a folder named today, and go into it | `run` and `chdir` together |
| `big-here` | What is taking up the room here | `message`, and a timeout longer than the default |

## What a plugin command is

A **process**, not code inside the program. That is why one artifact works on every system,
why there is no sandbox to get wrong, and why a plugin that crashes crashes alone. The cost
is a process per invocation, which for "which branch is this" is nothing.

The conversation is short:

- the context arrives as **JSON on standard input** — `cat >/dev/null` at the top of each
  script is there so the program writing it does not get a broken pipe — and the same facts
  arrive in the **environment**, which is what a shell script actually wants:
  `SCMD_DIR` is the panel's directory, `SCMD_CWD` the path as the program spells it,
  `SCMD_CURSOR` the row under the cursor, `SCMD_SELECTED` the selected names one per line,
  `SCMD_SELECTED_COUNT` how many;
- what the plugin writes back is **one action per line**:
  `{"status": "…"}` or `{"error": "…"}` for the status bar, `{"run": "panel.reread"}` to run a
  command of the program's own vocabulary, `{"chdir": {"path": "…", "panel": "other"}}`,
  `{"message": {"title": "…", "lines": [...]}}`, `{"cursor": "name"}`,
  `{"select": ["a", "b"]}`;
- anything else it writes is shown rather than dropped, so a half-written plugin is
  debuggable rather than silent.

`needs` is why `SCMD_DIR` is set at all: nothing is passed that is not asked for, so a plugin
that does not need the selection never sees it. A path inside an archive or on a server has no
directory on this machine, so `SCMD_DIR` is simply absent there rather than holding something
that cannot be opened — which is why every script here checks it before using it.

An action that runs something names a command of **our** vocabulary, the one every key and
macro uses, so everything a plugin can do a key can do too. `scmd --list-commands` prints all
of them.

## Two systems, two programs

A shell script and a `.cmd` are not interchangeable, so a command may name both:

```toml
run = "status.sh"
run_windows = "status.cmd"
```

The host runs `run_windows` on Windows when it is there and `run` everywhere else. Both are
checked at load for leaving the plugin's own folder; only the one this system would run has to
exist, so a plugin written for both loads on either.

`git-here` ships both, and the `.cmd` is the pattern to copy. For something longer than a few
lines, a two-line `.cmd` that calls PowerShell is easier than batch:

```bat
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0status.ps1"
```

> The shell scripts here are run and checked on Linux and macOS. **The `.cmd` is written but
> not run on Windows** — if it misbehaves, that is a bug worth reporting, and the `.sh` beside
> it says what it is meant to do.

## What is refused, and when

At load, not at the first press: a command id outside the `user.` or `plugin.` name space, a
program that leaves the plugin's own folder (by `..`, by an absolute path, or through a
symbolic link), a `needs` nobody can give, a missing label, and a second plugin claiming an id
another already provides. A plugin that would surprise somebody does not load at all, and
`scmd --list-plugins` says why.

`docs/spec/08-plugins.md` §1b is the whole protocol, and §2 and §3 are the tiers above this
one: a plugin that *adds* a panel column, a preview or a filesystem, and one that is asked a
question rather than told to run.
