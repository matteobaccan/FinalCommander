# Example plugins

A plugin is a folder with a `plugin.toml` in it, put in the plugins directory —
`scmd --paths` says where that is, and `scmd --plugins <dir>` points somewhere else for a
try. `scmd --list-plugins` says what was found and what each one brings.

| Folder | What it shows |
| --- | --- |
| `git-here/` | A `[[command]]` run as a process: the context on standard input, one action per line back |

## git-here

Three files' worth of plugin: a manifest naming one command, and a shell script that answers
it. `F11` in the panels lists it; choosing it says which branch the directory is on and how
many files have changed.

```sh
cp -r examples/plugins/git-here "$(scmd --paths | awk '/^plugins/ {print $2}')"/
chmod +x .../git-here/status.sh      # the host runs it, so it has to be executable
```

The script is the whole of the plugin API at this tier, and it is worth reading because it is
short:

- the context arrives as **JSON on standard input** — `cat >/dev/null` is there so the write
  end does not get a broken pipe — and the same facts arrive in the **environment**, which is
  what a shell script actually wants: `SCMD_DIR` is the panel's directory, `SCMD_CURSOR` the
  row under the cursor, `SCMD_SELECTED` the selected names one per line;
- what it writes back is **one action per line**: `{"status": "…"}` for the status bar,
  `{"run": "panel.reread"}` to run a command of the program's own vocabulary, `{"chdir": …}`,
  `{"message": …}`, `{"cursor": …}`, `{"select": […]}`;
- `needs = ["panel"]` is why `SCMD_DIR` is set at all. Nothing is passed that is not asked
  for, so a plugin that does not need the selection never sees it.

A path inside an archive or on a server has no directory on this machine, so `SCMD_DIR` is
simply absent there rather than holding something that cannot be opened — which is why the
script checks it before using it.

`docs/spec/08-plugins.md` §1b is the whole protocol, including what is refused at load and
why: a command id outside the `user.`/`plugin.` space, a program that leaves the plugin's own
directory, or a context nobody can give.

## Windows

The same manifest works; `run` points at a `.cmd` or a `.ps1` instead, and reads the
environment variables rather than the JSON. There is nothing platform-specific in the protocol
itself — that is the reason it is a process and not a library.
