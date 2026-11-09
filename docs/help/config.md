# Where things are kept

Everything is a file you can read, diff and copy to another machine:

    fcmd.toml          the settings — and your keys, themes, servers, filters and colours
    fcmd-state.toml    what the program remembers: where the panels were, the histories
    fcmd-macros.toml   the macros you recorded
    fcmd-menu.toml     your own F2 menu
    fcmd-secrets.vault your keys and passwords, encrypted with a password of your own
    fcmd-plugins/      plugins

They sit side by side in the folder the platform keeps settings in — `~/.config` on Linux,
`~/Library/Application Support` on macOS, `%APPDATA%` on Windows — or beside the executable,
when an `fcmd.toml` is there, which is how a copy on a USB stick keeps everything with it.
`fcmd --paths` says exactly where each one is on this machine, and `fcmd --dump-config` prints
the settings the program is actually running with, which is the quickest way to see what a key
is called.

The clock in the top right corner is off until you ask for it, in `F9 → Options → Interface
settings`: it is the only thing on the screen that changes with nobody pressing anything, so
with it on the program wakes up when the minute turns to redraw four characters. That is one
redraw a minute, and over `ssh` it is traffic you are paying for — which is the whole of why it
is not on by default.

`F9 → Options → All settings` is every setting there is, in one list: the key, the value it is
at, and a `·` on the ones that are no longer what the program was built with. Type to narrow it,
`Enter` or `F4` to type a value, `Space` to flip a yes-or-no one, `Del` to put the built-in
value back. The dialogs above it in that menu are the dozen settings most people change, with
words saying what each one means; this is the rest, and the way to find out what a key is
called.

`Options → Save setup` writes the settings file; nothing else does, so a setting changed to see
what it does is gone at the next start. If you would rather it were not, *Save the setup on
exit* in *Options → Interface settings* (`ui.save_setup_on_exit`) writes the file on the way
out as well. Where the panels were is a different file and comes back either way.

# Asking an AI

The easiest way is the **AI** row of `Alt+F1` / `Alt+F2`: it turns the panel into the list of
your services. `F7` adds one, `F4` changes it, `F8` removes it. `Enter` logs in — fcmd asks the
service for its list of models, which proves the address and the key without asking the model
anything, and the status line says what came back (`HTTP 200 in 120 ms, 12 models`), or why it
failed (a server that is not running, a wrong key). Then the panel is the **conversation**:

* whatever you type on the command line under it goes to the AI, as the next turn of the same
  conversation — no prefix needed;
* `F5` in the other panel puts the files selected there into the **context**, and they go with
  every question from then on — before the first one that carries them, a dialog says which
  files, how big and to which host;
* the panel shows the files of the context and the conversation itself: `F3` or `Ctrl+Q` reads
  it, `F8` takes a file out of the context (on the conversation row, it starts a new one).

The rest of this chapter is the same thing from the command line.

`ai: question` on the command line asks a language model and opens the answer in the viewer.
The services are `[[ai]]` blocks in `fcmd.toml`:

```toml
[[ai]]
name = "claude"
url = "https://api.anthropic.com/v1/messages"
api = "anthropic"            # openai, anthropic or ollama
model = "claude-sonnet-5-5"
token_env = "ANTHROPIC_API_KEY"
```

They need not be written by hand: *Commands → AI services* lists them as NetBox lists the
sessions. `Ins` adds one through its dialog (name, protocol, address, model, the variable the
key is in) — changing the protocol rewrites the address for it, on the same host, and an
address typed without `http://` or `https://` gets `http` on this machine and `https`
anywhere else; a bare host gets the protocol's usual path, and a list of models pasted for
the address is taken back to the endpoint beside it — `F4` changes it, `Del` removes it, and `Enter` makes it the one `ai:` asks. With no
variable, or one that is not set, the key is asked for the first time it is needed and kept in
`fcmd-secrets.vault`, fcmd's own encrypted file beside the settings — so it travels with them
to another machine. The first time, you choose that file's password (leave it empty to keep the
key for this session only). Afterwards the password is asked for once, stays in memory while
you work, and is forgotten after five minutes without a key pressed; the next secret needed
asks for it again. The file is AES-256-GCM under a key derived from the password with
PBKDF2-SHA256, and never holds anything in clear.

