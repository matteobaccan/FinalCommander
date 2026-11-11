# SPEC 06 — Tools, shell integration, dialogs

## 1. Find files (`Alt+F7`)

Dialog fields: name masks (include/exclude, comma-separated, `*`/`?`/`**`) or an approximate
name (§1d), a condition on the file's metadata (§1d), content (plain /
regex / hex, case, whole word, encodings to try), size range, date range (absolute or relative
`<7d`), attributes/permissions predicate, depth limit, follow symlinks, search in archives,
search in the selected items only, use the predicate language (SPEC 02 §5) for anything else.

Execution: a parallel walker (`ignore`-crate-style, respecting `.gitignore` optionally) on the
rayon pool with a bounded result channel; results stream into a list that is usable while the
search runs. Actions on results: view (`F3`) and edit (`F4`) — both where the text was
found, when the search had text in it: the viewer at that offset, the editor on its line,
the hex editor on its byte; the offset is the one the search reported with the hit and is kept
beside the row — go to the file in the panel, **panelize**
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

**Inside archives** is a box of its own, and off unless it is ticked: a search that opened
every container of a whole drive is a search of a different size. Ticked, a `.zip` on the way
is walked as `x.zip!/` — which is a directory to everything above it, so the depth limit, the
loop guard, the mask, the size condition and the content scanner work in there exactly as they
do on the disk, and a hit comes back as `/home/mb/src.zip!/doc/readme.md`, which `Enter`, `F3`
and *panelise* already understand. The engine gained one question for its filesystem — "may I
step into this file?" — and one `impl` that answers it, so the walk itself did not learn what
an archive is. A nested archive and one that needed a password open the way they do in a
panel, because it is the same container cache. What an entry costs is its bytes: a container
is not a file one can seek about in per entry, so a content search reads one entry at a time
and refuses one over 64 MB with a problem rather than filling the machine.

**What is deliberately not in the first versions**: the date and attribute conditions (the filter object carries them
already — the dialog does not offer them yet) and running a job over the whole result set.
Each is a line in the backlog. Saving and loading a search is §1c.

**Problems are collected, not announced.** A search over a whole drive meets unreadable
directories by the dozen; saying so one at a time would bury the results. They are counted and
kept, and the list can show them on request.

### 1c. Searches kept under a name, and opened as panels

A search one runs every week is a search one should not have to type every week, and the
histories of the dialog's rows (`Ctrl+↓`) remember each row by itself, not the query as a
whole.

**Keeping one.** `F2` in the find dialog asks for a name and keeps every row of the dialog
under it, together with the folders the scope row pointed at when the key was pressed. The
rows are checked first, by the same function `OK` uses, so a search that would be refused is
refused before any name is asked for; the dialog stays open behind the question and comes
back whatever the answer. A name that is already taken replaces the search that had it,
which is how a kept search is changed: load it, change a row, keep it again under the same
name. The searches live in the state file (`[[searches]]`, beside the folder shortcuts and
for the same reason — a key keeps them, and what a key keeps must not wait for *Save
setup*).

**Loading one.** `F9` in the find dialog opens the kept searches over it; `Enter` fills the
dialog in with the one chosen and `Esc` goes back to the dialog as it was typed. Loaded into
the dialog, the scope row means *this* panel again: that is a search to run here.

**As a panel.** `tool.find.saved` — `Alt+Shift+F7`, *Commands → Saved searches*, and a row
of the change-drive dialog once there is a search to show — opens the same list from the
panels. There `Enter` makes the panel the results of the search: a temporary panel like the
one `F5` makes out of a results list, named after the search, whose rows are the real files
wherever they live. The search runs from the folders it was kept with, not from wherever
the panel happens to be, which is what lets it be opened from anywhere and mean the same
files; and it runs again at every re-read (`Ctrl+R`, and after a job that changed
something), so the panel says what matches now and not what matched when the search was
kept. A re-read asked for while the previous walk is still going stops that walk. A search
that cannot start — a mask that no longer parses because a `<group>` it named is gone —
leaves the panel empty and says why on the status line. `F4` in the list puts the search
into the find dialog instead, and `Del` forgets it; a panel still showing a forgotten
search becomes an ordinary list of the files it held.

The list is the same one in both places and tells them apart by where it was opened from:
in the dialog one is filling in a dialog, in the panels one is going somewhere.

### 1d. Approximate names, and what a file says about itself

Two more conditions, each a row of the dialog, each *and*-ed with the rest like the size and
the text.

