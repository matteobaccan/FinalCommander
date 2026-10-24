# Where things are kept

Everything is a file you can read, diff and copy to another machine:

    config.toml     the settings
    keymap.toml     your own bindings, over the profile
    session.toml    where the panels were
    themes/         colour schemes
    sites.toml      stored connections

They live where the platform keeps such things — `~/.config/scmd` on Linux, `Library/
Application Support` on macOS, `%APPDATA%` on Windows. `scmd --dump-config` prints the settings
the program is actually running with, which is the quickest way to see what a key is called.

# Keys

The keymap is a profile plus your changes. Two profiles ship: `far`, which is the default, and
`mc`. A binding of your own is a line in `keymap.toml`:

    [[bind]]
    keys = ["Ctrl+D"]
    scope = "panel"
    cmd = "file.delete"

`scope` is where it applies — `global`, `panel`, `viewer`, `editor` or `dialog` — and `cmd` is
one of the names on the "Commands" page of this help. `scmd --check-keymap` says whether a
keymap is sound and names anything bound twice.

# Colours

`Ctrl+Shift+T` steps through the themes; the menu offers them as a list. A theme is a TOML file
of colour names, and one dropped into `themes/` is found at startup — there is nothing to
register and nothing to rebuild.

The default theme is FAR's own palette, taken from its source rather than from a screenshot.