The first block is the one `ai:` asks; `ai: local: question` asks the block called `local`.
The key is never written in `fcmd.toml`: `token_env` names the environment variable it is in,
and with none, it goes into the secrets file described above.
`api = "openai"` works with every server that speaks OpenAI's chat API, and `ollama` with a
model running on your own machine, which needs no key at all.

Only the question is sent, unless files are **selected**: then a dialog says how many, how big,
and to which host, and they go only if you press *Send*. Each answer is a Markdown file in the
`fcmd-ai` folder beside the settings, and `ai:` with nothing after it opens that folder in the
panel, so what you asked is there to read again. The status line says what the answer cost, in
the tokens the service counted, and the answer file says it too.

Three more ways to ask:

| What you type or press | What it does |
| --- | --- |
| `ai+ question` | asks a **follow-up** in the same thread: the earlier questions and answers go again (no service keeps a conversation), and the new answer is added to the same file. The thread is the last answer of this session, or the one under the cursor in the `fcmd-ai` folder |
| `ai! what to do` | asks for **one command** for your shell and system, and puts it on the command line — **not run**: read it, change it, and press `Enter` yourself |
| `Ctrl+Alt+X` in the viewer or the editor | asks **what this is**: the marked block in the editor, or what is on the screen, goes with the file's name — after a dialog that says how much and to which host |
| `Shift+F7` in a repository | drafts a **commit message** from the staged diff — after the same dialog — and opens the commit prompt with it, to read and change before anything is committed |

A question to the model is not a command: `!` and the other metasymbols are not expanded in it.

# Updates

`Options → Check for updates` asks GitHub which release is the latest and, when it is newer
than the one running, offers to open its page in the browser — the page's address is in the
dialog too, to copy on a machine with no desktop. Nothing is downloaded or installed: the
archives and the changelog are on that page. The program asks only then, unless *Check for a
new release once a day* in *Options → System settings* (`general.update_check`) is on; it is
off, because the program opens no connection you did not ask for. With it on, the check runs
in the background at most once a day, says nothing when there is nothing new or no network,
and offers each new release once.

# Programs from elsewhere

Most of what fcmd does it does itself — zip, the tar family, 7z, `ar` and `.deb`, `cpio` and
`rpm` are read in process, SSH, FTP, WebDAV, S3 and IMAP are spoken in process, and the viewer
disassembles x86 and AArch64 on its own. For a few things it runs a program that is already on
the machine instead, never linked into it. `Alt+F1` → *External programs* lists every one of
them, ticked when it is on the `PATH` and crossed when it is not, with the version it says it
is and what fcmd asks it for. `F3` on a row is its card: where it is, what it is for, and,
when it is missing, how to get it and its website. `F4` installs it: it shows the line it will
run — `winget` on Windows, `brew` on macOS, `apt`, `dnf`, `pacman` or `apk` elsewhere,
whichever this machine has — and, once you say yes, runs it in a terminal of its own, so that
`sudo` can ask for your password and `winget` for its agreements. `Ctrl+Alt+E` goes back to the
panels and `Ctrl+R` in the panel looks again. If the install fails, the terminal says where the
product's own download page is; where no package manager here is known to have the program,
`F4` offers to open that page instead. A package name is only used where it is certain — a
wrong one is worse than none.

Why each one is a program and not part of fcmd:

| Program | For | Why it is not built in |
| --- | --- | --- |
| `7z` | archives fcmd does not read itself: rar, cab, iso, lha, arj, msi, wim, dmg… | RAR's unpacking code comes under its own licence, not an open one, and the rest are dozens of formats one program already reads well. A table drives it (`[[archiver]]`, *Formats of your own*), and it is only ever asked to list and extract |
| `git` | the Git panel (`Ctrl+Alt+G`) and the branch in the prompt | what a repository holds is what `git` says it holds; a second implementation inside fcmd could disagree with the first |
| `javap`, `ilspycmd`, `python3`, `wasm2wat`, `objdump`, `retdec-decompiler` | `Shift+F6` in the viewer, through the `[[decompile]]` rules | each is a specialist's project for one kind of code; fcmd disassembles machine code itself and leaves decompiling to them |
| `ffmpeg` | video frames in the viewer — coming: nothing runs it yet | video codecs carry patents, the libraries that decode them are LGPL or GPL, and no pure-Rust decoder is ready; `F11` already reads a video's container without it |
| `xdg-open` / `open` | opening a link in the browser | which browser is the desktop's choice, not fcmd's |
| `journalctl` / `log` / `wevtutil` | *Commands → System log* | each system has one tool that knows its log |
| `systemctl` / `launchctl` | the services panel | the service manager is the only thing that can start and stop a service |

A program counts as installed when it is on the `PATH`, by the same rule a shell uses, and
nowhere else. 7-Zip's Windows installer, for one, puts `7z.exe` in *Program Files* without
adding it to the `PATH`: the panel then says it is missing, and adding that folder to the
`PATH` is the fix. *External programs* in the `F9` dialog of `Alt+F1`
(`drives.panels.tools`) takes the row away; `Ctrl+7` there hides every panel row at once.

# A copy that remembers nothing

`fcmd --no-save` starts with your histories and panels as usual but writes nothing back when it
quits — for a second copy opened for a quick job, which would otherwise overwrite the main
copy's histories with its own when it is the last to close.

# What is remembered

Between sessions the program keeps, in its state file beside the settings: the command lines
that have run (`Alt+F8`), where each panel had been (`Alt+F12`), **the files you have read or
edited** (`Alt+F11`), the folders you keep to hand, the tabs of both panels and where they
were. `ui.command_history` and `ui.file_history` say how many of the first and third to keep,
and `ui.history_lifetime` how many days a line nobody has pinned is kept after it was last
used — ninety by default, `0` to keep them for ever. `Ins` in any of the three lists pins the
row under the cursor, and nothing pushes a pinned row out.

A file you have read before opens where you left it — the position, the code page, the mode,
the wrap and the ten bookmarks — unless you turn *Remember where a file was left* off in
*Options → Viewer settings*. The last five hundred and twelve files are kept.

The list of files is written wherever a viewer or an editor opens, so it is the same list
whichever key opened the file. A file on a server is remembered by its remote path, not by the
copy that was fetched to read it; a file opened again moves to the top rather than appearing
twice. `Enter` on a row reads it, `F4` edits it, and a row whose file has gone says so.

# Keys

The keymap is a profile plus your changes. Two profiles ship: `far`, which is the default, and
`mc`. Your changes are a keymap of your own in `fcmd.toml`, built on one of the two and named
in `[ui]`:

    [ui]
    keymap = "mine"

    [[keymap]]
    name = "mine"
    inherit = "far"

    [[keymap.bind]]
    keys = ["Ctrl+D"]
    scope = "panel"
    cmd = "file.delete"

    [[keymap.unbind]]
    keys = ["Ctrl+Q"]
    scope = "global"

Each `[[keymap.bind]]` adds or replaces one binding, and `[[keymap.unbind]]` takes one of the
profile's away. `scope` is where it applies — `global`, `panel`, `viewer`, `editor` or `dialog` — and `cmd` is
one of the names on the "Commands" page of this help. `fcmd --check-keymap` says whether a
keymap is sound and names anything bound twice.

Before rebinding anything, it is worth knowing what your terminal actually sends: *Options →
Learn keys* draws a cell for every key worth testing — the function keys with each modifier,
the arrows with `Ctrl` and `Shift`, the editing keys — and lights the cell up when that key
arrives. Press them and watch. A cell that never lights is a key your terminal is not sending
at all, and no binding will conjure it; the line underneath says what the last key that *did*
arrive is bound to here, which is how you tell "the terminal swallowed it" from "it arrived
and does nothing". Two `Esc` leave, because one `Esc` is a key under test like any other.

Midnight Commander, where this comes from, goes on to let you type in the escape sequence a
key should have sent. This does not: the sequences are read by a library with a table for the
terminals in use, which is better at it than anything typed in by hand — so what is missing
here is missing from the terminal, and the answer is a different key rather than a different
table.