**Approximate name.** A box under the mask row changes what that row means: letters to
find in order anywhere in a name, rather than a mask. `mnrs` finds `main.rs`; several words
must each match, in any order (`test app` finds `app_tests.rs`); case follows the dialog's
own box; `*` and `?` left over from a search by mask are ignored, so the default `*` still
means every name. It is a subsequence and nothing cleverer — a match can always be
explained by pointing at the letters — and what makes it useful is the order of the
results: letters next to each other beat letters apart, a letter that starts a word (the
start of the name, after `_ - . space`, a capital after a small letter) beats one in the
middle, and of two equal matches the shorter name wins. The matcher is
`fcmd_core::fuzzy`, written here: forty lines of a two-row table did not justify a
dependency. The engine reports each hit with its score (`Hit::score`), and the results
list keeps itself best first as the hits arrive — a hit is inserted after the rows that
match as well or better, so equal matches stay in the order they were found. A cursor
nobody has moved stays on the first row, which is the best match so far; a cursor somebody
moved stays on its row. A kept search opened as a panel hands its rows over best first
too, though the panel's own sort then decides. The frame writes the pattern as `~mnrs`.

**Metadata.** A row that asks about what the file says of itself — the camera of a
photograph, the artist of a song, the author of a document. No parser was written for it:
the viewer's `F11` already reads EXIF and PNG text, ID3, FLAC and Ogg comments, the
properties of PDFs and office documents, the version resource of a program and the rest
into rows of a label and a value (SPEC 05 §1.8), and the condition is a question asked of
those same rows (`fcmd_find::MetaCond`). So what `F11` shows about a file is exactly what
it can be found by, and a format the pane learns is a format the search knows.

The grammar is `field=value`, both matched as substrings without regard to case, against
a row's label and its value: `artist=bowie`. Either side may be left out — `=canon` (or
just `canon`) is "canon" in any row, `gps=` is a file that has a GPS row at all — and
several separated by `;` must all hold. `format` is a row every file the readers know has:
what it is, with its one-line summary (`format=jpeg`, `format=pdf`). A heading of the pane
is not a row, a directory says nothing about itself, and a file the readers do not know
simply does not match — that is most files and not a problem to report; a file that cannot
be opened is one, like any other.

The cost is the reading: the name conditions are checked first, then this, then the text.
A file on the disk is read at the offsets the readers go to (a few kilobytes of a
photograph, the directory at the end of a `.docx`), never whole. An entry of an archive
has no offsets to go to and is read whole, up to the same 64 MB a text search allows.
Only plain files are opened: a pipe or a device would be read for ever.

### 1e. An index as the source

Every system keeps a list of its files somewhere, and asking it is milliseconds where a
walk of the same disk is minutes. A box of the dialog — *Ask the system's index instead of
walking* — makes the search ask one for its candidates.

**A program, by the rule for programs from elsewhere** (SPEC 02 §14, *External
programs*): nothing is linked and nobody's database is read. A program on the `PATH` is
run and what it prints is read — `plocate`, or `locate`, on Linux; Spotlight's `mdfind` on
macOS; `es`, the command line of voidtools' Everything, on Windows. Each is a row of the
`:tools` panel like `7z` and `git`, with its card and, where a package is certain, its
install line. `find.index` in the configuration says which: empty for the system's own,
`off` for none ever (the box then leaves the dialog), or a command line of one's own whose
output is one path per line or NUL-separated, with `{root}` for the folder to look under
and `{name}` for the letters the name must hold — the line is split at spaces before the
placeholders are filled, so a folder with a space in it stays one argument. The program
is looked up by the worker, not when the dialog opens: finding one is a walk of the
`PATH`, and nothing that touches the filesystem runs on the interface's thread.

**Candidates, never results.** An index is as old as its last update and knows a path and
little else. So it is asked as little as possible — the folder, and a *hint*: the longest
run of plain letters in the mask, when the mask is one plain pattern (`*.rs` gives `.rs`;
a list, an exclusion, a `<group>`, a regex or an approximate name give none) — and every
path it prints is then looked at again on the disk and put through the whole query by the
same function a walk's entries go through (`fcmd_find::find_among`, sharing `examine`
with the walk), so the two sources cannot disagree about what matches. That is also where
the scope is enforced: a path outside the folder, or deeper than "this folder" goes, is
dropped whatever the index thinks, a path named twice is one result, and a file the index
still lists but the disk no longer has is left out without being a problem. Hits stream
as the program answers, and `Esc` stops the program as well as the search.

**Optional by construction.** The search walks instead — and the reason is kept with the
problems, so a slow answer explains itself — when there is no such program, when it will
not start, when it ends badly having proposed nothing and said why (`plocate` without a
database, Everything not running), when the folder is on a server or inside an archive,
and when *Inside archives* is ticked, since no index has looked in one. `locate` ending
with 1 and nothing to say is an answer, "nothing found", and is taken as one. When an
index did answer, the frame of the results says which (`· plocate`).

**What cannot be made up for** is what the index has not seen: a file created since its
last update is not proposed and so is not found. The box says so, and it is off until it
is ticked. A kept search remembers the box, and its panel asks the index again at every
re-read.

Tests need none of these programs: the questions each is asked are built by a function
that is tested as text, the answer is parsed from bytes, the conditions run over a plain
list of candidates, and the one test that runs a program runs `cat` (or `cmd /c type`) as
a configured index.

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
from; the second `Esc` closes the menu.

