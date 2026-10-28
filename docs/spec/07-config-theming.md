# SPEC 07 — Configuration, keymaps, themes, i18n

## 1. Files: one, plus one optional directory

FinalCommander is a **single executable**. Themes, keymap profiles, language catalogues and
help are compiled in, so the program is fully usable with nothing else on disk.

| Path | What | Required? |
| --- | --- | --- |
| `fcmd.toml` | **all** user settings: options, `[[theme]]` blocks, `[[keymap]]` blocks, filters, associations, user menu, remote sites (no secrets), terminal key fixes | no |
| `fcmd-state.toml` | remembered state beside it: histories, last session, panel paths, viewer positions | written only if there is something to remember |
| `plugins/` | the **only** directory ever read: one file per plugin | no |

Discovery, in order:

1. `--config <file>` / `--plugins <dir>` (and `$FCMD_PLUGINS`);
2. **portable**: an `fcmd.toml` next to the executable — a release zip or a USB stick needs no
   setup, and nothing is written outside that folder;
3. otherwise the per-OS config location, as a single file: `~/.config/fcmd.toml`,
   `~/Library/Application Support/fcmd.toml`, `%APPDATA%\fcmd.toml`. Note the absence of a
   directory of our own — there is one file to back up and one file to delete.

Each of these files is looked for under its own name first and under the name it had before
the program was renamed after that (`scmd.toml`, `scmd-state.toml`, `scmd-macros.toml`,
`scmd-menu.toml`, `scmd-plugins/`). A configuration somebody already has is not something to
lose over a rename: the old file is read, and written back to, and the startup screen says
once which file that was and what it should be called.

`fcmd --paths` prints all three locations and says whether each exists. `fcmd --dump-config` prints
the effective configuration (including the themes it carries) with provenance
(`default` / `fcmd.toml` / `--set`), which is what a bug report should include.
Sections inside `fcmd.toml`: `[general]`, `[panel]`, `[operations]`, `[viewer]`, `[editor]`,
`[ui]`, `[terminal]`, `[[theme]]`, `[[keymap]]`, `[[filter]]`, `[[highlight]]`, `[[assoc]]`,
`[[menu]]`, `[[site]]`, `[keys.local]`.

## 2. fcmd.toml (shape, with defaults)

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
show_hidden = true
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
syntax = true                   # colour the languages and markups we recognise
clock = false
language = "auto"
clipboard = true                # ask the terminal to carry the clipboard (OSC 52, SPEC 06 §11)
table = true                    # a .csv, .jsonl or .dbf opens as a table (SPEC 05 §1.11)
file_history = 512              # how many read-or-edited files Alt+F11 keeps

[terminal]
shell = "auto"
pty = true
cwd_sync = true
output_lines = 10000
kitty_keyboard = "auto"
```

CLI overrides: `--set panel.show_hidden=true`, `--theme`, `--keymap`, `--config`, `--portable`.
Environment: `FCMD_CONFIG_DIR`, `FCMD_THEME`, `FCMD_KEYMAP`, `NO_COLOR`, `COLORTERM`.

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
`fcmd.toml` wins over a default for the same mask. `{file}` is substituted; a rule that forgets
it still gets the path appended.

## 2b-bis. `[[highlight]]` groups

Which rows the panel paints differently, and how (SPEC 02 §5). A group is a condition and an
ink; the first group that matches a row decides, and FAR's built-in six come after all of them.

```toml
[[highlight]]
name = "Rust"             # named, so a warning about it can say which group
mask = "*.rs"             # a mask list, with the groups a filter knows: `<arc>`, `a|b`
fg = "#d7875f"            # #rrggbb, a colour name, idx:N, ansi:N — or a theme key
bg = "idx:235"            # left out, the row keeps the panel's background

[[highlight]]
name = "Fresh"
age = "<=1"               # whole days: <=1, >=30, 7-30
date_field = "create"     # write (the default), create, access, change
fg = "file.archive"       # a theme key follows the theme when it is switched

[[highlight]]
name = "Big and hidden"
size = "1G-"              # 1M-2G, -4K, 1G-; a lone 1M is a lower bound
attrs = "h-s"             # h r x s b, each one optionally after a `-` for "must not"
kind = "file"             # file | dir | both (the default)
fg = "red"
```

A group with no condition is refused — it would paint every row — and one that does not parse
is a warning on the startup screen rather than a reason to refuse to start.

## 2c. `[net]` and `[[site]]`

The netbox's settings and its site list (SPEC 04 §6). No secret ever appears here: a site says
*how* to authenticate and the password or passphrase is asked for when the connection needs it.

```toml
[net]
known_hosts = ""          # empty = ~/.ssh/known_hosts
keepalive_secs = 30       # 0 turns it off
fetch_limit_mb = 512      # the largest remote file `F3` will fetch into a temporary copy

[[site]]
id = "jump"               # shown in the list (Ctrl+Shift+N) and usable by name
host = "jump.example.net"
port = 2222               # 22 when left out
user = "mb"               # the local account name when left out
auth = "agent"            # agent | key:~/.ssh/id_ed25519 | ask
start_dir = "/srv"
group = "work"            # a heading in the site list