# Colours

`Ctrl+Shift+T` steps through the themes; the menu offers them as a list. Seven ship with the
program; one of your own is a `[[theme]]` block of colour names in `fcmd.toml`, or comes with a
plugin, and is found at startup — there is nothing to register and nothing to rebuild.
`fcmd --list-themes` says which themes there are and where each came from.

The default theme is FAR's own palette, taken from its source rather than from a screenshot.

# Colouring files by what they are

A theme says what an archive or a hidden file looks like. A `[[highlight]]` group in
`fcmd.toml` says it for whatever you like: a mask, a size, an age, attributes, files or
folders — written the way the filter dialog writes them.

```toml
[[highlight]]
name = "Rust"
mask = "*.rs"
fg = "#d7875f"

[[highlight]]
name = "Written today"
age = "<=1"
fg = "brightgreen"

[[highlight]]
name = "Big and hidden"
size = "1G-"
attrs = "h-s"
fg = "red"
bg = "idx:52"
```

The first group that matches a row decides, so the order in the file is the order that counts,
and the built-in groups come after yours. The ink is a colour — `#d7875f`, `red`, `idx:208` —
or the name of a theme colour such as `file.archive`, which then follows the theme when you
change it. A group that does not parse is named on the startup screen and skipped; the program
starts either way.

You do not have to write the file. `F9 → Options → Files highlighting` lists the groups in the
order they are tried, each row drawn in its own colours: `Enter` or `F4` edits one, `Ins` makes
a new one beside it, `Del` takes one away, and `Ctrl+↑`/`Ctrl+↓` move one past its neighbour
when two of them match the same file. Every row of the dialog takes exactly what the file
takes, and a condition that does not parse comes back with the problem named rather than being
kept. A change is in force on the panels at once; `Options → Save setup` is what writes it to
the file.

# Columns of your own

The ten view modes are lists of columns somebody else decided on. A `[[column_set]]` block
makes one of them yours: the `id` is the digit the view is on, so `id = 3` is what `Ctrl+3`
draws, and a view nobody has written a set for stays the built-in one.

```toml
[[column_set]]
id = 3
columns = [
  { type = "name",  width = "*" },
  { type = "size",  width = 10, align = "right", fmt = "bytes" },
  { type = "mtime", width = 16, fmt = "%Y-%m-%d %H:%M" },
]
```

The types are `name`, `ext`, `size`, `allocated`, `mtime`, `btime`, `atime`, `attrs`, `owner`,
`group`, `links`, `descr` and `git` — the same fields the built-in modes draw, and `git`, which
says where each row stands in its git repository: `M` or ` M` changed (staged on the left, not
yet on the right), `A` added, `??` new, `!!` ignored, and for a folder `M` when anything inside
it is changed. `{ type = "git", width = 2 }` beside the name is the column File Ninja calls git
status; *sort by git status* in `Ctrl+F12` puts the changed files first. A width is a number of
columns, a percentage of the panel, `*` to take whatever is left, or `0` for the column's own
natural width, which for a name or a description means "what is left over" because neither has
a width of its own. `align` is `left` or `right` where the column's own habit is not what you
want; `fmt` is `human` or `bytes` on a size — `1.18 M` against `1 234 567` — and a `strftime`
string on a date.

A column the panel has no room for is dropped from the right, and a set that would leave the
name unreadable drops another, which is the rule the detailed view already follows with its
extra dates. A column that costs a lookup per file — the owner, the group, the link count, the
description — is fetched by the same worker the view that shows it uses, so it fills in a
moment after the listing; a view whose set does not ask for one of them pays nothing at all.

Coming from FAR you can write the pair of strings its own panel-mode dialog writes instead:

```toml
[[column_set]]
id = 4
far_types  = "N,SC,DM"
far_widths = "0,9,16"
```

What FAR has and this has not — the stream size, the change time, the stream count — is named
on the startup screen rather than quietly left out, and a set with nothing in it that could be
read leaves that key's built-in view alone.

