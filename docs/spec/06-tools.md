# SPEC 06 — Tools, shell integration, dialogs

## 1. Find files (`Alt+F7`)

Dialog fields: name masks (include/exclude, comma-separated, `*`/`?`/`**`), content (plain /
regex / hex, case, whole word, encodings to try), size range, date range (absolute or relative
`<7d`), attributes/permissions predicate, depth limit, follow symlinks, search in archives,
search in the selected items only, use the predicate language (SPEC 02 §5) for anything else.

Execution: a parallel walker (`ignore`-crate-style, respecting `.gitignore` optionally) on the
rayon pool with a bounded result channel; results stream into a list that is usable while the
search runs. Actions on results: view, edit, go to the file in the panel, **panelize**
(turn the results into a panel — mc's feature), copy the list, run a job on the whole result
set, save/load the search.

`tool.find.folder` is the same dialog restricted to directories, with a "go to" action.

### 1b. What shipped first, and why in this shape

The dialog above is the whole of it; the first version is the half that answers the question
people actually ask, with the other half named in `docs/BACKLOG.md` rather than half-built.

**In 0.0.4**: masks in the filter grammar of SPEC 02 §5 (lists, `*`, `?`, ranges, `/regex/i`,
exclusions after `|`, groups like `<arc>`), content as text or as a hex pattern, case
sensitivity, whole words, **files that do *not* contain it**, five scopes (this folder; this
folder and below; the folders selected in the panel; from the root of the drive; the other
panel and below), following symbolic links, and directories in the results. The engine is its
own crate, `fcmd-find`, so the dialog does not know how a walk works and the walk does not know
what a panel is: the dialog produces a plain `FindSpec`, and one place turns that into the
engine's query.

**Results stream.** A result is useful before the search has finished, so the list grows under
the cursor while the walk is still going; the counters say how many files and directories have
been looked at, and the row at the foot says where it is. `Enter` goes to the file (the panel
changes directory and the cursor lands on it), `F3` reads it without leaving the list, `F5`
**panelises** — the results become the panel, which is what makes every other key in the
program work on them — and `Esc` stops the search while keeping what it found, with a second
`Esc` closing the list. Nothing is thrown away by a single keystroke.

**The size condition** came next, because the question it answers is asked often and by
itself: where did the space go. The row takes the size grammar of SPEC 02 §5 — `100M` for a
lower bound, `1M-1G` for a range, `-4K` for an upper one — and is checked when the dialog is
accepted, so a typo comes back on the dialog with the field still filled in rather than as a
search that finds nothing. The results frame repeats it next to the mask. The engine needed
nothing: the query has carried a whole `Filter` since the start, and this fills in one more of
its fields.

**What is deliberately not in the first versions**: searching inside archives (the archive
provider comes first), the date and attribute conditions (the filter object carries them
already — the dialog does not offer them yet), saving and loading a search, running a job over
the whole result set, and `tool.find.folder`. Each is a line in the backlog.

**Problems are collected, not announced.** A search over a whole drive meets unreadable
directories by the dozen; saying so one at a time would bury the results. They are counted and
kept, and the list can show them on request.

## 2. Grep with jump-to-match

`tool.grep` is a content-first search: pattern, file filter, context lines. Results are grouped
by file with line numbers and the matching line; `Enter` opens the editor at that exact
line:column; `Alt+Enter` opens the viewer there. The result set can be panelized or exported.

## 3. Disk usage (`Alt+Shift+S`)

Recursive size scan of the current directory with live updates: sortable table (size, items,
%, bar), drill-down with `Enter`, parent with `Backspace`, "show file counts vs bytes", apparent
vs allocated size, hardlink-aware dedup, and actions (delete, move, open in panel). Cancellable;
results cached per path+generation.

## 4. User menu (`F2`)

**Shipped.** `fcmd-menu.toml` beside the configuration — `fcmd --paths` prints where — and
`.fcmd-menu.toml` in a directory for a menu that travels with a project, merged above the
global one. A row is a label, the key that takes it, and the command lines it runs:

```toml
[[menu]]
key = "c"
label = "Build (cargo)"
when = "panel.has('Cargo.toml')"
run = ["cargo build --release"]

[[menu]]
key = "g"
label = "Git"
submenu = [
  { key = "s", label = "status", run = ["git status -sb"] },
  { key = "l", label = "log",    run = ["git log --oneline -40"] },
]
```

Three things settled differently from this spec's first sketch, and the code is right in each.

The file is `fcmd-menu.toml` and not `menu.toml`, because the three files beside it are already
`fcmd.toml`, `fcmd-state.toml` and `fcmd-macros.toml`, and one of four named differently is one
too many to remember.

A `run` entry is **a command line**, not a `shell.run('…', wait=true)` call. The command line
already takes substitutions (§5b), internal commands and prefixes, so a menu row written this
way gains whatever the command line gains, and there is no second dialect — which is also what
FAR's own `FarMenu.ini` holds.

The lines of one row run **in order, stopping at the first failure**, one at a time rather than
at once. Who owns the next step is the rule that makes that work: a line that started a shell
owns it and the rest waits for that shell to report; a line that asked a question (`{ask:…}`)
owns it, because the answer is what starts the shell; a line that was internal (`cd /tmp`) is
finished when it returns, so the next one goes at once; and a line that put something else on
the screen — the viewer, the editor — ends the row, because commands running behind what the
user is now looking at is the opposite of what the screen says.

`when` is deliberately tiny: four facts — `panel.has('<mask>')` (the mask grammar of §5),
`file.is('<mask>')`, `panel.selected`, `panel.local` — with `!`, `&&` and `||`, and `&&`
binding tighter. There are no parentheses: a condition that needs them belongs in a script. A
condition it cannot read is refused **by name**, and the row stays on screen with the reason on
the status line — an entry that vanished over a typo in its own `when` would be impossible to
work out.

A submenu row says it is one (`▸`), and `Esc` inside it goes back to the menu it was opened
from; the second `Esc` closes the menu. Still to come: reading FAR's `FarMenu.ini` as it
stands, separators between groups, a dialog to edit the entries from inside the program, and
FAR's `Shift+F10` for "the item I took last".

## 5. File associations (`[[assoc]]`)

Per mask: `open` (what `Enter` runs), `view` and `edit` (what `Alt+F3` and `Alt+F4` run), and
an optional `name` for the *Open with* list. They are `[[assoc]]` blocks in `fcmd.toml` rather
than a file of their own — this spec first said `assoc.toml`, and one file for four blocks is
one file too many when `fcmd.toml` is already open in front of the user.

```toml
[[assoc]]
match = "*.md"
name  = "Markdown"
open  = "glow -p {file}"
view  = "bat --style=plain {file}"
edit  = "hx {file}"
```

The mask is a whole name (`Makefile`) or `*.` and an extension (`*.md`), compared without
regard to case; the first matching block wins. The rule is `fcmd_config::mask_matches`, shared
with the decompile rules so the two cannot drift apart.

Macros available in commands: `{file}` (the path, **in quotes**, because a name with a space in
it has to be one argument) and `{file.raw}` for the bare path, `{name}`, `{stem}`, `{ext}`,
`{dir}`, `{other.dir}`, `{other.file}`, `{left.dir}`, `{right.dir}`, `{selection}`, `{list}` (a
temp file holding the selection), and `{ask:…}`. This spec first said `{file.q}` for the quoted
one and the code settled the other way round — quoted by default, since that is what nine
commands in ten want — and the code is right: a substitution that needs to be remembered to be
safe is the wrong default.

`F3` and `F4` stay the internal viewer and editor whatever a block says, which is FAR's
arrangement and the reason the alternative keys exist; `Alt+F3` and `Alt+F4` take what a block
names and otherwise `$PAGER` and `$VISUAL`/`$EDITOR`. With nothing to go on at all, `Enter`
hands the file to the desktop opener (`xdg-open`, `open`, `cmd /c start`), which
`general.desktop_open` turns off. Deriving the defaults from the OS (xdg-mime,
`LSCopyDefaultRoleHandler`, the registry) is still to come; the desktop opener is the stand-in
and it is the same answer for most files.

### 5b. Substitutions, and FAR's spelling of them

The same substitution table serves the user menu, the associations and apply-command. Ours is
named (`{file}`, `{file.raw}`, `{list}`, `{other.dir}`); FAR's is punctuation, and it is what a
migrating user's `FarMenu.ini` and associations are written in, so we accept it too and document
the mapping (`docs/04-far-inventory.md` §8):

| FAR | Means | Ours |
| --- | --- | --- |
| `!!` | a literal `!` | `{{` |
| `!` / `!.!` | name without / with extension | `{name}` / `{file}` |
| `` !` `` | extension alone | `{ext}` |
| `!@!` | a temporary file holding the list of selected names | `{list}` |
| `!&` | the selected names inline | `{selection}` |
| `!\` | the current path | `{dir}` |
| `!=\` | the current path with symlinks resolved | `{dir.real}` |
| `!?!` | the description of the current file | `{descr}` |
| `!?<title>?<init>!` | **ask the user**, with `$<history>$` for a named history | `{ask:title=…,init=…,history=…}` |
| `!` `!^` `![` `!]` | switch every following symbol to the passive / active / left / right panel | `{right.file}` … and a `{panel:left}` prefix form |
| modifiers on `!@!`: `Q` `S` `F` `A` `U` `W` | quote · forward slashes · full paths · ANSI · UTF-8 · UTF-16 | `{list.q}`, `{list.slash}`, `{list.full}`, `{list:enc=utf8}` |

Two things in that table are features, not spellings, and we do not have them: the **interactive
prompt** (a command that asks for a value before it runs, optionally remembering what was typed
in a named history) and the **panel prefix** that makes a whole command line refer to the other
panel. Both are in `docs/BACKLOG.md`.

## 6. Histories

Command, directory, view/edit, search, rename pattern, and mask histories: persisted, size-capped,
searchable (type to filter), with pinning (FAR's `Ins` to lock an entry so it is not evicted) and
per-history clearing. Stored as JSONL under `$STATE/history/`.

## 7. Shell integration

| Feature | Design |
| --- | --- |
| Command line | always available under the panels; `Enter` runs in the integrated shell, `Shift+Enter` in a new OS terminal, `Ctrl+Alt+Enter` elevated |
| Macros | `!`/`%`-style substitutions documented in one table: current name, full path, other panel, selection list, cwd — expanded *before* the shell sees them, with quoting per platform |
| User screen | `Ctrl+O` takes both panels off the screen and gives it to what the commands printed, FAR's "Panels on/off": the command line, the menus and every key keep working, and the same key brings the panels back. `PgUp`/`PgDn` turn pages through the output, which is capped in lines. Searching it (`Ctrl+Shift+F`) is still to come |
| Subshell | a persistent PTY (`portable-pty`) running `$SHELL` (or `cmd`/`pwsh` on Windows) with **bidirectional cwd sync**: we inject a prompt hook (`PROMPT_COMMAND`/`precmd`/`PS1` OSC 7) so a `cd` in the shell moves the panel, and `panel.cd` sends a `cd` to the shell |
| Terminal pane | **Implemented** (0.4.0) as `fcmd-term`, on `Ctrl+Alt+E`: a pseudo-terminal (`openpty` on Unix, ConPTY on Windows) and a VT parser into a grid of the same cells the panels use. The alternate screen, scrolling regions, 256-colour and 24-bit colour, the title, autowrap and save/restore are there, so `vim`, `top` and `ssh` run inside it and leave the shell's output alone when they exit. It is a screen like the viewer: parked with the program running, listed by `F12`, and reused rather than duplicated. Every key belongs to the program except the one that got you in, and there is no key bar for the same reason. Still owed: scrollback one can scroll (the lines are kept), searching the output, the mouse, and more than one terminal at a time. |
| Elevation | a privileged helper process (`fcmd-helper`) started on demand via `pkexec`/`sudo`/UAC, speaking a minimal JSON protocol over a pipe, used only for explicitly elevated jobs; never started implicitly |
| Open with OS | `tool.os.open` (xdg-open/open/ShellExecute), `tool.os.terminal`, `tool.os.filemanager`, `tool.os.properties` |

### 7.1 Commands we run ourselves

A commander that shells out for `cd` cannot keep the panel in step, so some commands are ours.
FAR's list, which is the one users expect, mapped to what we do:

| Command | Behaviour |
| --- | --- |
| `cd` / `chdir [path]` | change the active panel's directory; on a remote panel `cd` moves inside the provider while `chdir` always means the local filesystem. `cd ~` goes to the configured home, `cd -` to the previous directory |
| `<drive>:` | Windows: change the panel's drive |
| `cls` | clear the user screen |
| `set [name=[value]]` | show, set or remove an environment variable for our own child processes |
| `if [not] exist <path> <command>` · `if [not] defined <name> <command>` | nestable, so an association or a menu entry can test before it runs |
| `pushd [path]` / `popd` / `clrd` | a directory stack |
| `title [text]` | fix the window title until told otherwise |
| `chcp [n]` | show or set the code page used for child-process output (Windows) |
| `exit` | leave |

### 7.2 Command prefixes

FAR ships these as a plugin (`farcmds`); for us they are core, because they are the cheapest way
to reach the viewer, the editor and the clipboard from a command line or a macro:

| Prefix | Does |
| --- | --- |
| `view:<file>` | open it in the internal viewer |
| `edit:[line[:col]] <file>` | open it in the internal editor at a position |
| `goto:<path>` | move the panel there and put the cursor on it |
| `whereis:<name>` | find it and go there |
| `clip:<file>` | put its contents on the clipboard (and `clip:` with no argument clears it) |
| `link:<target> <name>` | create a link |
| `run:<file><command>` | run a command and send its output to a file, the screen, or nowhere |

Prefixes work wherever a command line does: the line itself, the user menu, associations,
apply-command and macros.

### 7.3 Apply command (`Ctrl+G`, shipped)

One command line, run once per selected file. Each file takes its turn as "the current file",
so the substitutions of §5 mean one file at a time: `!.!` is that file's name, `!` its name
without the extension, and the selection list is that one file rather than all of them.

Decisions, with the reasoning, because each of them is a question the dialog cannot ask:

- **In sequence, not at once.** A hundred converters started together is not what anybody
  asked for, and the output would interleave into nonsense. The next command starts when the
  one before it ends, and the status line counts through the list.
- **Stop at the first failure.** Pushing ninety-nine more commands over a broken one wastes
  the user's time and buries the error. The status line says how far it got and which command
  failed. FAR asks what to do; a dialog mid-run is the better answer and is a backlog item.
- **A template that asks for a value is refused**, with a message saying why. `!?title?init!`
  asks once; using that one answer for every file would be a different feature, and guessing
  which one the user meant would be worse than saying so.
- **Each command goes through the user screen**, so `Ctrl+O` afterwards shows the whole run,
  command by command, rather than only the last one.

## 8. Dialogs and common UI behaviour

- Every dialog: `Tab`/`Shift+Tab` navigation, `Enter` = default action, `Esc` = cancel, `F1` =
  context help for that dialog, history dropdown on text fields (`Ctrl+↑/↓`), completion for
  paths and masks, tri-state checkboxes, and mouse support.
- Dialogs are movable and resizable where it matters (viewer/editor search, job queue).
- Long operations inside dialogs (listing a remote dir for a path picker) are cancellable.
- The **command palette** (`Ctrl+Shift+P`) fuzzy-searches every command, shows its binding and
  scope, and is the discovery mechanism that replaces reading the manual.
- **Learn keys** dialog: press a key, see what the terminal sent and what `fcmd` resolved it to;
  write a local override.

## 9. Help system

Offline, built from the docs: one page per topic with context links; `F1` opens the page for the
current screen/dialog; full-text search inside help; localisable (fluent + markdown sources
compiled into the binary). Help is generated from `docs/` so it cannot drift from the specs,
and the keymap pages are generated from the live keymap (never hand-written).

## 10. Session & state

`session.toml` holds panels, tabs, paths, view modes, sorts, filters, selection (optional),
viewer/editor positions, and the job journal pointer. `--no-session` starts clean;
`fcmd path1 path2` opens the two panels at those paths; `fcmd --goto file:line` opens the editor.

## 11. The clipboard

The clipboard belongs to the window system, and a program in a terminal is not in the window
system: it is at the far end of a pipe that may well cross a network. So the terminal is asked
to do it, with `OSC 52` — `ESC ] 52 ; c ; <base64> ESC \`. The terminal that receives it is
the one in front of the user, which is the clipboard they mean; it needs no X11 or Wayland
library, no `pbcopy`, no `unsafe` and no platform code at all, and it works unchanged over
`ssh`, where a clipboard library on the remote machine would reach the wrong clipboard or none.

What this buys, and the keys FAR uses for it:

| Key | What goes on the clipboard |
| --- | --- |
| `Ctrl+Ins` in the panel | the names of what is selected, one per line |
| `Alt+Shift+Ins` | the same, with the whole path on each line |
| `Ctrl+Ins` with something on the command line | that line |
| `Ctrl+Ins` / `Ctrl+X` in the editor | the block, which also stays on the editor's own clipboard |

`ui.clipboard = false` turns it off, for somebody who would rather their terminal were not
asked by a program running on another machine. A terminal that does not understand the request
ignores it, and there is no answer to wait for, so what the program can honestly say is what
it copied — not that the clipboard took it.

**Reading the clipboard is the terminal's job, not ours.** `OSC 52` can ask for the contents
as well, and most terminals refuse for the obvious reason; there is no need to ask, because
the terminal's own paste key hands the program the text as a *bracketed paste*, which arrives
as one piece rather than as a run of keystrokes. A paste is not typing and is not treated as
typing: the editor and a real terminal take the whole of it, newlines and all, while
everything that is one line — the command line, a dialog row, a prompt — takes the first line
and says how many more there were, because a command copied from a web page arrives with its
newline still attached and running it before it can be read is exactly the accident to avoid.
