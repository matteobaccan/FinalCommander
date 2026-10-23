# SPEC 07 — Configuration, keymaps, themes, i18n

## 1. Files: one, plus one optional directory

SupremeCommander is a **single executable**. Themes, keymap profiles, language catalogues and
help are compiled in, so the program is fully usable with nothing else on disk.

| Path | What | Required? |
| --- | --- | --- |
| `sc.toml` | **all** user settings: options, `[[theme]]` blocks, `[[keymap]]` blocks, filters, associations, user menu, remote sites (no secrets), terminal key fixes | no |
| `sc-state.toml` | remembered state beside it: histories, last session, panel paths, viewer positions | written only if there is something to remember |
| `plugins/` | the **only** directory ever read: one file per plugin | no |

Discovery, in order:

1. `--config <file>` / `--plugins <dir>` (and `$SC_PLUGINS`);
2. **portable**: an `sc.toml` next to the executable — a release zip or a USB stick needs no
   setup, and nothing is written outside that folder;
3. otherwise the per-OS config location, as a single file: `~/.config/sc.toml`,
   `~/Library/Application Support/sc.toml`, `%APPDATA%\sc.toml`. Note the absence of a
   directory of our own — there is one file to back up and one file to delete.

`sc --paths` prints all three locations and says whether each exists. `sc --dump-config` prints
the effective configuration (including the themes it carries) with provenance
(`default` / `sc.toml` / `--set`), which is what a bug report should include.
Sections inside `sc.toml`: `[general]`, `[panel]`, `[operations]`, `[viewer]`, `[editor]`,
`[ui]`, `[terminal]`, `[[theme]]`, `[[keymap]]`, `[[filter]]`, `[[highlight]]`, `[[assoc]]`,
`[[menu]]`, `[[site]]`, `[keys.local]`.

## 2. sc.toml (shape, with defaults)

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
theme = "commander"
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
# a [[keymap]] block inside sc.toml
[[keymap]]
name = "mine"
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
id = "commander"
name = "Commander"
author = "SupremeCommander"
variant = "dark"
inherit = ""          # optional: start from another theme and override keys below

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
- **Built-in themes (compiled in, always available, no files needed):**

  | id | look |
  | --- | --- |
  | `commander` | the classic orthodox blue: dark blue panels, cyan text, yellow selection, cyan key bar — the default |
  | `black` | near-monochrome dark: black background, grey text, white cursor; for low-contrast terminals and e-ink |
  | `white` | light theme: white background, dark text, blue accents; readable in daylight and on projectors |
  | `matrix` | green phosphor on black, bright green cursor, dim green chrome |
  | `tron` | neon on black: cyan and orange accents, thin bright highlights |
  | `blade` | amber/teal on near-black, warm selection — "blade" |

- **External themes** are `[[theme]]` blocks in `sc.toml` (or themes a plugin declares). They
  are read at startup and hot-reloaded when the file changes; a block with the id of a built-in
  overrides it, so a user can retune `commander` without renaming it. `ui.theme = "<id>"`
  selects, `app.theme.next` cycles through everything available, and `sc --list-themes` prints
  the resolved list with where each came from (`built-in`, the config file, or `plugin:<id>`).
- A theme file may `inherit = "<id>"` and override single keys; unknown keys are a warning, not
  a failure, so a theme written for a later version still loads.
- `sc --export-theme <id>` prints a built-in theme as a ready-to-paste `[[theme]]` block
  (`sc --export-theme tron >> ~/.config/sc.toml`).
- Themes are re-derived from published palettes, never copied from another application's theme
  files (`01-licensing.md` §4).
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
