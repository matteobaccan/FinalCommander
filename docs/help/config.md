# Where things are kept

Everything is a file you can read, diff and copy to another machine:

    fcmd.toml          the settings — and your keys, themes, servers, filters and colours
    fcmd-state.toml    what the program remembers: where the panels were, the histories
    fcmd-macros.toml   the macros you recorded
    fcmd-menu.toml     your own F2 menu
    fcmd-secrets.vault your keys and passwords, encrypted with a password of your own
    fcmd-plugins/      plugins
    fcmd-jobs/         the journals of the copies that are running — there only while one is

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
failed (a server that is not running, a wrong key). Then the panel is the **conversation**,
shown in it as it goes, rendered as Markdown: the files of the context at the top, each
question on the right, each answer on the left as the model writes it, and while it is still
coming a turning wheel with the seconds it has been thinking. The frame names the model that
answered, how full its context window is (`ctx 1.5k/32.8k (4%)`; the size is shown when
the server says it at the login) and about how much the next question will take (`next ≈ 2.1k`:
the conversation so far, which goes again, the files and the question, at four bytes a token). An answer stays in the panel; it never opens a viewer. The arrows and `PgUp`/`PgDn` scroll it; `Ctrl+R` builds it
again and keeps it a conversation:

* whatever you type on the command line under it goes to the AI, as the next turn of the same
  conversation — no prefix needed;
* `F5` in the other panel puts the files selected there into the **context**, and they go with
  every question from then on — before the first one that carries them, a dialog says which
  files, how big and to which host;
* `F8` lists the files of the context with their sizes: `Enter` or `Del` takes the one under
  the cursor out — the file stays on the disk — and the last row takes them all; with no files
  in the context, `F8` starts a new conversation;
* `F7` lists the conversations kept so far, newest first, each by its first question: `Enter`
  goes on with one, and the next question carries it;
* `Esc` while the model is still answering stops the question there: nothing more is shown of
  it and nothing is written;
* a question that would take more than the model holds is stopped first at a dialog that says
  so, with the estimate, even when the files are the ones sent before.

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

OpenAI's newer models — the reasoning ones and GPT-5 — want a request shaped a little
differently: `max_completion_tokens` rather than `max_tokens`, sometimes the `developer` role
rather than `system`, and an answer not streamed until the organisation is verified. fcmd
sends OpenAI the new name from the start, and when any service refuses one of these and says
what it wants instead, asks again the way it said and remembers it for the session.

They need not be written by hand: *Commands → AI services* lists them as NetBox lists the
sessions. `Ins` adds one through its dialog (name, protocol, address, model, the variable the
key is in) — changing the protocol rewrites the address for it, on the same host, and an
address typed without `http://` or `https://` gets `http` on this machine and `https`
anywhere else; a bare host gets the protocol's usual path, and a list of models pasted for
the address is taken back to the endpoint beside it; while a question is out, the corner of
the status line says which service is thinking and for how many seconds — `F4` changes it, `Del` removes it, and `Enter` makes it the one `ai:` asks. With no
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

More ways to ask:

| What you type or press | What it does |
| --- | --- |
| `ai+ question` | asks a **follow-up** in the same thread: the earlier questions and answers go again (no service keeps a conversation), and the new answer is added to the same file. The thread is the last answer of this session, or the one under the cursor in the `fcmd-ai` folder |
| `ai! what to do` | asks for **one command** for your shell and system, and puts it on the command line — **not run**: read it, change it, and press `Enter` yourself |
| `Ctrl+Alt+X` in the viewer or the editor | asks **what this is**: the marked block in the editor, or what is on the screen, goes with the file's name — after a dialog that says how much and to which host |
| `Ctrl+Alt+X` in the panels, after a command that failed | asks **why it failed**: the command, its exit code and the end of what it printed — the last 200 lines at most, the ones `Ctrl+O` shows — go after the same dialog, and the answer opens in the viewer. Once another command has ended well there is nothing to ask about |
| `Ctrl+Alt+U` on selected files | asks for a **rename pattern** from words, and opens the multi-rename dialog with it — see *Renaming many files at once* |
| `Ctrl+Alt+W` in the editor | has the marked block **rewritten**, shows the result as a comparison and puts it in only on `Enter` — see *Having a block rewritten* |
| `Shift+F7` in a repository | drafts a **commit message** from the staged diff — after the same dialog — and opens the commit prompt with it, to read and change before anything is committed |

