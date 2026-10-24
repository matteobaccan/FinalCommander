# SPEC 07 — Configuration, keymaps, themes, i18n

## 1. Files: one, plus one optional directory

SupremeCommander is a **single executable**. Themes, keymap profiles, language catalogues and
help are compiled in, so the program is fully usable with nothing else on disk.

| Path | What | Required? |
| --- | --- | --- |
| `scmd.toml` | **all** user settings: options, `[[theme]]` blocks, `[[keymap]]` blocks, filters, associations, user menu, remote sites (no secrets), terminal key fixes | no |
| `scmd-state.toml` | remembered state beside it: histories, last session, panel paths, viewer positions | written only if there is something to remember |
| `plugins/` | the **only** directory ever read: one file per plugin | no |

Discovery, in order:

1. `--config <file>` / `--plugins <dir>` (and `$SCMD_PLUGINS`);
2. **portable**: an `scmd.toml` next to the executable — a release zip or a USB stick needs no
   setup, and nothing is written outside that folder;
3. otherwise the per-OS config location, as a single file: `~/.config/scmd.toml`,
   `~/Library/Application Supportmd.toml`, `%APPDATA%md.toml`. Note the absence of a
   directory of our own — there is one file to back up and one file to delete.

`scmd --paths` prints all three locations and says whether each exists. `scmd --dump-config` prints
the effective configuration (including the themes it carries) with provenance
(`default` / `scmd.toml` / `--set`), which is what a bug report should include.
Sections inside `scmd.toml`: `[general]`, `[panel]`, `[operations]`, `[viewer]`, `[editor]`,
`[ui]`, `[terminal]`, `[[theme]]`, `[[keymap]]`, `[[filter]]`, `[[highlight]]`, `[[assoc]]`,
`[[menu]]`, `[[site]]`, `[keys.local]`.

## 2. scmd.toml (shape, with defaults)

```toml
[general]
confirm = { delete = true, delete_nonempty = true, overwrite = true, exit = false,
            drag_drop = true, esc_closes_panel = false }
portable = false
# There is no `update_check`: a setting nobody reads is a setting that lies, and this one
# was a promise wearing a setting's clothes. The program opens no connection the user did not
# ask for; the only outbound code in the tree is the SFTP client behind `Alt+F1`.
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
Environment: `SCMD_CONFIG_DIR`, `SCMD_THEME`, `SCMD_KEYMAP`, `NO_COLOR`, `COLORTERM`.

## 2b. `[[decompile]]` rules

External tools the viewer may call for `F6` (SPEC 05 §1.6). A rule is used only when its
program is on the `PATH`, so the defaults can name tools that are not installed.

```toml
[[decompile]]
name = "javap"            # shown in the status line
match = "*.class"         # `*.ext`, or an exact file name
cmd = "javap -c -p -constants {file}"   # run directly, never through a shell
```

Compiled-in defaults, each active only if present: `javap` (`*.class`), `ilspycmd` (`*.dll`),
`python3 -m dis` (`*.pyc`), `wasm2wat` (`*.wasm`, though WebAssembly is printed in process
anyway), `objdump -d -C` (`*.o`, `*.so`, `*.a`), `retdec-decompiler` (`*.exe`). A rule in
`scmd.toml` wins over a default for the same mask. `{file}` is substituted; a rule that forgets
it still gets the path appended.

## 2c. `[net]` and `[[site]]`

The netbox's settings and its site list (SPEC 04 §6). No secret ever appears here: a site says
*how* to authenticate and the password or passphrase is asked for when the connection needs it.

```toml
[net]
known_hosts = ""          # empty = ~/.ssh/known_hosts
keepalive_secs = 30       # 0 turns it off
fetch_limit_mb = 512      # the largest remote file `F3` will fetch into a temporary copy

