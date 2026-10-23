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