A question to the model is not a command: `!` and the other metasymbols are not expanded in it.

## What is masked before it goes

Files hold keys: an `.env` beside the sources, a token pasted into a script, a private key in
a folder that was selected whole. Before a question leaves the machine — the question itself,
the files that go with it, the block from the editor, the earlier turns of the conversation —
fcmd masks what looks like a secret and writes `[REDACTED:kind]` where it was:

| What is found | Written in its place |
| --- | --- |
| a private key block (`-----BEGIN … PRIVATE KEY-----` to its end) | `[REDACTED:private-key]` |
| the keys that say what they are: AWS (`AKIA…`), GitHub (`ghp_…`, `github_pat_…`), GitLab (`glpat-…`), Slack (`xoxb-…`), Stripe (`sk_live_…`), OpenAI and Anthropic (`sk-…`), Google (`AIza…`), npm (`npm_…`), a JWT | `[REDACTED:aws-key]`, `[REDACTED:github-token]`, `[REDACTED:api-key]` and so on |
| the password in an address: `postgres://app:secret@db/shop` | `postgres://app:[REDACTED:url-password]@db/shop` |
| what follows `Authorization: Bearer` or `Basic` | `[REDACTED:auth-header]` |
| the value of a line `KEY=value`, `key: value` or `"key": "value"` whose key ends in `password`, `secret`, `token`, `api_key`, `access_key`, `private_key`, `credential` and the like | `[REDACTED:assignment]` |

The name of the key, the host and the user of an address stay: they are what makes the line
worth sending. A value that is a reference (`${VAR}`), a number or a call in a program is left
alone. It errs towards masking, and it is still a net, not a guarantee — a password that is an
ordinary word in an ordinary sentence has no shape to find.

You can see what it did. The dialog that names the files gains a line a moment after it opens —
`Secrets masked before sending: 2 (1 assignment, 1 aws-key)` — and **`F3`** there opens
**what is sent**, exactly as it will go: the instruction, the conversation so far, the
question with its files, masked. `Esc` comes back to the dialog, which is still yours to
answer. In a conversation, `F3` in the `F8` list of the context shows the same for what goes
with every question. After the answer the status line says how many secrets were masked,
beside what it cost, and the answer file says it on its second line.

Masking is **on** unless you turn it off: `F5` in the list of services (the AI row of
`Alt+F1`) switches it, and the panel's frame says which way it is. In `fcmd.toml`:

```toml
[ai_guard]
redact = true
patterns = ['ACME-\d{6}', 'customer id: (\d+)']   # your own, as regular expressions
```

A pattern of your own masks its first group when it has one — so the words before it stay —
and otherwise everything it matched; they show as `[REDACTED:custom]`.

## Only models on this machine

**Local only** is a switch for when nothing may leave: with it on, fcmd asks only a service
whose address is this machine — `localhost` and anything under it, `127.0.0.1` and the rest of
`127.x.x.x`, `::1`, a socket in the file system (`unix:/run/llm.sock`) — and the hosts you
list as your own. Any other service is **refused before anything is sent**: the question, the
files, the login that would send the key. The status line names the service and the host it
would have gone to.

`F6` in the list of services (the AI row of `Alt+F1`) switches it; the frame then says
`local only`, and every row that will be refused says so. In `fcmd.toml`:

```toml
[ai_guard]
local_only = true
local_hosts = ["gpu-box.lan", "192.168.1.20"]   # a model on your own network counts as local
```