[[site]]
id = "jump"               # shown in the list (Alt+N) and usable by name
host = "jump.example.net"
port = 2222               # 22 when left out
user = "mb"               # the local account name when left out
auth = "agent"            # agent | key:~/.ssh/id_ed25519 | ask
start_dir = "/srv"
group = "work"            # a heading in the site list
```

## 2d. The keys the settings dialogs write (implemented today)

§2 above is the shape the configuration is heading for; what the binary reads today is flat and
smaller. The settings dialogs (SPEC 10 §3) edit exactly these keys, and they go in through
`Config::set`, the same function `--set key=value` calls, so the dialog can never write
something the file format does not accept:

| Dialog | Keys |
| --- | --- |
| System | `panel.natural_sort`, `panel.allow_reverse_sort`, `net.fetch_limit_mb`, `net.keepalive_secs` |
| Panel | `panel.show_hidden`, `panel.dirs_first`, `panel.selected_first`, `panel.reverse`, `panel.view`, `panel.sort`, `panel.quick_search` |
| Interface | `ui.key_bar`, `ui.status_bar`, `ui.keymap` |
| Confirmations | `general.confirm_delete`, `general.confirm_overwrite`, `general.confirm_exit` |

`Save setup` (`Shift+F9`) writes the whole configuration back out, themes declared in the file
included, and what it writes parses back into the same configuration.

## 3. Keymaps

```toml
# a [[keymap]] block inside scmd.toml
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
reported at load time with the file:line of both bindings; `scmd --check-keymap` validates and
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
- **Every built-in theme defines every key** — there is a test for it — so adding a key means
  adding it to all of them, and a theme can never be half-coloured. The groups are: `panel.*`
  (background, text, frame, the path in the header, the active panel's inverted title, column
  titles, footer, scrollbar), `cursor.*` and `selected.fg`, `file.*` (the semantic file
  colours), `keybar.*`, `status.*`, `cmdline.*`, `hmenu.*` (the menu bar), `menu.*` (a
  pulled-down menu or a list: text, frame, title, selection, shortcut, dim), `dialog.*`
  (text, frame, title, input fields, buttons, the chosen button), `warn.*` (a dialog that
  reports a failure), and `error.fg` / `filter.fg` / `search.fg`.
- Menus are not dialogs and do not borrow their colours: in FAR a dropdown is cyan with yellow
  shortcuts while a dialog is grey with cyan fields, and a theme has to be able to say both.
- A floating window **drops a shadow** on what is behind it — two columns to the right, one row
  below, which is FAR's geometry. The shadow darkens whatever is already there (RGB halved, or
  one step down the ANSI palette) instead of painting a colour of its own, so it works in every
  theme and in 16 colours.
- **Built-in themes (compiled in, always available, no files needed):**

  | id | look |
  | --- | --- |
  | `commander` | the classic orthodox blue: dark blue panels, cyan text, yellow selection, cyan key bar — the default |
  | `far` | FAR Manager's own default palette, element for element, from `far/palette.cpp` of `FarGroup/FarManager` (BSD-3-Clause): the sixteen console colours, a cyan bar on the active panel's path, yellow column titles, cyan menus with yellow shortcuts, light grey dialogs with cyan input fields, red warning dialogs. For a user who wants the screen they already know |
  | `black` | near-monochrome dark: black background, grey text, white cursor; for low-contrast terminals and e-ink |
  | `white` | light theme: white background, dark text, blue accents; readable in daylight and on projectors |
  | `matrix` | green phosphor on black, bright green cursor, dim green chrome |
  | `tron` | neon on black: cyan and orange accents, thin bright highlights |
  | `blade` | amber/teal on near-black, warm selection — "blade" |

- **External themes** are `[[theme]]` blocks in `scmd.toml` (or themes a plugin declares). They
  are read at startup and hot-reloaded when the file changes; a block with the id of a built-in
  overrides it, so a user can retune `commander` without renaming it. `ui.theme = "<id>"`
  selects, `app.theme.next` cycles through everything available, and `scmd --list-themes` prints
  the resolved list with where each came from (`built-in`, the config file, or `plugin:<id>`).
- A theme file may `inherit = "<id>"` and override single keys; unknown keys are a warning, not
  a failure, so a theme written for a later version still loads.
- `scmd --export-theme <id>` prints a built-in theme as a ready-to-paste `[[theme]]` block
  (`scmd --export-theme tron >> ~/.config/scmd.toml`).
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
  an id; `scmd --i18n-lint` finds missing/unused ids in CI.
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

- `scmd import --from far|tc|mc|dc <path>` reads what is safely readable from another
  commander's *user* configuration (its keymap assignments, user menu entries, associations,
  bookmark/hotlist paths, FTP site names) and produces our TOML, reporting everything it could
  not map. This touches the user's own data, not the vendor's: no proprietary file format is
  reverse engineered beyond what is needed to read the user's settings, and anything ambiguous
  is skipped with a note.
- Config version field + automatic migration on upgrade, with a backup of the previous file.
