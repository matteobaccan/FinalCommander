# Plugins

Twenty-one of them, ready to copy. A plugin is a folder with a `plugin.toml` in it, put in the
plugins directory — `fcmd --paths` says where that is, and `fcmd --plugins <dir>` points
somewhere else for a try. `fcmd --list-plugins` says what was found and what each one brings.
`F11` in the panels lists the commands they provide, in the scope you are in.

```sh
dir=$(fcmd --paths | awk '/^plugins/ {print $2}')
mkdir -p "$dir"
cp -r examples/plugins/* "$dir"/
chmod +x "$dir"/*/*.sh        # the host runs them, so they have to be executable
```

## What is here

Where a line says "FAR" or "Total Commander", that is where the idea comes from: these are the
plugins people have been reaching for in those programs for twenty years, written against this
one's plugin tier.

| Plugin | Commands | Where the idea is from |
| --- | --- | --- |
| `filecase` | names to lower, UPPER, Title case | FAR's **FileCase** — the day a camera hands over four hundred `IMG_0001.JPG` |
| `proclist` | what is running; what has this file open | FAR's **ProcList**, plus the question one actually opens it for |
| `duplicates` | which of these are the same file twice | FAR's find-duplicates, Total Commander's duplicate search |
| `checksums` | write a `SHA256SUMS`; check files against one | Total Commander's **create/verify checksums** |
| `unpack-any` | unpack with whatever is installed; test an archive | Total Commander's **packer plugins (WCX)**, for the formats we do not read |
| `open-with` | open as the desktop would; open this folder | Total Commander's **Open with**, FAR's associations |
| `clip-path` | the full path, or the names, to the clipboard | Total Commander's `Ctrl+Shift+C`, FAR's copy-name |
| `line-endings` | to LF, to CRLF, and what do these have | FAR's line-ending converter |
| `touch-now` | time these as now, or as the file under the cursor | the half of FAR's and TC's attributes dialog everybody uses |
| `what-is-this` | what this file really is | Total Commander's **content plugins (WDX)**, as a message rather than columns |
| `big-here` | what is taking up the room here | `du`, ranked, which is the question one has |
| `git-here` | which branch, how many changed | — |
| `git-select` | select what git says changed | — |
| `dated-folder` | make a folder named today and go into it | — |
| `goto-root` | go to the top of this project | the twenty times a day one types `cd ../../..` |
| `dir-sizes` | every folder here, measured and ranked | FAR's `Ctrl+Space`, which measures one at a time — the question is which of the twenty is the big one |
| `line-count` | lines by extension under here | `cloc`, without `cloc`, and without counting `node_modules` |
| `syntax-check` | does this parse? php, python, node, json, ruby, sh, perl | the check one runs before committing, with the interpreter that owns the file |
| `env-diff` | which keys `.env` is missing from `.env.example` | the deploy that fails for one variable nobody copied. Names only, never values |
| `ssh-hosts` | the hosts in `~/.ssh/config`, with user and port | what the rest of the machine already knows, where the netbox can use it |
| `strip-bom` | which files start with a BOM; take it off | the blank line before `<?php`, headers already sent, a JSON no parser reads |

Each one is short enough to read before trusting it, which is the point of shipping them as
scripts rather than as binaries.

## What a plugin command is

A **process**, not code inside the program. That is why one artifact works on every system,
why there is no sandbox to get wrong, and why a plugin that crashes crashes alone. The cost is
a process per invocation, which for "which branch is this" is nothing.

The plugin does its own work — `filecase` renames the files itself, `checksums` writes the
file itself — and the actions are only for driving the program afterwards. That division is
the whole API: the plugin is a program with a working directory, and the actions are what a
keystroke could have done.

- The context arrives as **JSON on standard input** — `cat >/dev/null` at the top of each
  script is there so the program writing it does not get a broken pipe — and the same facts
  arrive in the **environment**, which is what a shell script actually wants:
  `FCMD_DIR` the panel's directory, `FCMD_CWD` the path as the program spells it,
  `FCMD_CURSOR` the row under the cursor, `FCMD_SELECTED` the selected names one per line,
  `FCMD_SELECTED_COUNT` how many, `FCMD_COMMAND` which command was chosen — which is how one
  program answers to three ids, as `filecase` does.
- What it writes back is **one action per line**: `{"status": "…"}` or `{"error": "…"}` for the
  status bar, `{"run": "panel.reread"}` to run a command of the program's own vocabulary,
  `{"chdir": {"path": "…", "panel": "other"}}`, `{"message": {"title": "…", "lines": [...]}}`,
  `{"cursor": "name"}`, `{"select": ["a", "b"]}`.