An address is judged by what is written, not looked up: a name that happens to point at
`127.0.0.1` is not local unless you list it, because looking it up is already a question to
somebody else. It is off by default.

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
| `ffmpeg` | the frames of a video in the viewer and the quick view (`F3`, `Ctrl+Q`), and playing one without sound | video codecs carry patents, the libraries that decode them are LGPL or GPL, and no pure-Rust decoder is ready; without it a video opens as its bytes and `F11` still reads its container |
| `xdg-open` / `open` | opening a link in the browser | which browser is the desktop's choice, not fcmd's |
| `journalctl` / `log` / `wevtutil` | *Commands → System log* | each system has one tool that knows its log |
| `systemctl` / `launchctl` | the services panel | the service manager is the only thing that can start and stop a service |
| `plocate` or `locate` / `mdfind` / `es` | the index `Alt+F7` may ask instead of walking (*Finding things*) | the list of a disk's files is the system's to keep up to date; a second one of our own would be a second service to run. Never needed: without one the search walks |

A program counts as installed when it is on the `PATH`, by the same rule a shell uses, and
nowhere else. 7-Zip's Windows installer, for one, puts `7z.exe` in *Program Files* without
adding it to the `PATH`: the panel then says it is missing, and adding that folder to the
`PATH` is the fix. *External programs* in the `F9` dialog of `Alt+F1`
(`drives.panels.tools`) takes the row away; `Ctrl+7` there hides every panel row at once.

# Pictures in real pixels

`viewer.pixels` says how the viewer and the quick view draw a picture or a frame of a video.
It is a row of *Options → Viewer settings* (*Pictures*) and a line of *All settings*.

| Value | What is drawn |
| --- | --- |
| `auto` | the default: real pixels in a terminal known to draw them, half blocks in every other |
| `kitty` | kitty's graphics protocol, whatever the terminal says it is |
| `iterm` | iTerm2's inline images |
| `sixel` | sixel |
| `off` | half blocks, always |

What `auto` goes by is the environment the terminal gave the program — never a question put
to the terminal, which could go unanswered and leave fcmd waiting, or be answered late into
your typing:

| Terminal | Recognised by | Drawn with |
| --- | --- | --- |
| kitty | `KITTY_WINDOW_ID`, or `TERM=xterm-kitty` | kitty |
| Ghostty | `TERM=xterm-ghostty`, or `TERM_PROGRAM=ghostty` | kitty |
| iTerm2 | `TERM_PROGRAM=iTerm.app` | iterm |
| WezTerm | `TERM_PROGRAM=WezTerm` | iterm |
| foot, mlterm, contour | `TERM` | sixel, when the size of a cell is known |
| inside tmux, screen, zellij | `TMUX`, `STY`, `ZELLIJ`, `TERM=screen…` | half blocks |
| anything else | — | half blocks |

Four terminals draw pixels and cannot be vouched for from here, so they are yours to name:
**Windows Terminal** from 1.22 draws `sixel` and does not say which version it is; **xterm**
draws `sixel` only when started as a VT340 (`xterm -ti 340`); **VS Code**'s terminal draws
`iterm` only with its own *Enable Images* setting on; **Konsole** draws `kitty`, but how much
of that protocol depends on more than its version. A name that turns out wrong shows as a blank
where the picture should be, or as a line of letters: set it back to `off`.

kitty and iTerm2 are told how many cells a picture covers, so they need nothing more. Sixel is
drawn pixel for pixel, so fcmd has to know **how large a cell is**. It asks the system — the
terminal's own window size, from the kernel, not from the terminal — and where that says
nothing it takes eight by sixteen, or the ten by twenty Windows Terminal always uses for
sixel. `viewer.cell_width` and `viewer.cell_height` state it instead, for a terminal where a
sixel picture comes out a different size from the frame left for it; both have to be set, and
`0` is "ask".

```toml
[viewer]
pixels = "sixel"     # auto | kitty | iterm | sixel | off
cell_width = 9       # 0 = ask the system
cell_height = 18
```

# What a copy keeps

```toml
[operations]
preserve = ["mtime", "mode", "xattrs"]
```

`operations.preserve` lists what a copy keeps of a file beside its bytes: `mtime` is the time
it was last written, `mode` its permissions, `xattrs` its extended attributes — the tags and
labels a desktop writes (the chapter on files says which, system by system). All three are
kept unless the list says less, and an empty list keeps none. Each is a box of the `F5`
dialog, which opens as this list says, so one copy can still be told otherwise. From the
command line it is written with commas: `--set operations.preserve=mtime,mode`.

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

Nine themes ship with the program:

