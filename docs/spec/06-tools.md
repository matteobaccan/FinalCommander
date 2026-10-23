# SPEC 06 — Tools, shell integration, dialogs

## 1. Find files (`Alt+F7`)

Dialog fields: name masks (include/exclude, comma-separated, `*`/`?`/`**`), content (plain /
regex / hex, case, whole word, encodings to try), size range, date range (absolute or relative
`<7d`), attributes/permissions predicate, depth limit, follow symlinks, search in archives,
search in the selected items only, use the predicate language (SPEC 02 §5) for anything else.

Execution: a parallel walker (`ignore`-crate-style, respecting `.gitignore` optionally) on the
rayon pool with a bounded result channel; results stream into a list that is usable while the
search runs. Actions on results: view, edit, go to the file in the panel, **panelize**
(turn the results into a panel — mc's feature), copy the list, run a job on the whole result
set, save/load the search.

`tool.find.folder` is the same dialog restricted to directories, with a "go to" action.

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

`menu.toml` entries with label, hotkey, condition and command list:

```toml
[[menu]]
key = "c"
label = "Build (cargo)"
when = "panel.has('Cargo.toml')"
run = ["shell.run('cargo build --release', wait=true, capture=true)"]

[[menu]]
key = "g"
label = "Git"
submenu = [
  { key = "s", label = "status", run = ["shell.run('git status -sb', wait=false)"] },
  { key = "l", label = "log",    run = ["shell.run('git log --oneline -40 | less')"] },
]
```

A directory may carry its own `.scmd-menu.toml`, merged above the global menu (FAR/mc local menus).

## 5. File associations (`assoc.toml`)

Per mask: `view`, `edit`, `execute`, `alt_view`, `alt_edit`, `extract`, plus an optional
`preview` entry (SPEC 05 §1.6). Macros available in commands: `{file}`, `{file.q}` (quoted),
`{name}`, `{ext}`, `{dir}`, `{other.dir}`, `{list}` (a temp file with the selection),
`{line}`, `{col}`. Default associations are derived from the OS where possible (xdg-mime,
`LSCopyDefaultRoleHandler`, registry) and overridable.

## 6. Histories

Command, directory, view/edit, search, rename pattern, and mask histories: persisted, size-capped,
searchable (type to filter), with pinning (FAR's `Ins` to lock an entry so it is not evicted) and
per-history clearing. Stored as JSONL under `$STATE/history/`.

## 7. Shell integration

| Feature | Design |
| --- | --- |
| Command line | always available under the panels; `Enter` runs in the integrated shell, `Shift+Enter` in a new OS terminal, `Ctrl+Alt+Enter` elevated |
| Macros | `!`/`%`-style substitutions documented in one table: current name, full path, other panel, selection list, cwd — expanded *before* the shell sees them, with quoting per platform |
| User screen | `Ctrl+O` shows the output of the last command full-screen; the output buffer is scrollable and searchable (`Ctrl+Shift+F`), capped in lines |
| Subshell | a persistent PTY (`portable-pty`) running `$SHELL` (or `cmd`/`pwsh` on Windows) with **bidirectional cwd sync**: we inject a prompt hook (`PROMPT_COMMAND`/`precmd`/`PS1` OSC 7) so a `cd` in the shell moves the panel, and `panel.cd` sends a `cd` to the shell |
| Terminal pane | `Ctrl+Alt+E` toggles an embedded terminal with full VT support (alternate screen, mouse, 24-bit colour) so `vim`/`htop` run inside it; multiple terminal tabs (M3) |
| Elevation | a privileged helper process (`scmd-helper`) started on demand via `pkexec`/`sudo`/UAC, speaking a minimal JSON protocol over a pipe, used only for explicitly elevated jobs; never started implicitly |
| Open with OS | `tool.os.open` (xdg-open/open/ShellExecute), `tool.os.terminal`, `tool.os.filemanager`, `tool.os.properties` |

## 8. Dialogs and common UI behaviour

- Every dialog: `Tab`/`Shift+Tab` navigation, `Enter` = default action, `Esc` = cancel, `F1` =
  context help for that dialog, history dropdown on text fields (`Ctrl+↑/↓`), completion for
  paths and masks, tri-state checkboxes, and mouse support.
- Dialogs are movable and resizable where it matters (viewer/editor search, job queue).
- Long operations inside dialogs (listing a remote dir for a path picker) are cancellable.
- The **command palette** (`Ctrl+Shift+P`) fuzzy-searches every command, shows its binding and
  scope, and is the discovery mechanism that replaces reading the manual.
- **Learn keys** dialog: press a key, see what the terminal sent and what `scmd` resolved it to;
  write a local override.

## 9. Help system

Offline, built from the docs: one page per topic with context links; `F1` opens the page for the
current screen/dialog; full-text search inside help; localisable (fluent + markdown sources
compiled into the binary). Help is generated from `docs/` so it cannot drift from the specs,
and the keymap pages are generated from the live keymap (never hand-written).

## 10. Session & state

`session.toml` holds panels, tabs, paths, view modes, sorts, filters, selection (optional),
viewer/editor positions, and the job journal pointer. `--no-session` starts clean;
`scmd path1 path2` opens the two panels at those paths; `scmd --goto file:line` opens the editor.