Beside the columns there are the switches of the same FAR dialog, which belong to the mode
rather than to the program:

```toml
[[column_set]]
id = 4
columns = [{ type = "name", width = "*" }, { type = "size", width = 10 }]
fullscreen              = true   # this mode gives the panel the whole screen
align_extensions        = true   # a file's extension down the right of the name field
align_folder_extensions = false  # and a directory's: off, its dot is part of its name
folders_upper           = false  # directory names in capitals
files_lower             = false  # file names in small letters
upper_to_lower          = false  # a name that is *all* capitals drawn in small letters
```

Each one left out is what the panel drew before they existed: the extensions of files lined
up, and nothing else. `fullscreen` is `Ctrl+F1` done by the mode — choosing the mode hides the
other panel and leaving it gives the screen back — but a panel you hid with the key yourself
is left alone. `upper_to_lower` is for the 8.3 names a FAT volume hands back, all capitals and
never meant to be read that way; a name with no letters in it is not all capitals, so `README`
changes and `2026` does not.

# What a file is

`panel.descriptions` decides whether the directory's own `descript.ion` is read at all. On by
default: `Ctrl+Z` writes a line about a file, `Ctrl+6` and `Ctrl+7` show them, and the
information panel spells one out. Off, and the file is never opened — which is what somebody
who keeps no descriptions wants on a network share, where a side file per directory is a round
trip per directory.

Two settings of FAR's say where that file is and how it is written:

```toml
[panel]
description_names = ["files.bbs", "descript.ion"]   # which names, in which order
description_utf8  = false                           # keep the code page it was read in
```

The names are looked for in order and the first is what a new file is written under; an empty
list is the two above. With `description_utf8` off the file goes back **in the page it was
read in** — there is nothing else to go on, and guessing this machine's own page would turn a
Russian `descript.ion` into a Latin one. A file that is not there yet, one that was already
UTF-8, and one whose page cannot hold what you typed are all written in UTF-8 anyway: a file
a DOS tool cannot read beats a line that lost its words.

# Breaking a tie

`panel.sort_chain` says what settles two rows the sort key leaves equal:

```toml
[panel]
sort = "size"
sort_chain = ["ext", "name"]
```

Up to three keys — `name`, `ext`, `size`, `mtime`, `btime`, `atime` — asked in the order they
are written. The name has the last word after them in any case, so the order never wanders
between two reads of the same directory, and turning the sort round turns the whole thing
round rather than only its first step.

# Putting kinds of file first

A `[[sort_group]]` block in `fcmd.toml` is a band of the listing: a mask, and which side of
the ordinary files it sits on. The bands pull some kinds of file in front of the listing and
push others behind it whatever the panel is sorted by.

```toml
[[sort_group]]
name = "sources"
mask = "*.rs,*.toml"

[[sort_group]]
name = "build output"
mask = "*.o,*.rlib,*.d"
place = "back"
```

The front bands keep the order they are written in, so do the ones behind, and the files that
match no band sit between them. Inside a band the ordinary sort still applies, folders keep
their place at the top, and `reverse` turns each band round rather than swapping the bands
over. Writing the blocks turns them on; `Shift+F11` — `Group` on the key bar — turns them off
and on again, one panel at a time. A block that does not parse is named on the startup screen
and skipped, as a highlighting group is.

# The palette editor

`F9 → Options → Colours…` lists every colour the interface uses, with the value as a theme file
writes it and two samples of it: the colour as ink, and the colour as a background. `Enter` asks
for a new one — a name (`red`, `brightcyan`, `default`), one of the 256 (`idx:208`), or
`#ff8000` — and the screen changes as you answer, which is the only way to judge a colour.

`F2` writes the result into `fcmd.toml` as a `[[theme]]` block under the theme's own id, so the
next run starts with it and `Shift+F12` still cycles the themes. Leaving without `F2` keeps the
colours for this session and says so.

# Your own menu

`F2` opens a menu you write. It lives in `fcmd-menu.toml` beside the settings — `fcmd --paths`
says exactly where — and a row is a label, a key to take it with, and the command lines to run:

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
      { key = "l", label = "log", run = ["git log --oneline -40"] },
    ]