- `commander` — the classic blue of the orthodox commanders, the default.
- `far` — FAR Manager's own palette, from its source.
- `black` — black behind everything, light text, the cursor bold and underlined.
- `white` — a light page for a light room.
- `matrix` — green on black.
- `tron` — cyan lines on a dark blue night.
- `blade` — amber and teal in the rain.
- `github-dark` — GitHub's dark terminal colours.
- `delphi-dark-2027` — the Delphi IDE's dark editor, for the whole program, block lines included.
- `delphi-inspiration` — FAR's classic blue panels, menus and dialogs, with the Delphi IDE's dark
  editor in the viewer (`F3`) and the editor (`F4`).

`Ctrl+Shift+T` steps through them while the program runs; `F9` → Options → Themes offers them
as a list, and `Shift+F9` keeps the choice. In `fcmd.toml` it is `theme = "far"` under `[ui]`,
and `fcmd --theme far` picks one for a single run. `fcmd --list-themes` says which themes there
are and where each came from. The project's README has a picture of each, all on the same
scene.

One of your own is a `[[theme]]` block of colour names in `fcmd.toml`, or comes with a plugin,
and is found at startup — there is nothing to register and nothing to rebuild. The quickest
start is a built-in one written out: `fcmd --export-theme tron >> ~/.config/fcmd.toml`, then
change the `id`, the `name` and the colours you want different. A block may `inherit` a
built-in theme and name only what it changes, and one with the id of a built-in replaces it.

The default theme is FAR's own palette, taken from its source rather than from a screenshot.

Besides colours, a theme may make a key's text bold, italic or underlined, in a `[styles]`
table of its own:

```toml
[theme.styles]
"syntax.keyword" = "bold"
"syntax.comment" = "italic"
"cursor.fg" = "bold underline"
```

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

A plugin can add columns of its own — a line count, what kind of file this really is, a
photo's date — and a set names one by the plugin and the column together:
`{ type = "plugin:line-count.lines" }`. *Options → Plugins configuration*, `F3` on a plugin,
lists the columns it has, spelled this way. Its cells show `…` until the plugin's program has
answered and are remembered afterwards, so a file is asked about once and again only when it
changes. The *Plugins* chapter below says how one is written.

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

# The order of names

```toml
[panel]
natural_sort = true     # img2 before img10
sort_case = "auto"      # auto | yes | no
```

`sort_case` says whether the sort tells upper case from lower. `auto` does what the system's
own file manager does: the case counts on Linux, where `Makefile` and `makefile` are two
files, and does not on Windows and macOS, where they are one. `yes` and `no` say it outright,
for a configuration that should sort the same on every machine. With the case counting the
capitals come first, as they do in `ls`.

The Latin accents are folded either way, and there is no setting for it: `é` sorts as `e`,
`ß` as `ss`, `æ` as `ae`, `ø` as `o`, `ł` as `l` — every letter of Latin-1 and Latin
Extended-A, and an accent written as a separate combining character after its letter, which
is how macOS stores them. Two names that read the same are settled by how they are written,
the plain one first. It is a table of letters and not a locale: no language's own alphabet
is consulted, and nothing outside Latin is reordered. Both settings are rows of
`F9 → Options → System settings`, and of *All settings*.

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

# Synchronisations kept by name

A `[[sync]]` block in `fcmd.toml` is a pair of folders and how to synchronise them, under a
name. The synchronisation dialog writes one when you give the pair a name, and `Ctrl+Alt+Y`
lists them to run again; they can as well be written by hand.

```toml
[[sync]]
name = "Evening backup"
left = "~/work"
right = "/media/stick/work"
recursive = true
both_ways = false
delete_extra = true
by_content = false
filter = "*.rs,*.md"
report = "~/backup-report.txt"
```

`left` is the source when the run is one way. `recursive` goes into the subfolders and is on
when it is left out; `both_ways`, `delete_extra` (offer to take out what the other side does
not have) and `by_content` (compare the bytes rather than size and date) are off. `filter` is
a mask and `report` the file a run writes what it did to; both may be left out. A block is a
question, not an order: running it shows the table, and `F2` there is what starts anything.

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