# One block per object storage service (SPEC 04 §8). The name is what a path calls it:
# `s3://work@bucket/2024/`. There is deliberately no place here for an access key — a secret
# key is a password, and this file is not where passwords live (SPEC 04 §8.5).
[[s3]]
name = "work"             # empty name = the default, reached by `s3://bucket/`
endpoint = ""             # empty = AWS itself; else `https://minio.lan:9000`
region = "eu-west-1"      # empty = $AWS_REGION, then $AWS_DEFAULT_REGION, then us-east-1
path_style = false        # left out = by host for AWS, by path for everything else
credentials = "env"       # env | aws | aws:<profile> | anonymous
listing_cap = 100000      # rows a listing with no delimiter may gather; 0 = no limit
```

## 2c-quater. `[compare]`

What counts as the same file when the two panels are compared (SPEC 03 §11). The comparison
dialog writes exactly these keys.

```toml
[compare]
case = false              # match the names with their case
size = true               # a different size is a difference
time = true               # a different time is a difference
seconds = 0               # times this many seconds apart are the same moment
contents = false          # read the bytes
when_time_differs = false # read them only where the times differ
fragment_kb = 0           # read only the first so many KiB; 0 = all of it
ignore_eol = false        # line endings alone are not a difference
selected_only = false     # compare only what is selected
stop_on_first = false     # stop at the first difference
newer_only = true         # select only the newer copy, not both sides
subdirs = false           # compare the whole of both trees (SPEC 03 §11b)
depth = 0                 # how deep when it does; 1 = the two directories, 0 = all the way
```

## 2c-ter. `[cmdline]`

```toml
[cmdline]
# The prompt (SPEC 06 §7.0b). FAR's letters, plus `$r` for ` (branch)` in a git repository and
# `%GITBRANCH%` for the bare branch name.
prompt = "$p$r$g "
```

## 2c-bis. `[drives]`

Which columns the change-drive dialog shows and which drives it lists (SPEC 10 §4.3). FAR keeps
the same settings in one `DriveMenuMode` word; this is that word written out, and the dialog's
own `Ctrl`-and-a-digit keys write straight into it.

```toml
[drives]
show_type = true          # Ctrl+1  the kind of thing it is
show_device = false       # Ctrl+2  what it is a mount of
show_label = true         # Ctrl+3  the volume's own name
show_fs = true            # Ctrl+4  the filesystem
show_size = true          # Ctrl+5  the total and the free space
show_removable = true     # Ctrl+6  list removable drives at all
show_panels = true        # Ctrl+7  list the panels that are not drives
show_optical = true       # Ctrl+8  list optical drives at all
show_network = true       # Ctrl+9  list network shares at all
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
| Interface | `ui.key_bar`, `ui.status_bar`, `ui.syntax`, `ui.keymap` |
| Confirmations | `general.confirm_delete`, `general.confirm_overwrite`, `general.confirm_exit` |

`Save setup` (`Shift+F9`) writes the whole configuration back out, themes declared in the file
included, and what it writes parses back into the same configuration.

## 3. Keymaps

```toml
# a [[keymap]] block inside fcmd.toml
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
reported at load time with the file:line of both bindings; `fcmd --check-keymap` validates and
prints the resolved table (also used to generate the help pages).

Modal profile (`vi`) adds `mode = "normal" | "insert" | "visual"` to a binding's scope, with
pending-key timeouts. The keymap engine therefore supports: single keys, modifiers, chords,
ranges, and modal states — nothing else (no scripting in the keymap).

## 4. Themes

```toml
id = "commander"
name = "Commander"
author = "FinalCommander"
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
  colours), `viewer.*` and `editor.*` (body, status line, selection — `Viewer.Text`,
  `Viewer.Status`, `Viewer.Text.Selected` and the editor's three, which FAR keeps apart from
  the panel's and which the hex pane shares with the editor), `keybar.*`, `status.*`,
  `cmdline.*`, `hmenu.*` (the menu bar), `menu.*` (a
  pulled-down menu or a list: text, frame, title, selection, shortcut, dim), `dialog.*`
  (text, frame, title, input fields, buttons, the chosen button), `warn.*` (a dialog that
  reports a failure), and `error.fg` / `filter.fg` / `search.fg`.
- **The full-screen views are not the panel.** `F3` and `F4` have their own body, status line
  and selection, because FAR's palette does: `Viewer.Text` is `F_LIGHTCYAN|B_BLUE` while
  `Viewer.Status` and `Viewer.Text.Selected` are both `F_BLACK|B_CYAN` — the key bar's colour,
  not the panel header's. Drawing the title with `panel.header.fg` over `status.bg`, which is
  what this did until 0.1.2, mixed two roles belonging to other things and came out wrong in
  every theme where those two happen not to match.
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

- **External themes** are `[[theme]]` blocks in `fcmd.toml` (or themes a plugin declares). They
  are read at startup and hot-reloaded when the file changes; a block with the id of a built-in
  overrides it, so a user can retune `commander` without renaming it. `ui.theme = "<id>"`
  selects, `app.theme.next` cycles through everything available, and `fcmd --list-themes` prints
  the resolved list with where each came from (`built-in`, the config file, or `plugin:<id>`).
- A theme file may `inherit = "<id>"` and override single keys; unknown keys are a warning, not
  a failure, so a theme written for a later version still loads.
- `fcmd --export-theme <id>` prints a built-in theme as a ready-to-paste `[[theme]]` block
  (`fcmd --export-theme tron >> ~/.config/fcmd.toml`).
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
  an id; `fcmd --i18n-lint` finds missing/unused ids in CI.
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

- `fcmd import --from far|tc|mc|dc <path>` reads what is safely readable from another
  commander's *user* configuration (its keymap assignments, user menu entries, associations,
  bookmark/hotlist paths, FTP site names) and produces our TOML, reporting everything it could
  not map. This touches the user's own data, not the vendor's: no proprietary file format is
  reverse engineered beyond what is needed to read the user's settings, and anything ambiguous
  is skipped with a note.
- Config version field + automatic migration on upgrade, with a backup of the previous file.
