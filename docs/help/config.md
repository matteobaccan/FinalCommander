# Where things are kept

Everything is a file you can read, diff and copy to another machine:

    config.toml     the settings
    keymap.toml     your own bindings, over the profile
    session.toml    where the panels were
    themes/         colour schemes
    sites.toml      stored connections

They live where the platform keeps such things — `~/.config/fcmd` on Linux, `Library/
Application Support` on macOS, `%APPDATA%` on Windows. `fcmd --dump-config` prints the settings
the program is actually running with, which is the quickest way to see what a key is called.

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
`mc`. A binding of your own is a line in `keymap.toml`:

    [[bind]]
    keys = ["Ctrl+D"]
    scope = "panel"
    cmd = "file.delete"

`scope` is where it applies — `global`, `panel`, `viewer`, `editor` or `dialog` — and `cmd` is
one of the names on the "Commands" page of this help. `fcmd --check-keymap` says whether a
keymap is sound and names anything bound twice.

# Colours

`Ctrl+Shift+T` steps through the themes; the menu offers them as a list. A theme is a TOML file
of colour names, and one dropped into `themes/` is found at startup — there is nothing to
register and nothing to rebuild.

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
`group`, `links` and `descr` — the same fields the built-in modes draw. A width is a number of
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

A directory can carry its own `.fcmd-menu.toml`, and its rows come above the ones that are
everywhere: the menu of the project you are in is the one you want while you are in it.
