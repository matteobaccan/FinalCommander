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
interface editor-provider  { can-handle: func(name: string) -> bool;
                             open: func(path: string) -> result<session-id, error>;
                             key: func(id: session-id, k: key) -> result<edit-effect, error>;
                             render: func(id: session-id, geom: geometry) -> result<render, error>;
                             save: func(id: session-id, path: string) -> result<_, error>; }
interface command-provider { commands: func() -> list<command-desc>;
                             invoke: func(id: string, args: string) -> result<string, error>; }
interface column-provider  { columns: func() -> list<column-desc>;
                             value: func(col: string, path: string) -> option<string>; }
interface highlighter      { highlight: func(path: string, text: string) -> list<span>; }
```

- **One plugin is one file.** The whole program is a single executable and the plugins folder
  is the only directory it reads, so a plugin must not need a directory either:
  `plugins/<id>.wasm` (a WASM component) or `plugins/<id>.lua` (a script). The manifest lives
  *inside* the file — a `sc.plugin` custom section in the component, or a leading
  `--[[ sc.plugin ... ]]` block in the script — and declares id, name, version, api version,
  platforms (`["*"]` for WASM/Lua), capabilities (`vfs:net`, `vfs:fs:<root>`, `shell`,
  `clipboard`, `ui`), author and licence. A subdirectory per plugin is accepted but never
  required, and nothing outside `plugins/` is ever read.
- Capabilities are shown to the user before the first load and the grant is stored in
  `sc-state.toml` (keyed by plugin id and file hash, so a changed plugin asks again).
- Lifecycle: `plugins/` is scanned at startup (`sc --list-plugins` prints what was found),
  plugins load lazily on first use, unload on idle, and hot-reload in dev mode. A plugin
  crash/trap disables it for the session with a readable message.
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
source URLs; `sc plugin search/install/update/remove` verifies the checksum and signature and
drops **one file** into `plugins/` (`<id>.wasm` or `<id>.lua`, version recorded in the
manifest and in `sc-state.toml`), never auto-updates, and never installs a native binary blob.
Removing a plugin is deleting one file.

## 7. Porting the existing plugin ecosystems (and why plugins must be portable too)

**We rewrite the plugins. We do not load FAR's or Total Commander's.** Their plugins are
Windows DLLs compiled against a C/C++ ABI, they call Win32 directly, and several are GPL or
closed source — so they are neither loadable nor relicensable on Linux/macOS. Binary
compatibility would also freeze our internals around a 1990s Windows API.

So portability is a property of the plugin *model*, not of a compatibility layer:

| Tier | Form | Portability | Who writes it |
| --- | --- | --- | --- |
| **Core** | Rust inside the workspace | builds everywhere we build | us |
| **First-party plugin** | WASM component (or Lua script) in this repo, MIT | **one artifact, every OS and CPU** | us |
| **Third-party plugin** | WASM component or Lua script | same single artifact | anyone |
| **Native plugin** (M4, last resort) | one `cdylib` in the same folder, against the `sc_plugin_v1` C ABI | per-platform, declared in the manifest | only where WASM cannot reach the OS |

Rules that keep the promise:

1. **No core feature may depend on a non-portable plugin.** If something only works via a
   native plugin, it is an extra, never part of a milestone's definition of done.
2. A plugin manifest declares `platforms = ["*"]` for WASM/Lua, or an explicit list
   (`["windows-x86_64"]`) for native ones; the UI shows the difference and the registry filters
   by platform.
3. WASM plugins get no ambient authority: filesystem and network access go through the host's
   capability-scoped API (`vfs:net`, `vfs:fs:<root>`), so a plugin behaves identically on all
   three OSes — it never sees a drive letter or a `/proc` path unless we hand it one.
4. Anything genuinely OS-specific that we still want everywhere (trash, clipboard, drives,
   elevation, FS watch, PTY) lives in `sc-platform` in the core, not in plugins.

### Where the classic plugins land

| Classic plugin (FAR / TC / far2l) | Our destination | Why |
| --- | --- | --- |
| multiarc, arclite, zip/7z packers (WCX) | **core** VFS providers (`zip`, `tar`, `7z`, `libarchive` fallback) — SPEC 04 §3 | archives are table stakes, must work offline on every platform |
| NetRocks, FTP/SFTP/WebDAV/S3/SMB (WFX) | **core** VFS providers + connection manager | ditto; async networking belongs in the engine, not in a sandbox |
| TmpPanel | **core** (temp panel kind, SPEC 02 §7) | it is panel behaviour, not an add-on |
| Advanced compare, SyncDirs, Checksums, split/combine, multi-rename | **core** tools (SPEC 03, SPEC 06) | they drive the job engine |
| Colorer (syntax highlighting) | **core** tree-sitter highlighter + a `highlighter` plugin interface | grammars are data, loaded at runtime |
| Hexitor, ImageViewer, Markdown/Office viewers (WLX "Lister") | **first-party plugins** via `viewer-provider` / preview pipeline | optional, perfect WASM fit (pure transforms) |
| EditorComp, autowrap, align, drawline, editcase, SimpleIndent, truncate… | **Lua scripts** shipped in `scripts/` | tiny editor behaviours, hot-reloadable |
| Macros (FAR 3 / far2m Lua) | **core** macro engine + Lua API (§2, §5) | keys, menus and scripts share the command bus |
| ADB, MTP | **first-party plugin**; MTP may need a **native** one | `adb` is an external binary (portable); `libmtp` is not |
| Process list, registry, services | **first-party plugins** per platform, same `vfs-provider` interface | the interface is portable, the data source is not |
| Git status/branch panel | **first-party plugin** (WASM, shells out via the host) | optional, nice to have, no core coupling |
| Python plugin (far2l) | **not reimplemented**: Lua is our scripting runtime | one scripting VM is enough; a WASM plugin may embed any language |
| TC packer plugins (WCX) on Windows | **optional bridge** (M4), written against the published ABI only, Windows-only, clearly labelled non-portable | lets veterans keep exotic formats without polluting the core |

### Migration path for plugin authors

`sc plugin new <id> --kind vfs|viewer|column|command|highlighter` scaffolds a Rust project that
compiles to a WASM component against the versioned WIT interfaces, plus a `plugin.toml` and a
test harness that runs the VFS conformance suite against it. A FAR/TC plugin author therefore
ports *logic*, not glue: the interfaces are deliberately close in shape (list / open / read /
write for VFS, can-handle / render for viewers) so the port is mechanical.

## 8. The three plugins that matter first: viewer, editor, netbox

Priority decision. Of everything that can be a plugin, three carry the weight, and the plugin
layer is designed around them — if the ABI can host these, it can host anything:

| Plugin | Interface it implements | What it covers | Milestone |
| --- | --- | --- | --- |
| **`sc-viewer`** | `viewer-provider` + the preview pipeline | text / dump / hex, encodings, search, huge files, follow mode, per-type previews (SPEC 05 §1) | M2 |
| **`sc-editor`** | `editor-provider` | full-screen editor, column blocks, undo, search & replace, syntax highlighting (SPEC 05 §2) | M2 |
| **`sc-netbox`** | `vfs-provider` (one plugin, several schemes) | SFTP/SSH, FTP/FTPS, WebDAV, S3-compatible, plus the connection manager's site list (SPEC 04 §3, §6) — the FAR NetBox / far2l NetRocks role | M3 |

How they are built, in order:

1. **In-tree first, behind the public interface.** `sc-view`, `sc-edit` and `sc-net` are
   workspace crates that implement exactly the `viewer-provider` / `editor-provider` /
   `vfs-provider` shapes above, called through a trait object. They ship as part of the binary,
   so M2 and M3 do not wait for the WASM host.
2. **Then moved out, unchanged, as the three reference plugins.** The WASM ABI is **not
   considered done** until the viewer, the editor and netbox run as plugins through it, with no
   private back channel into the core. They are the acceptance test for the plugin layer, and
   the examples plugin authors read.
3. **Always replaceable, never removable.** A user may point `viewer`, `editor` or a scheme at
   a third-party plugin; if none is present, the built-in one is used. No configuration can
   leave the commander without a working `F3`, `F4` or network panel.

Consequences recorded elsewhere in the specs: the viewer/editor interfaces must be able to
express a full-screen component driven by keys (not just "render this file"), and `vfs-provider`
must support credentials, host-key prompts and long-lived connections — so the host exposes
`ui.prompt`, `keyring.get/put` and a connection-lifetime hook to plugins holding the
`vfs:net` capability.
