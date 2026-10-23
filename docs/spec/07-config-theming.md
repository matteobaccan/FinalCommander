# SPEC 07 — Configuration, keymaps, themes, i18n

## 1. Files

```
config.toml          general settings
keymaps/far.toml     shipped profiles: far, mc, tc, nc, vi
keymaps/user.toml    user overrides (inherit = "far")
keys.local.toml      machine-specific terminal key fixes (written by "learn keys")
themes/*.toml        colour themes
filters.toml         named filter sets + highlight groups
assoc.toml           file associations and preview commands
menu.toml            user menu (F2)
sites.toml           remote sites (no secrets)
history/*.jsonl      histories
session.toml         restored session
```

Everything is TOML, commented, and regenerable: `sc --dump-config` prints the effective
configuration with defaults and provenance (`default` / `config.toml:42` / `--set`), which is
also what bug reports should include.

## 2. config.toml (shape, with defaults)

```toml
[general]
confirm = { delete = true, delete_nonempty = true, overwrite = true, exit = false,
            drag_drop = true, esc_closes_panel = false }
portable = false
update_check = false            # never on by default
single_instance = false

[panel]
view = "full"
sort = { key = "name", reverse = false, dirs_first = true, selected_first = false,
         natural = true, case_sensitive = "auto", chain = ["name"] }
show_hidden = false
allow_reverse_sort = true
mouse = true
quick_search = "alt"            # alt | plain | off   (FAR vs mc behaviour)
dir_size_on_enter = false
watch = true
columns_brief = 3

[operations]
copy_buffer = "auto"
preserve = ["mtime", "mode"]    # + atime, btime, owner, xattr, acl, hardlinks
verify = false
on_conflict = "ask"
concurrency = { local = 1, remote = 4 }
delete_to_trash = true
wipe_passes = 1

[viewer]
wrap = "off"                    # off | char | word
tab_width = 8
detect_dump = true
max_line = 10000

[editor]
tab_width = 4
expand_tabs = true
auto_indent = true
trim_trailing = false
keep_eol = true
max_open_size = "256MiB"
undo_depth = 10000

[ui]
theme = "far-classic"
keymap = "far"
icons = false                   # nerd-font glyphs
menu_bar = "auto"               # auto | always | never
key_bar = true
clock = false
language = "auto"

[terminal]
shell = "auto"
pty = true
cwd_sync = true
output_lines = 10000
kitty_keyboard = "auto"
```

CLI overrides: `--set panel.show_hidden=true`, `--theme`, `--keymap`, `--config`, `--portable`.
Environment: `SC_CONFIG_DIR`, `SC_THEME`, `SC_KEYMAP`, `NO_COLOR`, `COLORTERM`.

## 3. Keymaps

```toml
inherit = "far"
# scope: global | panel | viewer | editor | dialog | terminal
[[bind]]
keys = ["F5"]                    scope = "panel"   cmd = "file.copy"
[[bind]]
keys = ["Ctrl+Shift+P"]          scope = "global"  cmd = "app.palette"
[[bind]]
keys = ["F12", "5"]              scope = "panel"   cmd = "file.copy"        # chord fallback
[[bind]]
keys = ["Alt+1".."Alt+9"]        scope = "panel"   cmd = "nav.bookmark.go($1)"
[[bind]]
keys = ["Ctrl+G"]                scope = "panel"   cmd = "shell.apply_to_selection"
[[unbind]]
keys = ["Ctrl+Q"]                scope = "global"
```

Rules: a command may have several bindings; a binding may carry arguments; conflicts are
reported at load time with the file:line of both bindings; `sc --check-keymap` validates and
prints the resolved table (also used to generate the help pages).

Modal profile (`vi`) adds `mode = "normal" | "insert" | "visual"` to a binding's scope, with
pending-key timeouts. The keymap engine therefore supports: single keys, modifiers, chords,
ranges, and modal states — nothing else (no scripting in the keymap).

## 4. Themes

```toml
name = "far-classic"
author = "SupremeCommander"
variant = "dark"

[palette]                        # named colours, reused below
blue = "#000080"
cyan = "#00aaaa"
white = "#c0c0c0"
bright = "#ffffff"

[ui]
panel.bg = "blue"
panel.fg = "cyan"
panel.header.fg = "bright"
cursor.bg = "cyan"
cursor.fg = "blue"
selected.fg = "#ffff55"
keybar.fg = "black"
keybar.bg = "cyan"
dialog.bg = "white"
dialog.fg = "black"
error.fg = "#ff5555"

[file]                           # semantic file colours, used by highlight groups
dir = "bright"
exec = "#55ff55"
symlink = "cyan"
archive = "#ff55ff"
hidden = "#808080"
```

- Colours accept hex, `palette` names, 256-index, and `ansi:N` for 16-colour terminals; the
  backend degrades automatically and themes may declare a `[fallback16]` section.
- Shipped themes: `far-classic`, `nc-classic`, `mc-classic`, `tc-light`, `gruvbox`,
  `tokyo-night`, `dracula`, `nord`, `solarized-dark`, `solarized-light`, `high-contrast`,
  `mono`. Re-derived from published palettes, not copied from other apps' theme files
  (`01-licensing.md` §4).
- `app.theme.next` cycles; a theme change is hot-reloaded, as is any config file change
  (watched), with parse errors shown as a non-blocking banner and the previous value kept.

## 5. Internationalisation

- `fluent` catalogues per language in `i18n/<lang>/*.ftl`, compiled in, plus an override
  directory so a translator can iterate without rebuilding (MTN2's approach).
- Shipped languages: **English** (source), **Italian**, **German**, **French**, **Spanish**,
  **Russian**, **Chinese (Simplified, `zh-Hans`)**. Language is chosen in the settings or with
  `ui.language`/`--lang`; `auto` follows the OS locale and falls back to English. Adding a
  language means dropping an `i18n/<lang>/` directory next to the binary — no rebuild.
- Rules: no string concatenation in code; plurals and gender through fluent; every message has
  an id; `sc --i18n-lint` finds missing/unused ids in CI.
- Russian and Chinese drive two requirements that are not optional: full Unicode input/display
  (no codepage assumptions anywhere) and correct double-width layout for CJK. Both are tested
  per language in the snapshot suite, and German/French/Spanish strings are the ones that break
  narrow dialogs, so every dialog has a long-string snapshot test.
- Width-aware layout: labels are measured in terminal columns, dialogs auto-size, and
  truncation happens on grapheme boundaries. CJK/wide and combining characters are tested with
  a dedicated snapshot suite.
- Date/time/size formats follow the locale with an explicit override
  (`ui.date_format = "%Y-%m-%d %H:%M"`, `ui.size_units = "binary" | "decimal"`).

## 6. Migration & compatibility helpers

- `sc import --from far|tc|mc|dc <path>` reads what is safely readable from another
  commander's *user* configuration (its keymap assignments, user menu entries, associations,
  bookmark/hotlist paths, FTP site names) and produces our TOML, reporting everything it could
  not map. This touches the user's own data, not the vendor's: no proprietary file format is
  reverse engineered beyond what is needed to read the user's settings, and anything ambiguous
  is skipped with a note.
- Config version field + automatic migration on upgrade, with a backup of the previous file.