- Anything else it writes is shown rather than dropped, so a half-written plugin is debuggable
  rather than silent.

`needs` is why `FCMD_DIR` is set at all: nothing is passed that is not asked for, so a plugin
that does not need the selection never sees it. A path inside an archive or on a server has no
directory on this machine, so `FCMD_DIR` is simply absent there rather than holding something
that cannot be opened — which is why every script here checks it before using it.

An action that runs something names a command of **our** vocabulary, the one every key and
macro uses, so everything a plugin can do a key can do too. `fcmd --list-commands` prints all
of them.

## Asking first

The host cannot know what somebody else's program will do, so a command that renames, moves or
writes over files says the sentence the user should read:

```toml
confirm = "This renames files. The case of the names changes; nothing else does."
```

The program shows it, counts what it would act on, and runs nothing until the answer is yes.
A macro's destructive steps are caught by their names; this is the same guardrail where the
names are not ours to read. Every plugin here that changes a file declares one — `filecase`,
`checksums`, `line-endings`, `touch-now`, `unpack-any` — and the ones that only look do not.

## Two systems, two programs

A shell script and a `.cmd` are not interchangeable, so a command may name both:

```toml
run = "status.sh"
run_windows = "status.cmd"
```

The host runs `run_windows` on Windows when it is there and `run` everywhere else. Both are
checked at load for leaving the plugin's own folder; only the one this system would run has to
exist, so a plugin written for both loads on either.

For something longer than a few lines, a two-line `.cmd` calling PowerShell beats batch:

```bat
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0case.ps1"
```

`filecase`, `git-here`, `proclist`, `open-with` and `clip-path` ship both halves.

> The shell scripts here were each run against real directories on Linux. **The Windows halves
> are written and reviewed but not run** — there is no Windows where this was built. If one
> misbehaves that is a bug worth reporting, and the `.sh` beside it says what it is meant to do.

## When a tool is missing

Several of these lean on something that may not be installed: `7z` or `unar` for the formats we
do not read, `xdg-open` for the desktop, `xclip` or `pbcopy` for the clipboard, `lsof` for what
has a file open, `identify` or `ffprobe` for what a media file is. Each says which program it
wanted rather than failing quietly, because "nothing happened" is the worst possible answer.

## What is refused, and when

At load, not at the first press: a command id outside the `user.` or `plugin.` name space, a
program that leaves the plugin's own folder (by `..`, by an absolute path, or through a
symbolic link), a `needs` nobody can give, a missing label, and a second plugin claiming an id
another already provides. A plugin that would surprise somebody does not load at all, and
`fcmd --list-plugins` says why.

`docs/spec/08-plugins.md` §1b is the whole protocol, and §2 and §3 are the tiers above this
one: a plugin that *adds* a panel column, a preview or a filesystem, and one that is asked a
question rather than told to run. `what-is-this` is the clearest argument for the first of
those — it says in a message what Total Commander puts in columns.

## One shell trap, written down because it cost a build

`sh -n` on your machine is not `sh -n` on everyone's. macOS ships **bash 3.2** — it has since
2007, when bash changed licence — and in that version a `case` statement written **inside a
`$( … )`** does not parse: the `)` closing the first pattern closes the substitution instead,
and the error arrives at the `;;` on a line that looks perfectly fine.

Three of the scripts here had one. The fix is either to move the `case` into a function defined
outside the substitution and call it inside, as `what-is-this` and `line-endings` now do, or to
write the condition as a test — `[ "${path#*/}" = "$path" ]` instead of a `*/*)` pattern, as
`git-select` now does. A test in the repository reads the scripts and refuses a `case` inside a
`$( … )`, because a parse check on a newer shell cannot see it.

## Where the rest of the ideas are

[plugring.farmanager.com](https://plugring.farmanager.com) is twenty years of FAR plugins, and
FAR's own are in its source tree under `plugins/`. Reading them is worth an afternoon, with one
thing to know before starting: about half of them are *editor* plugins — **Align**, **AutoWrap**,
**Brackets**, **DrawLine**, **EditCase** — and this program's plugin tier cannot write them. A
plugin here is a process that is told about the panels and answers with actions on the panels;
it never runs inside the editor's buffer. The panel-side ones are the ones to steal from, and
several already have better answers built in rather than bolted on: **Compare** is `F9 →
Commands` (by name, size and time, and by content), **TmpPanel** is the list panel, **SameFolder**
is `Ctrl+=`, **ProcList** and **FileCase** are in this folder.