`separator = true` makes a row a rule between groups, FAR's `--`; a `label` on it is drawn
into the rule as a heading, centred. A separator may carry nothing else (`run`, `submenu` and
`key` are refused by name), and after the conditions are applied a rule only stands between
rows that are shown — none at either end, and of two in a row the later one, which is the
heading of what follows.

`Shift+F10` (`tool.usermenu.last`, panels only — the editor's is save-and-quit) is FAR's
"select the last executed menu item": the menu opens, walks down the submenus to the row run
last and puts the cursor on it, without running it. The row is remembered by its labels and
not its position, because the conditions move rows between one opening and the next; a row
that is gone leaves the cursor on the deepest level still found and says so. It is kept for
the session, not across restarts.

Still to come: reading FAR's `FarMenu.ini` as it stands, and a dialog to edit the entries
from inside the program.

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

Command, directory, view/edit, search, rename pattern, and mask histories: persisted,
size-capped, searchable (type to filter), with pinning (FAR's `Ins` to lock an entry so it is
not evicted) and per-history clearing.

**Shipped**: the command history (`Alt+F8`, `Ctrl+E`/`Ctrl+X` on the line, `ui.command_history`
lines kept), the folder history per panel (`Alt+F12`), and — new in 0.7.0 — **the files that
have been read or edited** (`Alt+F11`, `ui.file_history` entries kept).

That last one is written in the one place every viewer and every editor passes through, which
is also what makes it right for a file on a server: the viewer is given the remote path, so
that is what is remembered, while the copy fetched into the program's own temporary directory
is left out — a history of paths that will not exist tomorrow is worse than no history. A file
opened again moves to the top rather than appearing twice, because the list is *where I have
been* and a file read five times is one place. `Enter` on a row reads it and `F4` edits it; a
row whose file has gone says so rather than opening nothing.

All three live in the state file rather than under `$STATE/history/`: one file to back up and
one to delete is the rule the configuration follows (SPEC 07 §1), and a JSONL file per history
would be five more.

**A list one types into, and FAR's three keys.** Each of the three narrows as one types — the
frame says what was typed — and then:

| Key | What it does |
| --- | --- |
| `Ins` | pins the line under the cursor, and unpins it again. A pinned row is marked with a `*` beside the list's own `√` column, and neither the size limit nor the lifetime can push it out |
| `Shift+Del` | takes that one line out of the history, and refuses a pinned one rather than quietly making the pin mean nothing |
| `Del` | empties the history of everything nobody has pinned, after asking: it is one key away from the one above, and only one of the two can be undone by typing the line again |

Those are FAR's own keys in its history window, including FAR's question before the `Del`.

**A lifetime.** `ui.history_lifetime` is how many days a line nobody has pinned is kept after
it was last *used* — FAR's own setting, and FAR's own default of ninety days. A line used
again is as young as the use, so a lifetime counts idle days rather than days since the line
was first typed; `0` keeps everything for ever, and a pinned line is never counted.

The sweep happens once, when the state file is written on the way out: a life is counted in
days and a session in hours, so there is nothing to gain by checking the clock while the
program runs, and that is the one moment where all three histories are in hand. What each line
has to say for itself — when it was last used, whether it is pinned — is a small table beside
the three lists in the state file, so a line nobody has pinned and nobody has used since the
file was written simply has no row there.

**The dialogs' own fields.** A field one types the same thing into again keeps its own
history: the mask and the text of `Alt+F7`, what `Ctrl+F7` replaces and with what, what the
viewer and the editor search for, where `F5` copies to and through which filter, where an
archive is unpacked, and the masks of the colour and sort groups. `Ctrl+↓` on the row opens
what has been typed there before — FAR's own key — as the same list one types into, with the
same three keys; `Enter` puts the row in the field, `Esc` leaves the field as it was, and the
dialog waits behind the list either way. A field keeps sixty-four lines, which is not a
setting: this is a handful of answers one reaches for, not a log.

Nothing feeds these lists by hand. Every dialog's `OK` goes through one door, and that is
where every text row with something in it goes into its field's history — a history that
depended on each dialog remembering to feed it would be a history with holes in it.

## 7. Shell integration

| Feature | Design |
| --- | --- |
| Command line | always available under the panels; `Enter` runs in the integrated shell, `Shift+Enter` in a new OS terminal, `Ctrl+Alt+Enter` elevated |
| Macros | `!`/`%`-style substitutions documented in one table: current name, full path, other panel, selection list, cwd — expanded *before* the shell sees them, with quoting per platform |
| User screen | `Ctrl+O` takes both panels off the screen and gives it to what the commands printed, FAR's "Panels on/off": the command line, the menus and every key keep working, and the same key brings the panels back. `Enter` on an empty line is the exception: with no row on the screen it does nothing, as in FAR, rather than enter the folder or open the file the hidden cursor was left on (issue #29). `PgUp`/`PgDn` turn pages through the output, which is capped in lines. `Ctrl+Shift+F` (`shell.output.search`, 0.31.0) finds text in it, any case: the newest line that holds it is marked and scrolled to the bottom of the screen with the panels off; the same text again goes to the line before, and past the oldest round to the newest, saying so |
| Output as it is printed | A command's output reaches the screen **line by line while it runs**, not in one piece at the end: the worker reads both pipes into one channel and reports whole lines, and the application appends them under the heading it wrote when the command started. So the panels go off by themselves while a command runs, the way the console comes forward in FAR, and come back when it ends — `ui.command_screen = false` leaves the panels alone, and the output still arrives as it is printed, on the screen `Ctrl+O` opens. `Ctrl+O` during a run gives the screen back early and the command carries on behind the panels. A line rewritten in place with a carriage return (a progress bar) is kept as what it ended up saying |
| Subshell | **Implemented** (`fcmd_term::Shell`): one pseudo-terminal running `$SHELL`, started by the first command rather than at startup, with **two-way cwd sync**. The line typed is written into it as typed — no second shell in between, so an alias, a `*` and a `$HOME` mean what that shell says they mean — and a `cd`, an `export` or a function set by one command is still there for the next. The hook is sent when it starts, by shell family: `PS0` + `PROMPT_COMMAND` for bash, `preexec`/`precmd` for zsh, the `fish_preexec`/`fish_postexec` events for fish, and `PS1` alone for a POSIX shell, which has no hook before a command but does expand its prompt afresh every time it draws one. None of them puts the directory *in* the prompt: a prompt is measured by the shell's line editor, which counts an escape sequence it was not told to ignore as letters on the screen, and a prompt measured wrongly is redrawn wrongly — `readline` repeats the character that lands in the last column, which in a path is a letter that was never there. The marks are printed to the terminal and the POSIX prompt expands to nothing. What it emits is what terminals already use: `OSC 133;C` before a command, `OSC 133;D;<status>` and `OSC 7` at the prompt, so a shell whose own configuration emits them is understood anyway. The output is read as *lines* (`fcmd_term::Transcript`) rather than into a grid, because the user screen is a line buffer and this is what feeds it; the prompt and the echo of the command are dropped, so the heading the user screen writes is not doubled. `cmd.exe` has no prompt hook, so it keeps a child per command — half a session, where `cd` works and the status never arrives, would be worse than none. `[term] subshell` turns the whole thing off and `[term] follow_cwd` the following; `Ctrl+C` on the output screen sends the interrupt |
| Terminal pane | **Implemented** (0.4.0) as `fcmd-term`, on `Ctrl+Alt+E`: a pseudo-terminal (`openpty` on Unix, ConPTY on Windows) and a VT parser into a grid of the same cells the panels use. The alternate screen, scrolling regions, 256-colour and 24-bit colour, the title, autowrap and save/restore are there, so `vim`, `top` and `ssh` run inside it and leave the shell's output alone when they exit. It is a screen like the viewer: parked with the program running, listed by `F12`, and reused rather than duplicated. Every key belongs to the program except the one that got you in, and there is no key bar for the same reason. Still owed: scrollback one can scroll (the lines are kept), searching the output, the mouse, and more than one terminal at a time. |
| Learn keys | **Implemented** (`app.learn.keys`, *Options → Learn keys*): a cell for every key worth testing — the function keys with each modifier, the arrows with `Ctrl` and `Shift`, the editing keys — lit as that key arrives, with the last one named beside what it is bound to here. A cell that never lights is a key the terminal is not sending, which is the question this answers and the one no keymap can. Two `Esc` leave, because one is a key under test. Midnight Commander goes on to let one type in the escape sequence a key should have sent; this does not, and will not: the sequences are read by `crossterm`, which carries a table for the terminals in use, and a table typed in by hand would be a worse one |
| Elevation | a privileged helper process (`fcmd-helper`) started on demand via `pkexec`/`sudo`/UAC, speaking a minimal JSON protocol over a pipe, used only for explicitly elevated jobs; never started implicitly |
| Open with OS | `tool.os.open` (xdg-open/open/ShellExecute), `tool.os.terminal`, `tool.os.filemanager`, `tool.os.properties` |

### 7.0b The prompt (`[cmdline] prompt`)

What is written to the left of what one types. It was the panel's path and a `>`; it is now a
format, with FAR's own letters and two of our own:

| In the format | What it writes |
| --- | --- |
| `$p` | the whole path |
| `$w` | its last part, which is what fits on a narrow terminal |
| `$n` | where the path starts: `C:` on Windows, `/` on Unix, `sftp://user@host` on a connection |
| `$r` | ` (branch)` in a git repository, nothing outside one |
| `%GITBRANCH%` | the bare branch name, spelled the way the plugin everybody installs spells it |
| `$g` `$l` `$a` `$b` `$c` `$f` `$q` `$s` | `>` `<` `&` `|` `(` `)` `=` and a space |
| `$d` `$t` | the date and the time |
| `$@` | `[admin]` when the session has more rights than usual |
| `$_` | a new line: what comes before it goes on the rows above the command line, which the panels give up (at most three; FAR's #970) |
| `$h` | a backspace, which takes a decoration back out when what it decorated was empty |
| `$$` | a `$` |

The case of the letter does not matter, and a letter nobody knows is left as it was written so a
mistake in the configuration can be seen. The default is `$p$r$g `, which is FAR's default plus
the branch: the one thing a prompt in a repository is asked for.

The branch is read, not run: `git` need not be installed, and the walk up to the first `.git`
reads one small file (`HEAD`, or the `gitdir:` line of a worktree's `.git` file and then its
`HEAD`). A head that is not on a branch shows the commit, shortened to seven characters. It is
looked up when the panel's directory changes rather than on every frame, and not at all for a
directory on a server or inside an archive, which has no `.git` to read.

### 7.4 Autocomplete (`Ctrl+Space`, and while one types)

Four sources, each one a setting with three states — `on`, `off`, or `asked` for only when
`Ctrl+Space` asks. They are separate because they cost different things:

| Source | What it offers | Default |
| --- | --- | --- |
| `files` | the names in the directory the panel is showing, and under a path one has typed | `on` |
| `history` | the **whole line** again, out of the commands that have run | `on` |
| `path` | the executables on the `PATH` | `asked` |
| `env` | the environment's variable names, for a word that starts with `$` or `%` | `on` |

The panel's own names are free: that listing is already on screen, and reading it again per
keystroke would be a filesystem call inside a key handler. The `PATH` is not free — gathering
it is a read of every directory on it — so it is gathered once, the first time something asks,
and that is why it waits to be asked by default. The one filesystem call the completion makes
is for a word with a separator in it (`vim src/ma`): one `read_dir` of a local directory
somebody has just named, with a cap on how much of it is read, because a worker round trip per
keystroke would make the list arrive after the next letter.

The first word of a line is a command and the rest are files, so the first word is completed
from what the program runs itself, the prefixes (§7.1, §7.2) and the `PATH`, and a later word
from the names around it. The history completes the whole line rather than a word, which is
what makes it worth having: a line one has run is a line one may run again.

The list is drawn **above** the command line, over the bottom of the panels — below it are the
status line and the key bar, and a list that covered those would hide the row that says what
the keys do. It owns five keys and no others: `↑`/`↓` walk it, `Tab` takes what is under the
cursor, `→`/`End` take what is drawn after the caret, `Esc` closes it. `Enter` runs the line as
it always did **until** somebody has walked into the list, and takes the row after that — a
list that stole `Enter` the moment it appeared would make typing feel like a trap. Every other
key closes the list and goes on to the line. `cmdline.complete.modal = true` gives FAR's
manners instead, where the list takes `Enter` as soon as it appears.

**The best answer while one types**, which is `cmdline.complete.append`:

| | |
| --- | --- |
| `hint` (the default) | drawn after the caret in a dim colour and **not** in the line, so `Enter` runs what was typed. `→` or `End` takes it. This is the habit the shells that grew it after FAR settled on, and it cannot change what a line means |
| `line` | FAR's own: the rest goes into the line, marked out, so the next character replaces it — and `Enter` runs the line with it |
| `off` | neither; the list is the whole of the offer |

Three things have to hold before anything is appended, and each is a case where appending
would be *wrong* rather than merely unhelpful: the caret is at the end of the line, so there is
an end to append to; the answer begins with what is already typed, so nothing typed is changed;
and the answer leaves its caret at its own end, so a row that writes a prefix like `view:` and
puts the caret inside it is not an append. The second is why `Cargo.toml`, which is offered for
`car` because a completion that minded the case would be one nobody uses, is *not* drawn after
`car`: the line would read `cargo.toml` and one would get `Cargo.toml`. `Tab` takes that one.

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
| `ai:[name:] <question>` | ask a language model; the answer opens in the viewer (§14) |
| `sql:<statement>` | query the SQLite database the panel is in or the cursor is on, and view the rows as a table (SPEC 04 §4b) |

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

### 7.6 The insertion keys (`Ctrl+Enter` and its neighbours)

FAR has eleven keys that put something about a panel on the command line. They are two
questions asked in every combination — *what* about the panel, and *which* panel — so here
they are one command with a table behind it rather than eleven arms:

| What | Means |
| --- | --- |
| `name` | the name under the cursor, on its own. A cursor on `..` has no name to give, so the directory is the answer — which is what one wanted the key for anyway |
| `path` | the panel's directory |
| `full` | the directory and the name together: the one a command usually wants |
| `unc` | the full name in the form another machine could use. A remote path already is one; a path on this disk is not, and the key says so rather than inserting a local path under a name that promises more |

| Which | Means |
| --- | --- |
| `active` (the default) | the panel with the focus |
| `passive`, written `.other` | the one without it |
| `left`, `right` | that side of the screen, whatever has the focus |

FAR has both pairs for the reason a script does: `active` follows the focus and `left` does
not, so a line written once can mean either. The id is `cmd.insert.<what>` with the panel in
brackets — `cmd.insert.full(left)` — and the two spellings the keymaps were written with
before there was a table, `cmd.insert.name` and `cmd.insert.path.other`, keep working.

The keys bound by default are FAR's own: `Ctrl+Enter` and `Ctrl+Shift+Enter` for the name,
`Ctrl+[` and `Ctrl+]` for the path, `Ctrl+Shift+[` and `Ctrl+Shift+]` for the left and right
panel's path, `Ctrl+F` for the full name, `Ctrl+Alt+F` for its network form, and
`Ctrl+Alt+[` / `Ctrl+Alt+]` for the left and right panel's. Everything inserted is quoted for
the platform, as the substitutions of §5b are.

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

`fcmd-state.toml` holds panels, tabs, paths, view modes, sorts, filters, selection (optional),
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

### 11b. The screen grabber (`Alt+Ins`)

The keys above copy what the program *knows* — names, paths, a block of a file. The grabber
copies what the program *drew*, which is a different set and the one with no other answer: a
process card, a help page, a hex dump, the foot of a diff, the sentence a comparison ended
with. None of it is a file, so none of it can be copied as one.

The terminal's own selection is the obvious alternative and it is worse at this: it takes the
frame, the column rule and the padding with it, it cannot see the part of a line that is
scrolled sideways, and over `ssh` what it selects is whatever the far end last painted. FAR
has had a grabber since its first version for exactly this reason.

`Alt+Ins`, from any screen, holds the screen as it is — a snapshot, so moving about cannot
change what is being copied, and the program underneath is not running while one reads it:

* the arrows move the cursor, `PgUp`/`PgDn` by a screen, `Home`/`End` to the ends of the row,
  `Ctrl+Home`/`Ctrl+End` to the corners of the screen;
* `Shift` with any of them marks a rectangle, dropping the mark **where the cursor already
  was** rather than where one noticed it, so the rectangle grows from where one started;
* `Ctrl+A` marks the whole screen, for the one case where the rectangle is all of it;
* `Enter` or `Ctrl+Ins` copies and closes; with nothing marked it takes the row the cursor is
  on, because "copy this line" is what one wants from a grabber more often than a rectangle;
* `Esc` closes and copies nothing.

Trailing blanks come off each line — a rectangle round a short line is mostly blanks — and a
blank row inside the rectangle stays, because it was on the screen. A double-width character
is one character and not its two cells.

The frame comes from the loop that draws rather than from the key handler, because that is the
only place it exists: the key asks, and the screen is handed over on the way round. The screen
the grabber was opened on is parked rather than dropped, so closing it comes back to the file
one was reading.

**Reading the clipboard is the terminal's job, not ours.** `OSC 52` can ask for the contents
as well, and most terminals refuse for the obvious reason; there is no need to ask, because
the terminal's own paste key hands the program the text as a *bracketed paste*, which arrives
as one piece rather than as a run of keystrokes. A paste is not typing and is not treated as
typing: the editor and a real terminal take the whole of it, newlines and all, while
everything that is one line — the command line, a dialog row, a prompt — takes the first line
and says how many more there were, because a command copied from a web page arrives with its
newline still attached and running it before it can be read is exactly the accident to avoid.

## 11c. Saved synchronisations (`Ctrl+Alt+Y`)

Synchronising is SPEC 03 §10a; what belongs here is the tool around it. `tool.sync.saved`
opens a pick list of the `[[sync]]` blocks of `fcmd.toml` (SPEC 07 §2b-septies), one row per
block: `name: left → right`, or `↔` for a pair that runs both ways. `Enter` runs the row —
the walk, the table, and `F2` there — `F4` opens the synchronisation dialog filled in from
it, `Del` or `F8` removes it and saves the file. With no block saved the command opens
nothing and the status line says how to save one. The same list is the middle button,
*Saved…*, of the dialog `Ctrl+Shift+Y` opens, which is also where a pair is given its name
and where the file a run reports to is named.

## 12. Disk usage

`Alt+Shift+F3` (`panel.usage`, *Commands → Disk usage*) answers the question a full disk asks:
what in this folder is taking the space. It opens a list over the panel with every entry of the
folder, largest first, each with its size, a bar against the folder's total and its share of
it. Every folder not yet measured is measured — the same asynchronous walk `F3` on one folder
starts, one per folder — and the list fills in as the answers arrive, keeping the row the
cursor is on; a folder still being walked says `…` rather than counting as empty. The sizes go
into the panel too, as `F3`'s do, so sorting the panel by size afterwards agrees with the list.

`Enter` on a folder goes into it, where `Alt+Shift+F3` asks the same question one level down;
on a file it puts the panel's cursor there. The list filters as you type, like every other.
Inside an archive and in a list of found files it is refused: the sizes there are what an index
says, not what a disk holds.

Still to come: a tree of the whole walk at once, rather than one level at a time, and the
treemap some tools draw.

## 13. Duplicate files

FAR's FindDup and Total Commander's *Find duplicate files*, as one command,
`file.dupes` (*Commands → Find duplicate files*). It looks under the panel's folder, or under
what is selected; a file selected is compared as well as the folders.

* **Three passes.** Size, then a SHA-256 of the first 64 KiB, then a SHA-256 of the whole
  file, each over what the one before left standing, so most files are only `stat`ed and most
  of the rest read for 64 KiB. Empty files are left out; links are never followed; a file
  reached twice (a root given twice, a root inside another) is one file.
* **The dialog**: subfolders or not, the smallest file that counts (the filters' size
  grammar), and whether the copies come back selected.
* **The answer** is a temporary panel (`:dupes`): the groups, the one wasting the most space
  first, each row named `N path` so a sort by name keeps them in that order and a sort by size
  keeps them together; the oldest file of a group first. With the copies selected, everything
  but the oldest of each group is, so `F8` is the clean-up and goes to the wastebasket.
* The walk is the jobs' filesystem, so a server, a bucket and an archive work as they do for
  the branch view; it runs on the worker, the progress is the status line, and it stops at a
  million files and says so.

## 14. Asking a language model

`ai:<question>` on the command line asks the first `[[ai]]` service (SPEC 07 §2b2);
`ai:<name>: <question>` asks the one of that name. Three wire formats: OpenAI's chat
completions (and every server that speaks it), Anthropic's messages, Ollama's chat.

* **Nothing leaves without being asked for.** Only the question goes, unless files are
  *selected*: then a dialog says how many, how big and to which host, with *Cancel* lit, and
  they go only on *Send*. The file under the cursor alone is never sent. Each file is sent as
  text, cut at 256 KiB with a line saying so.
* **The token** is read from the environment variable `token_env` names, at the moment of
  asking; it is never in `fcmd.toml`, the state file or the answer.
* **The answer** is a Markdown file in `fcmd-ai/` beside the settings — the question as its
  heading, the service and model under it, then the answer — opened in the viewer, which
  renders Markdown. `ai:` with no question opens that folder in the panel: the history of
  what was asked, each one a file to read again (`F3`), keep, copy or delete.
* One request, one answer, on the worker; no streaming. **What it cost** is read from the
  reply where each format puts it (`usage.input_tokens`/`output_tokens`, `usage.prompt_tokens`/
  `completion_tokens`, Ollama's `prompt_eval_count`/`eval_count`) and said on the status line
  and in the answer file's second line.
* **`ai+ question`** (0.39.0) is a follow-up: the thread is the last answer of the session, or
  the answer file under the cursor in `fcmd-ai/`; its turns are read back from the file — the
  first question under `# `, each follow-up under `## ` after a `---` rule, the italic line
  under each heading left out — and sent again as alternating user and assistant messages, and
  the new turn is appended to the same file.
* **`ai! what to do`** (0.39.0) asks, with an instruction in front of the service's own, for
  exactly one command line for the shell fcmd runs and the OS it is on; the first line of the
  answer that says anything, with fences, backticks, quotes and a `$ ` taken off, is put on the
  command line and **not run**.
* **`Ctrl+Alt+X`** (`ai.explain`, viewer and editor) sends the marked block — in the editor —
  or the rows on the screen, named after the file, after the same dialog files get.
* **`Shift+F7`** (`git.commit.draft`, panels) asks, after a dialog naming the staged diff and
  the host, for a one-line commit message from `git diff --cached --stat --patch` (run on the
  worker, capped like a file); the answer opens the commit prompt, which commits only on
  `Enter`.
* A line that starts with `ai:`, `ai+` or `ai!` is not expanded: FAR's metasymbols would turn
  the `!` of `ai!` into a file name.

### 14.1 The AI panel (0.39.0)

`Alt+F1` / `Alt+F2` has an **AI** row (`:ai`, hidden like the others with `[drives] panels`):
the `[[ai]]` services as a panel, one row each — name, protocol, model, host. `F7` or
`Shift+F4` adds one through the service dialog, `F4` edits the one under the cursor, `F8`
removes it.

`Enter` is the **login**: one `GET` of the service's list of models — `…/models` beside an
OpenAI or Anthropic endpoint, `…/api/tags` beside Ollama's chat — with the key it will be
asked with (from `token_env`, or asked for and kept in the secrets file, §14). Nothing else is
sent and nothing is generated. The status line says what happened: the host, the HTTP status,
the time, how many models it lists and whether the configured one is among them; a refusal,
a wrong key or a server that is not running is git-style error text from the request.

Logged in, the panel becomes the **conversation** (`:aichat`): first the files of its context,
then the conversation file itself once there is one — real files, so `F3`, `F4` and `Ctrl+Q`
read them. `F5` in the other panel puts the files selected there (or the one under the cursor)
into the context instead of copying anything; only files on this disk. `F8` takes a file out of
the context, or on the conversation row starts a new conversation. While the active panel is
the conversation, the **command line is the question**: what is typed goes to the service as
the next turn of the conversation (as `ai+` does), with the files of the context; the dialog
that names the files, their size and the host comes back whenever they are not the ones sent
last time. `ai:`, `ai+` and `ai!` keep their meaning there. Logging in again to the same
service keeps its conversation and its files; another service starts afresh.

## 15. Git

`Ctrl+Alt+G` (`tool.git`, *Commands → Git*) on a folder of this disk, and `F2` inside any of
the git panels: a menu of what git can do with the repository the panel is in. It runs the
`git` on the machine — no library, no second idea of what a repository is — on the worker,
and shows git's own error when it fails (not a repository, no git installed, a switch over
uncommitted work). Everything it shows is a **panel**, three of them, each a temporary panel
with an origin of its own and a key bar that says what its keys do there:

* **What changed** (`:git`): `git status --porcelain -z`, untracked files included, as the real
  files. Since 0.39.0 a row is one *part* of a file — conflict, staged (the index letter),
  not staged (the work-tree letter), untracked — so a file changed on both sides is two rows,
  each with its own id (a path that repeats takes the next free number after its hash). The
  part and the kind (modified, new, deleted, renamed, copied, type changed, to resolve) are
  words, padded into columns, then the path from the top of the work tree; the order is the
  parts in that order, then the path, and the panel is unsorted. *Show what changed as a tree*
  in the `F2` menu draws the same answer, without asking git again, with each folder heading
  its files once per part; a folder row is the folder for `F5`, `F6` and `F3`. The title
  counts staged and not staged. `F3` is the two versions side by side in the internal diff
  (SPEC 05 §3): `git show :path` against the file for a row not staged, `git show HEAD:path`
  against `git show :path` for a staged one, an absent version being an empty side; on a
  folder it is `git diff` of the folder, and an untracked file is read. The quick view beside
  the panel shows `git diff` (or `--cached`) of the row, coloured as a diff. Every job that
  ends asks git again for each side showing what changed. `F4` edits, `F5`
  stages (`git add --`), `F6` unstages (`git restore --staged --`), both on the selection or
  the row, `F7` commits after a prompt for the message (`git commit -m`, an empty message
  refused before git is asked), `F8` throws the changes of the selected rows or the one under
  the cursor away after a confirmation with Cancel lit — `git restore --worktree` for rows not
  staged, `git restore --source=HEAD --staged --worktree` for staged ones, `git rm --cached` for
  a file new in the index (it stays on disk, untracked), the wastebasket (a job, so the undo
  has it) for an untracked file, nothing for a conflict; the file's log is in the `F2` menu.
* **Branches** (`:gitbranches`): the local branches, the current one marked. `Enter` on another
  asks and then runs `git switch`; `F3` is that branch's log.
* **The log** (`:gitlog`): up to a thousand commits, newest first and kept in git's order (the
  panel is unsorted), one row each — hash, date, author, subject — of the repository, a branch,
  or one file (`--follow`). `Enter` or `F3` shows a commit with `git show --stat --patch`.

In the log and the branches panels (0.39.0) `F4` is what changed, `F5` pull, `F6` push, `F7`
commit, and `F8` the other of the two lists; the quick view beside either asks git on a worker
(`Request::GitQuick`) for what `F3` would show — the commit with its patch, the branch's log —
and shows the answer only if it is still for the row under the cursor. `Alt+F1`/`Alt+F2` list
**Git** among the panels when the panel's folder is in a repository (the place worker found a
`Repo` for it), and choosing it opens the log on that side.

`Blame` opens in the viewer. **Pull** and **push** run in the background with
`GIT_TERMINAL_PROMPT=0` and, unless one is set, `GIT_SSH_COMMAND="ssh -o BatchMode=yes"`, so
a credential git would ask for fails with git's message instead of a prompt on the screen fcmd
draws on; the status line carries the last line git said. Anything that changes the
repository — stage, unstage, commit, switch, pull — asks git for the panel's view again, and
`Ctrl+R` does the same rather than re-reading the rows as files.

## 16. The system log

*Commands → System log* (`tool.syslog`), FAR's EventViewer: the latest entries of the
system's own log in the viewer, read by the tool each system has for it — `journalctl -n 2000`
on Linux, `log show --last 1h` on macOS, `wevtutil qe System /c:2000` on Windows — on the
worker, into a file of the session's temporary folder. Without the rights to the system
journal, what the tool says about that is what is shown. The viewer's search, filter and
`Ctrl+F10` work on it as on any text; it is not a live tail.
