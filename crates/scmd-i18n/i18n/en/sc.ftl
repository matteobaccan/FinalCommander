# Key bar labels: short, they must fit in a few columns.
key-help = Help
key-menu = Menu
key-view = View
key-edit = Edit
key-copy = Copy
key-move = RenMov
key-mkdir = MkDir
key-delete = Delete
key-pulldown = PullDn
key-quit = Quit

# Panel chrome
panel-empty = (empty)
panel-up = ..
panel-loading = reading…
panel-filter = filter: { $pattern }
panel-totals = { $files } files, { $bytes }
panel-selected = { $files } of { $total } selected, { $bytes }
panel-reading-error = cannot read this directory: { $error }

# Prompts
prompt-select-mask = Select files by mask:
prompt-deselect-mask = Deselect files by mask:
prompt-goto = Change directory to:
prompt-filter = Show only names containing:
prompt-search = Quick search:

# Dialogs
dialog-ok = OK
dialog-cancel = Cancel
dialog-yes = Yes
dialog-no = No
dialog-error = Error
dialog-help-title = Keyboard reference — { $profile }
dialog-help-hint = Esc or F1 closes this page · ↑↓ PgUp PgDn to scroll

# Status messages
msg-not-implemented = { $command } is not implemented in this build yet
msg-selected = { $count } entries selected
msg-deselected = { $count } entries deselected
msg-mask-invalid = invalid mask: { $error }
msg-theme = theme: { $name }
msg-sort = sort: { $key }{ $reverse }
msg-view = view: { $mode }
msg-hidden-on = hidden files shown
msg-hidden-off = hidden files hidden
msg-no-history = no more history
msg-dirsize = { $path }: { $bytes } in { $files } files

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Copy { $what } to:
prompt-move-to = Move/rename { $what } to:
prompt-mkdir = Create the directory:
what-selected = { $count } selected items
confirm-delete-title = Delete
confirm-delete-body = Delete { $count } items ({ $bytes })?
confirm-delete-hint = Enter deletes · Esc cancels
confirm-yes-no = [ Enter = yes ]   [ Esc = no ]
job-copy = Copying
job-move = Moving
job-delete = Deleting
job-mkdir = Creating directory
job-planning = planning…
job-counters = { $files } / { $filestotal } files · { $bytes } / { $bytestotal }
job-hint = Esc cancels
job-done = { $kind }: { $files } files, { $bytes }
job-done-renamed = { $kind }: { $files } files moved without copying
job-done-skipped = , { $skipped } skipped
job-done-errors = , { $errors } errors
job-cancelled = { $kind } cancelled
job-errors-title = Errors
conflict-title = The destination exists
conflict-dst = destination  { $path }   { $bytes }
conflict-src = source       { $path }   { $bytes }
conflict-newer = the source is newer
conflict-older = the destination is newer
btn-overwrite = Overwrite
btn-overwrite-all = All
btn-skip = Skip
btn-skip-all = Skip all
btn-retry = Retry
btn-cancel = Cancel
msg-nothing-to-do = nothing selected

# Viewer (F3)
key-wrap = Wrap
key-hex = Hex
key-search = Search
key-code = Code
key-follow = Follow
viewer-prompt-search = Search for ({ $mode }):
viewer-prompt-goto = Go to offset, or NN% :
viewer-not-found = `{ $query }` not found
viewer-follow-on = following the end of the file
viewer-follow-off = follow mode off
viewer-codepage = codepage: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = no earlier position
viewer-empty = (empty file)
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
key-decomp = Decomp
viewer-disasm-failed = cannot disassemble { $name }: { $error }
viewer-decompile-running = { $tool } is running on { $name }…
viewer-decompile-none = nothing here can decompile { $format }; install a tool and add a [[decompile]] rule to scmd.toml
viewer-decompile-failed = { $tool }: { $error }
viewer-derived = { $producer } of { $name }

# Editor (F4)
key-save = Save
key-undo = Undo
key-replace = Replac
editor-prompt-search = Search for:
editor-prompt-replace-from = Replace:
editor-prompt-replace-with = With:
editor-prompt-goto = Go to line, or line:column :
editor-prompt-save-as = Save as:
editor-saved = { $name }: saved, { $bytes } bytes
editor-nothing-to-save = { $name } has no unsaved changes
editor-not-found = `{ $query }` not found
editor-replaced = { $count } replacements
editor-read-only = { $name } is read-only
editor-readonly-on = read-only: edits are refused
editor-readonly-off = read-only off
editor-no-undo = nothing left to undo
editor-no-redo = nothing to redo
editor-discard-title = Unsaved changes
editor-discard-body = { $name } has unsaved changes. Leave and lose them?
editor-changed-title = Changed on disk
editor-changed-body = { $name } changed on disk since it was opened. Save over it?
editor-reloaded = { $name } reloaded from disk
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Connections
net-sites-hint = Enter connect · n new · d disconnect · Esc close
net-no-sites = No sites yet. Press `n` to type a URL, or add [[site]] blocks to the config.
net-prompt-url = Connect to:
net-prompt-password = Password for { $user }@{ $host }:
net-prompt-passphrase = Passphrase for the key of { $user }@{ $host }:
net-connecting = connecting to { $host }…
net-connected = connected to { $host }
net-disconnected = { $host } disconnected
net-not-connected = { $host } is not connected
net-local-panel = this panel is on a local filesystem
net-nothing-pending = no connection is waiting for that
net-hostkey-title = Unknown host key
net-hostkey-body = { $host } is not in known_hosts. Its fingerprint is:
net-hostkey-question = Record it and connect?
net-fetching = fetching { $name }…
net-viewing-copy = { $name }: a local copy, fetched for the viewer
net-too-big = { $name } is larger than { $limit }: raise net.fetch_limit_mb to fetch it