**Coming from FAR**, you do not have to write any of this first: a `FarMenu.ini` is read as
it stands — put the one from your FAR profile beside `fcmd.toml`, and the ones in your
project folders are found where they already are. The hot keys, the labels, the `{` `}`
submenus and the `--` separators become the rows above, and FAR's symbols in the commands
become ours: `!.!` the name, `!` the name without its extension, `!\` the folder, `!@!` and
`!&` the selection as a file and as names, `!#` the other panel, `!?title?init!` a question.
The short-name forms (`!~`, `!-!`) are read as the long ones. A row that uses something
with no counterpart here — the drive letter `!:`, the other panel's selection — stays on the
menu and runs nothing: `Enter` on it says why. When anything was read differently from FAR
the status line says so as the menu opens, and the last row of the menu lists every remark.

Where both files are in one place, `fcmd-menu.toml` is the menu and `FarMenu.ini` is not read:
they are never mixed, so converting a menu by hand does not show every row twice.

# Plugins

A plugin is a folder in `fcmd-plugins/`, beside the settings (`fcmd --paths` says where), with
a `plugin.toml` in it. It is found at startup: nothing to register, nothing to build. It can
bring **data** — themes, keymaps, `[[assoc]]` blocks, `[[decompile]]` rules, `[[mask-group]]`
lists, all in the same words `fcmd.toml` uses — and **commands**: a `[[command]]` names a
program in the plugin's folder, and `F11` lists the commands for the screen you are on — in the panels `Ctrl+Shift+F11` as
well, which the key bar shows while both are held.

`F11` lists every command, those that can run where you are first. The others are greyed out
underneath, each saying why — another screen, a plugin turned off, or a condition its author
wrote in `[command.when]`: a folder on this machine (`local`), files to act on (`files`), files
matching some masks (`masks`), a name in this folder (`here`) or in it or a folder above
(`inside`, `[".git"]` for anything about a repository). `Enter` on a grey row says the reason
again and runs nothing. With no plugin installed, `F11` offers to open the plugins folder and
this page.

A command runs as a program of its own, never inside fcmd, so a plugin that crashes crashes
alone. It is told where the panels are and what is selected, in the environment and as JSON on
its input, and answers one line at a time: a message, a folder to go to, files to select, or a
command of fcmd's own to run — nothing a key could not do. A command that renames or writes
over files says so in its manifest, and fcmd asks before running it. `examples/plugins/` has
twenty-one worked ones, from finding duplicates to converting line endings.

A plugin can also compute a **column** for the panels. A `[[column]]` in its `plugin.toml`
names a program:

```toml
[[column]]
id = "lines"
title = "Lines"
width = 7
align = "right"
run = "count.sh"
masks = "*.rs,*.toml"
timeout = 10
```

and `{ type = "plugin:<plugin>.lines" }` in a `[[column_set]]` (see *Columns of your own*)
draws it. The program is run once per file, with the path as its first argument, and the
first line it prints is the cell; nothing printed, or a failing exit status, is an empty
cell. `masks` says which files it is asked about (all of them when absent; directories only
with `folders = true`), and `batch = true` runs it once per directory instead — the paths on
its input, one per line, and `path<TAB>value` for each on its output — for a program that is
slow to start. None of this holds the panel up: the listing is drawn at once with `…` in the
cells still being asked about, and they fill in as the answers come. An answer is kept by
path, date and size for the rest of the session. A run that takes longer than `timeout`
seconds (ten when absent) is stopped; after three of those in one directory the column gives
up there and the status line says so. Only folders on this machine's disks are asked about —
inside an archive or on a server the column stays empty — and the panel cannot be sorted by
such a column. A plugin's columns appear, or go, at the next start after it is turned on or
off.

*Options → Plugins configuration* lists every plugin found: `✓` for one that is on, `·` for
one that is off, and `!` for one that did not load. `Enter` turns the one under the cursor on
or off and writes it down at once (`[plugins] disabled` in `fcmd.toml`); its commands go grey
in `F11`, or back, straight away, while the rest of what it brings — themes, keys, associations,
decompilers, mask groups — is read at startup and changes at the next one. `F3` is its card:
who wrote it, where it lives, what it brings, its commands, its columns, and anything said about it while
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

