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