A line is the same command line the command line itself takes, so the substitutions work:
`{file}` is the file under the cursor in quotes, `{selection}` the names you picked, `{list}` a
file holding them, `{ask:title=Branch}` asks you before anything runs. Several lines run one
after another and stop at the first failure.

`when` decides whether a row is shown at all. Four facts — `panel.has('Cargo.toml')`,
`file.is('*.rs')`, `panel.selected`, `panel.local` — with `!`, `&&` and `||` between them. A
condition it cannot read leaves the row on screen and says why, rather than hiding it silently.

A row with `separator = true` is a line between two groups instead, and a `label` on it is
written into the line as the group's heading. A separator whose group the conditions emptied
goes with it, so the menu never shows two lines in a row.

    [[menu]]
    separator = true
    label = "Version control"

`Shift+F10` opens the menu again on the row you took last — inside its submenu if it was in
one — so running it once more is `Shift+F10` and `Enter`.

A directory can carry its own `.fcmd-menu.toml`, and its rows come above the ones that are
everywhere: the menu of the project you are in is the one you want while you are in it.

# Plugins

A plugin is a folder in `fcmd-plugins/`, beside the settings (`fcmd --paths` says where), with
a `plugin.toml` in it. It is found at startup: nothing to register, nothing to build. It can
bring **data** — themes, keymaps, `[[assoc]]` blocks, `[[decompile]]` rules, `[[mask-group]]`
lists, all in the same words `fcmd.toml` uses — and **commands**: a `[[command]]` names a
program in the plugin's folder, and `F11` lists the commands for the screen you are on.

A command runs as a program of its own, never inside fcmd, so a plugin that crashes crashes
alone. It is told where the panels are and what is selected, in the environment and as JSON on
its input, and answers one line at a time: a message, a folder to go to, files to select, or a
command of fcmd's own to run — nothing a key could not do. A command that renames or writes
over files says so in its manifest, and fcmd asks before running it. `examples/plugins/` has
twenty-one worked ones, from finding duplicates to converting line endings.

*Options → Plugins configuration* lists every plugin found: `✓` for one that is on, `·` for
one that is off, and `!` for one that did not load. `Enter` turns the one under the cursor on
or off and writes it down at once (`[plugins] disabled` in `fcmd.toml`); its commands come and
go from `F11` straight away, while the rest of what it brings — themes, keys, associations,
decompilers, mask groups — is read at startup and changes at the next one. `F3` is its card:
who wrote it, where it lives, what it brings, its commands, and anything said about it while
loading — for one that did not load, why. Its own settings are its own files: `F3` names the
folder, and the plugin's `plugin.toml` is what to edit. `fcmd --list-plugins` prints what was
found, and what did not load, without starting the interface.

# Formats of your own

An `[[archiver]]` block in `fcmd.toml` teaches the panels a format nothing shipped reads: it
names a program on the machine, the command line that lists an archive and the one that takes
an entry out, and how to read what the listing prints. The language is FAR's MultiArc, so a
table copied out of a `custom.ini` keeps its key names.

    [[archiver]]
    name = "unace"
    extensions = ["ace"]
    List = "v -y %A"
    Extract = "x -y %A @%L"

    [archiver.listing]
    start = "^Date    Time"
    end = "^listed:"
    rows = ['^(?<size>\d+)\s+(?<name>\S.*)$']

`%A` is the archive and `%L` a file holding the names to take out. A command line is never
handed to a shell: it is split on spaces once, and a name with a space in it stays one
argument. The rows are patterns with named captures, and `name` is the one that must be there.

Only reading is accepted — `List`, `Extract`, `ExtractWithoutPath`, `Test`. A block with a line
that would change an archive (`Add`, `Delete`, `SFX`…) is refused by name, and so is a block
that is wrong in any other way: each one is a warning at startup, with the block's name and
the reason, rather than a broken archive on the first `Enter`. Your blocks are asked before the
shipped `7z` one, and a block named `7z` replaces it, which is how its switches are changed. A
format this program reads itself — zip, 7z, the tars, `ar`, the bare compressors — is never
handed to a block.

