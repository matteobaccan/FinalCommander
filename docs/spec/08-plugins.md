# SPEC 08 — Extensibility: commands, Lua, WASM, native

Three layers, one vocabulary. Everything a plugin can do, a key or a macro can do too, because
they all go through the command bus (`03-keymap.md` §10).

## 1. The command bus

```rust
pub trait CommandHandler { fn run(&self, ctx: &mut Ctx, args: Args) -> Result<Value>; }
```

`Ctx` exposes capability-scoped handles: `panels`, `selection`, `jobs`, `vfs`, `ui` (dialogs,
menus, status), `config` (read), `events`. Every command has an id, a label, a scope, an arity
and a declaration of the capabilities it needs. Commands are discoverable
(`app.palette`), scriptable, bindable and introspectable (`sc --list-commands --json`) — which
is also how the docs and the help pages are generated.

## 2. Lua (M3)

`mlua` (Lua 5.4), one VM, scripts in `scripts/`, loaded at startup and hot-reloadable.

```lua
-- scripts/git_branch.lua
sc.register_command("user.git.branch", {
  label = "Git: checkout branch",
  scope = "panel",
  run = function(ctx)
    local out = sc.shell.capture("git branch --format='%(refname:short)'", { cwd = ctx.panel.cwd })
    local pick = sc.ui.menu("Branch", sc.util.lines(out))
    if pick then sc.shell.run("git checkout " .. sc.util.quote(pick), { cwd = ctx.panel.cwd }) end
  end,
})

sc.on("panel.entered", function(ev)        -- event hooks
  if sc.fs.exists(ev.path .. "/Cargo.toml") then sc.panel.set_view("detail") end
end)

sc.keymap.bind("Alt+B", "user.git.branch") -- scripts may add bindings
```

API surface (stable, versioned `sc.api_version`): `sc.panel`, `sc.selection`, `sc.fs` (via VFS,
sandboxed to allowed roots), `sc.jobs` (enqueue a real job, get progress), `sc.ui` (menu,
input, message, progress, custom panel columns), `sc.shell`, `sc.util`, `sc.config` (read),
`sc.keymap`, `sc.on` (events: panel.entered, panel.changed, selection.changed, job.done,
file.opened, startup, shutdown), `sc.register_*` (command, column, preview, highlighter,
sorter).

Safety: scripts run on the UI thread only for short callbacks; anything long must use
`sc.async(fn)` which runs it on the worker pool. A script that blocks the UI for more than
500 ms is reported and can be killed; a script error shows a one-line banner plus a log entry,
never a crash.

## 3. WASM plugins (M3) — the real plugin ABI

Why WASM and not a native ABI first: portability (one artifact for Linux/macOS/Windows,
x86_64/aarch64), sandboxing (a bad plugin cannot corrupt our heap or read `~/.ssh`), and ABI
stability without matching compiler versions. MTN2 proves the concept is practical for a
commander.

- Host: `wasmtime`, WASI preview 2 components, one instance per plugin, fuel- and
  memory-limited, with an explicit capability grant per plugin.
- Interface described in WIT, versioned; the host exports what the plugin may call, the plugin
  exports the hooks it implements:

```wit
package sc:plugin@1.0.0;

interface host {
  record entry { name: string, size: u64, mtime: s64, kind: entry-kind, flags: u32 }
  log: func(level: level, msg: string);
  config-get: func(key: string) -> option<string>;
  ui-progress: func(done: u64, total: u64, item: string);
}

interface vfs-provider {                 // a plugin can add a scheme
  scheme: func() -> string;
  caps: func() -> u32;
  list: func(path: string, token: u64) -> result<list<entry>, error>;
  open-read: func(path: string) -> result<stream-id, error>;
  read: func(id: stream-id, len: u32) -> result<list<u8>, error>;
  write: func(id: stream-id, data: list<u8>) -> result<u32, error>;
  ...
}

interface viewer-provider { can-handle: func(name: string, head: list<u8>) -> bool;
                            render: func(path: string, geom: geometry) -> result<render, error>; }
interface command-provider { commands: func() -> list<command-desc>;
                             invoke: func(id: string, args: string) -> result<string, error>; }
interface column-provider  { columns: func() -> list<column-desc>;
                             value: func(col: string, path: string) -> option<string>; }
interface highlighter      { highlight: func(path: string, text: string) -> list<span>; }
```

- Plugin manifest (`plugin.toml`): id, name, version, api version, entry wasm file, declared
  capabilities (`vfs:net`, `vfs:fs:<root>`, `shell`, `clipboard`, `ui`), author, licence.
  Capabilities are shown to the user before the first load and stored as a grant.
- Lifecycle: discovery in `plugins/`, lazy load on first use, unload on idle, hot reload in dev
  mode. A plugin crash/trap disables it for the session with a readable message.
- Plugin-provided VFS schemes, panels, columns, viewers, previews, highlighters, commands and
  menu entries are all first-class — no second-class plugin UI.

## 4. Native plugins (M4)

A narrow, stable **C ABI** (`sc_plugin_v1`) for things WASM cannot do: OS integration (shell
extensions, MTP, kernel-ish filesystems), hardware, or performance-critical codecs. Loaded
only from a user-approved directory, with an explicit warning that native plugins run with full
process privileges. The same WIT-derived shapes are mirrored in the C header so a port either
way is mechanical.

We do **not** attempt binary compatibility with FAR's or TC's plugin ABIs (WCX/WFX/WLX):
different language, different memory model, and a licence minefield. What we can do, and plan
for M4, is a **bridge plugin** on Windows that loads TC packer plugins through their documented
ABI, written against the public specification only — kept as a separate optional component so
the core stays clean.

## 5. Macros

Recorded keystroke sequences (`Ctrl+.` to record) stored with a name, a scope and an optional
condition; replayable, bindable to a key, and editable as a list of commands (a macro is just a
command list, so a recorded macro can be promoted to a Lua script with one action). Guardrails:
macros that invoke destructive commands prompt once per playback unless marked `trusted`.

## 6. Plugin registry (M4)

A JSON index (git-hosted) listing plugins with their versions, checksums, capabilities and
source URLs; `sc plugin search/install/update/remove` verifies the checksum and signature,
installs into `plugins/<id>/<version>/`, and never auto-updates. Everything the registry
installs is WASM or Lua — never a binary blob.
