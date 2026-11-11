# Key bar labels: short, they must fit in a few columns.
key-help = Help
key-menu = UserMn
key-view = View
key-edit = Edit
key-copy = Copy
key-move = RenMov
key-mkdir = MkFold
key-delete = Delete
key-pulldown = ConfMn
key-quit = Quit
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Add
key-extract = Extrct
key-arccmd = ArcCmd
key-edit-new = Edit…
key-rename = Rename
key-group = Group
key-selup = SelUp
key-left = Left
key-right = Right
key-sort-name = Name
key-sort-ext = Extens
key-sort-write = Write
key-sort-size = Size
key-sort-unsort = Unsort
key-sort-create = Creatn
key-sort-access = Access
key-sort = Sort
key-mklink = MkLink
key-find = Find
key-history = Histry
key-folders = FoldHs
key-plugins = Plugin

# Panel chrome
list-up = Up
list-folder = Folder
col-name = Name
col-ext = Ext
col-size = Size
col-allocated = Allocated
col-modified = Modified
col-created = Created
col-accessed = Accessed
col-attr = Attr
col-owner = Owner
col-group = Group
col-descr = Description
col-links = Lnk
col-git = Git
descr-title = Description of { $name }
descr-set = described { $name }
descr-cleared = description of { $name } taken away
descr-not-here = descriptions are kept beside a directory on this machine
col-date-time = Date       Time
panel-list-count = { $count } items in the list
panel-empty = (empty)
panel-loading = reading…

# Prompts
prompt-select-mask = Select files by mask:
prompt-deselect-mask = Deselect files by mask:
prompt-goto = Change directory to:
prompt-filter = File mask or part of a name:
prompt-search = Quick search:

# Dialogs
dialog-error = Error
dialog-help-hint = Esc goes back to the contents · F7 searches every chapter · F1 closes · ↑↓ PgUp PgDn to scroll
dialog-help-index = FinalCommander { $version } — Help
dialog-help-hint-index = Enter opens a chapter · F7 searches every chapter · Esc closes
help-search-prompt = Search the help
help-search-none = No line of the help holds “{ $query }”
help-search-title = “{ $query }” in the help: { $count } lines
output-search-prompt = Find in the output
output-search-empty = Nothing has been printed yet
output-search-found = Line { $line } of { $lines }
output-search-wrapped = Line { $line } of { $lines }, from the newest again
output-search-none = No line of the output holds “{ $query }”

# Status messages
msg-not-implemented = { $command } is not implemented in this build yet
msg-selected = { $count } entries selected
msg-deselected = { $count } entries deselected
msg-mask-invalid = invalid mask: { $error }
msg-theme = theme: { $name }
msg-font-bigger = font: a larger one asked of the terminal
msg-font-smaller = font: a smaller one asked of the terminal
msg-font-reset = font: the one it started with asked of the terminal
msg-sort = sort: { $key }{ $reverse }
msg-sort-groups-on = sort groups on ({ $count })
msg-sort-groups-off = sort groups off ({ $count })
msg-sort-groups-none = no sort groups are defined: see [[sort_group]] in the configuration
msg-view = view: { $mode }
msg-hidden-on = hidden files shown
msg-hidden-off = hidden files hidden
msg-no-history = no more history
msg-tab-last = a panel always has one tab
msg-tab-end = the tab is already at that end of the row
tabs-title = Tabs
msg-tab-moved = the tab moved to the other panel
msg-tab-locked = this tab is locked to its directory
msg-tab-locked-on = tab locked to its directory
msg-tab-locked-off = tab unlocked
msg-bookmark-set = shortcut { $digit }: { $path }
msg-bookmark-unset = shortcut { $digit } is not set
msg-bookmark-added = { $path } kept
msg-bookmark-removed = { $path } given up
msg-bookmark-known = that folder is already in the list
msg-dirsize = { $path }: { $bytes } in { $files } files
msg-dirsize-many = measuring { $count } folders…
usage-title = Disk usage: { $path } · { $size }
usage-not-here = the disk usage of a folder is measured on a disk, not inside an archive or a list of found files
msg-dirsize-no-folder = there is no folder here to measure

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Copy { $what } to:
prompt-move-to = Move/rename { $what } to:
prompt-mkdir = Create the directory:
prompt-tab-name = Call this tab:
prompt-bookmark-name = Call this folder:
what-selected = { $count } selected items
confirm-delete-title = Delete
confirm-delete-body =
    { $count ->
        [one] Delete { $name } ({ $bytes })?
       *[other] Delete { $count } items ({ $bytes })?
    }
prompt-link = Create a link to { $name } as:
link-created = { $name } created
confirm-wipe-title = Wipe
confirm-wipe-body =
    { $count ->
        [one] Overwrite and delete { $name } ({ $bytes })?
       *[other] Overwrite and delete { $count } items ({ $bytes })?
    }
confirm-wipe-honest = The bytes are overwritten once, then the file is unlinked. On an SSD, a copy-on-write filesystem or with snapshots, older copies may survive — and the name survives in the directory and the journal.
job-copy = Copying
job-move = Moving
job-delete = Deleting
job-wipe = Wiping
job-trash = Moving to the wastebasket
job-replace = Replacing
job-pack = Packing
job-rebuild = Writing the archive

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = File attributes
attr-count = { $count } selected items
attr-now = now { $text } ({ $octal })
attr-owner-read = Owner may read
attr-owner-write = Owner may write
attr-owner-exec = Owner may run
attr-group-read = Group may read
attr-group-write = Group may write
attr-group-exec = Group may run
attr-other-read = Everyone may read
attr-other-write = Everyone may write
attr-other-exec = Everyone may run
attr-setuid = Run as the owner (setuid)
attr-setgid = Run as the group (setgid)
attr-sticky = Only the owner may delete (sticky)
attr-readonly = Read-only
attr-hidden = Hidden
attr-system = System
attr-archive = Archive
attr-temporary = Temporary
attr-offline = Offline
attr-indexed = The indexer may look at it
attr-compressed = compressed
attr-encrypted = encrypted
attr-sparse = sparse
attr-reparse = a reparse point
attr-also = also { $what }
attr-mtime = Changed
attr-atime = Last read
attr-btime = Created
attr-ctime = Record changed
attr-btime-readonly = (the creation time can only be set on Windows)
attr-utc = Times in UTC
attr-owner = Owner (user:group)
attr-owner-is = owner { $who }
btn-attr-original = Original
btn-attr-current = Current
btn-attr-blank = Blank
attr-recursive = And everything inside the folders
attr-bad-time = `{ $text }` is not a date and time (YYYY-MM-DD HH:MM:SS)
attr-bad-owner = no account or group is called `{ $text }` here
attr-in-archive = changing this writes the archive again, with every other entry as it is
job-attrs = Setting attributes

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Copy
copy-title-move = Rename or move
copy-target = To
copy-conflict = If it is already there
copy-conflict-ask = ask
copy-conflict-overwrite = overwrite
copy-conflict-newer = overwrite when the source is newer
copy-conflict-skip = skip it
copy-conflict-rename = give the copy a free name
copy-conflict-append = add to the end of it
copy-only-newer = Only files newer than the ones there
copy-filter = Only the names that match
copy-follow-links = Copy what a link points at, not the link
copy-keep-times = Keep the times
copy-keep-mode = Keep the permissions
copy-verify = Read every file back and compare it
copy-several = { $count } destinations, one after the other

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = a terminal running { $shell }
term-hint = Ctrl+Alt+E: back to the panels
term-ended = it has ended — any key closes this
job-mkdir = Creating directory
job-planning = planning…
job-counters = { $files } / { $filestotal } files · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · { $elapsed } elapsed · { $eta } left
job-rates-no-eta = { $speed }/s · { $elapsed } elapsed
job-hint = Esc cancels
job-hint-queue = Esc cancels · B background
job-queued = { $kind } is in the queue, with { $ahead } other jobs not finished · Alt+Shift+F5 lists them
job-background = { $kind } goes on in the background · Alt+Shift+F5 lists the jobs
job-asks = { $kind } is waiting for an answer · Alt+Shift+F5
job-paused = { $kind } paused
job-resumed = { $kind } goes on
job-retry-nothing = Nothing to retry
job-still-waiting = That job is waiting for its turn
jobs-title = Jobs ({ $active } not finished)
jobs-hint = Enter show · Space pause · Del stop · F5 retry · Ctrl+↑↓ order · Esc close
jobs-quit-title = Jobs are still running
jobs-quit-body = These jobs stop if the program ends now:
job-state-running = running
job-state-paused = paused
job-state-asking = asking
job-state-queued = waiting
job-state-held = held
job-state-done = done
job-state-failed = errors
job-state-cancelled = cancelled
job-done = { $kind }: { $files } files, { $bytes }
job-done-renamed = { $kind }: { $files } files moved without copying
job-done-skipped = , { $skipped } skipped
job-done-verified = , { $verified } verified
job-done-unverified = , { $unverified } could not be read back here
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
btn-move = Move
btn-delete = Delete
btn-wipe = Wipe
btn-copy = Copy
btn-discard = Discard
btn-kill = Kill
btn-quit = Quit
btn-play = Play
btn-run = Run
btn-accept = Accept
btn-cancel = Cancel
msg-nothing-to-do = nothing selected
cmd-complete-nothing = nothing to finish that with
complete-several = { $count } names start that way
form-moving = the arrows move the dialog · Enter or Esc leaves it there
form-moved = the dialog stays where you put it
msg-trash-refused = The wastebasket would not take it — Shift+Del deletes it outright.

# Checksums (SPEC 03 §9)
sum-title = Checksums
sum-algorithm = Hash
sum-one-file = One list for all of them
sum-name = Called
sum-working = { $name }: { $done } of { $total }
sum-verifying = checking against { $name }
sum-stopped = stopped
sum-written = { $name } written
sum-no-name = the list needs a name
sum-local-only = a checksum can only read a file on this machine
sum-all-match = all { $count } match
sum-verify-title = Checksums
sum-verify-summary = { $ok } match, { $bad } do not
sum-differs = does not match
sum-missing = not there
sum-unreadable = cannot be read

# Synchronising two directories (SPEC 03 §10)
sync-title = Synchronise
sync-between = { $left }  ↔  { $right }
sync-recursive = Everything in the subdirectories too
sync-both-ways = Both ways, the newer one winning
sync-delete-extra = Offer to take out what the other side does not have
sync-by-content = Compare the bytes, not the size and the time
sync-filter = Only the names that match
sync-walking = looking at { $at } — { $pairs } so far
sync-planned = { $copies } to copy ({ $bytes }), { $deletes } to take out
sync-hint = Space the next answer · ← → which way · = leave it · F2 do it · Esc cancel
sync-hint-walking = Esc stops the walk
sync-nothing-to-do = every row says to leave things as they are
sync-local-only = a synchronisation can only read and write on this machine
sync-same-directory = both panels are in the same directory

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nothing here says how to open { $name }
usermenu-empty = No user menu yet: write one in { $path }
usermenu-last-gone = the row taken last is not in this menu any more
usermenu-bad-condition = Condition not understood — { $problem }
assoc-desktop = Hand it to the desktop
assoc-open-with = Open { $name } with
assoc-no-pager = no association and no $PAGER says what to read this with
assoc-no-editor = no association and no $VISUAL says what to write this with

# Viewer (F3)
key-wrap = Wrap
key-hex = Hex
key-text = Text
key-dump = Dump
key-doc = Doc
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
viewer-not-markdown = not a Markdown file, or too large to lay out
viewer-not-table = not a tabular file, or too large to read whole
viewer-base64-done = Decoded: { $bytes }
viewer-not-base64 = This file is not base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = cannot disassemble { $name }: { $error }
viewer-decompile-running = { $tool } is running on { $name }…
viewer-decompile-none = nothing here can decompile { $format }; install a tool and add a [[decompile]] rule to fcmd.toml
arc-needs-program = this archive is read by `{ $program }`, which is not installed on this machine
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame is of the file itself: go back to it first
viewer-derived = { $producer } of { $name }

# Editor (F4)
key-save = Save
key-replace = Replac
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
net-sites-hint = Enter connect · n URL · Ins store · F4 edit · d disconnect · Esc close
net-no-sites = No sites yet. Press `n` to type a URL, or add [[site]] blocks to the config.
net-not-a-stored-session = this row is an open connection, not a stored session
net-site-forgotten = { $name } is no longer stored
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
net-hostkey-changed-title = The host key has CHANGED
net-hostkey-changed-body = { $host } answered with a key that is not the one recorded.
net-hostkey-changed-where = The one on record is { $path }, line { $line }.
net-hostkey-changed-what = A rebuilt server looks exactly like somebody in the middle. Nothing is accepted here: open the file, read the line, and take it out yourself if you know why it changed.
btn-open-file = Open the file
net-fetching = fetching { $name }…
net-viewing-copy = { $name }: a local copy, fetched for the viewer
net-too-big = { $name } is larger than { $limit }: raise net.fetch_limit_mb to fetch it
net-editing = { $name }: editing a local copy; saving writes it back
net-writing-back = writing { $name } back…
net-written-back = { $name }: written back, { $bytes } bytes
net-write-back-failed = write-back failed: { $error } — your edits are in { $local }
net-changed-title = Changed on the server
net-changed-body = { $name } changed on the server since it was fetched. Write over it?
confirm-delete-permanent = This deletes permanently: there is no wastebasket for these.
confirm-trash-title = Move to the wastebasket
confirm-trash-body =
    { $count ->
        [one] Move { $name } ({ $bytes }) to the wastebasket?
       *[other] Move { $count } items ({ $bytes }) to the wastebasket?
    }
confirm-trash-where = They go where the desktop keeps deleted files, and can be put back from there.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Left
menu-right = &Right
menu-files = &Files
menu-commands = &Commands
menu-options = &Options
menu-brief = &Brief
menu-medium = &Medium
menu-full = &Full
menu-detailed = Detai&led
menu-owners = File own&ers
menu-links = File lin&ks
menu-alt-full = Full, si&zes in full
menu-descriptions = &Descriptions
menu-long-descriptions = Lon&g descriptions
menu-wide = &Wide
menu-info-panel = &Info panel
menu-quick-view = Quick &view
menu-tree-panel = &Tree panel
menu-tab-new = &New tab
menu-tab-close = Close tab
menu-tab-next = Ne&xt tab
menu-tab-prev = &Previous tab
menu-tab-move = Move t&he tab to the other panel
menu-tab-lock = Lock the tab to its director&y
menu-tab-name = Name this tab
menu-tab-list = All the tabs
menu-sort-modes = &Sort modes
menu-hidden = Hidden and system files
menu-filter = File filte&r
menu-filter-sets = Named filters
menu-panel-onoff = Panel &on/off
menu-reread = Re-re&ad
menu-change-drive = &Change drive
menu-view = &View
menu-edit = &Edit
menu-edit-new = Edit new &file
menu-copy = &Copy
menu-rename = &Rename or move
menu-link = Create lin&k
menu-multirename = Rename fr&om a pattern…
menu-multirename-undo = Undo the last multi-rename
menu-mkdir = &Make folder
menu-delete = &Delete
menu-wipe = &Wipe
menu-checksum = Checks&ums…
menu-checksum-verify = Check against a list
menu-split = Split file…
menu-combine = Combine files
menu-delete-permanent = De&lete permanently
menu-archive-add = Add &to archive
menu-archive-extract = E&xtract files
menu-archive-commands = Arc&hive commands
menu-archive-test = Test archive
menu-attributes = File &attributes
menu-apply-command = A&pply command
menu-replace-in-files = Replace in files…
menu-select-group = Select &group
menu-select-ext = Select &by this extension
menu-unselect-ext = Unselect b&y this extension
menu-unselect-group = U&nselect group
menu-invert-selection = &Invert selection
menu-select-all = Select all
menu-restore-selection = Re&store selection
menu-dirsize = Folder si&ze
menu-find-file = &Find file
menu-history = Command &history
menu-view-history = Vie&w and edit history
menu-folders-history = F&olders history
menu-usage = Dis&k usage
menu-jobs = &Jobs
menu-branch = Branch view
menu-bookmarks = Folder shortcuts
menu-swap-panels = &Swap panels
menu-sync-panel = S&ync other panel
menu-panels-onoff = &Panels on/off
menu-compare-folders = &Compare folders
menu-sync-folders = Sy&nchronise folders
menu-compare-contents = Compare &by content
menu-panelize = Panelize a command's output
menu-view-filtered = View through a command
menu-learn-keys = Learn keys
menu-tunnel = Open a &tunnel…
menu-tunnels = Tunn&els…
tunnel-title = Tunnel: [local:]host:port
tunnel-list-title = Tunnels
tunnel-none = no tunnel is open
tunnel-open = tunnel open: { $what }
tunnel-closed = tunnel closed
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = a tunnel goes over a connection: open one with Alt+N first
menu-connections = Connections
menu-disconnect = Disconnect
menu-user-menu = User &menu
menu-associations = File &associations
menu-folder-shortcuts = Fol&der shortcuts
menu-plugin-commands = Pl&ugin commands
menu-screens-list = Sc&reens list
menu-macro-record = Record a macro
menu-macro-list = Macros
menu-task-list = Task &list
menu-system-settings = S&ystem settings
menu-panel-settings = &Panel settings
menu-interface-settings = &Interface settings
menu-languages = Lan&guages
menu-themes = &Themes
menu-confirmations = Co&nfirmations
menu-viewer-settings = &Viewer settings
menu-editor-settings = &Editor settings
menu-codepages = Co&de pages
menu-sort-groups = S&ort groups
menu-mask-groups = &File masks
menu-settings-all = &All settings
menu-highlighting = Files &highlighting
menu-plugins-config = Pl&ugins configuration
menu-help = Help
menu-save-setup = &Save setup
menu-quit = &Quit
opt-natural-sort = Natural number order
opt-allow-reverse = Allow reverse sort
opt-select-folders = Masks select folders too
cmd-no-network-name = A path on this disk has no network name
opt-trash = Use the system recycle bin
opt-system-copy = Use the system copy routine
opt-save-history = Keep history between sessions
opt-history-lifetime = History lifetime, in days
opt-forever = for ever
opt-fetch-limit = Remote file size limit
opt-keepalive = SSH keep-alive
opt-off = off
opt-show-hidden = Show hidden and system files
opt-column-titles = Show the column titles
opt-status-line = Show the status line
opt-scrollbar = Show the scrollbar
opt-free-space = Show free space
opt-auto-refresh = Refresh the panel automatically
opt-dirs-first = Folders first
opt-selected-first = Selected files first
opt-reverse = Reverse order
opt-view = Panel view
opt-sort = Sort by
opt-quick-search = Quick search
opt-quick-alt = with Alt
opt-quick-plain = by letters
opt-key-bar = Show the key bar
opt-status-bar = Show the status bar
opt-syntax = Colour the languages we recognise
opt-markdown = Markdown laid out to be read
opt-table = Open a .csv or a .jsonl as a table
opt-keymap = Keymap
opt-clock = Clock in the corner
opt-mouse = Mouse support
opt-clipboard = Let the terminal carry the clipboard (OSC 52)
opt-command-screen = Show a command's output while it runs
opt-menu-always = Always show the menu bar
opt-save-exit = Save the setup on exit
opt-confirm-delete = Delete
opt-confirm-overwrite = Overwrite
opt-confirm-exit = Exit
opt-confirm-editor = Leaving the editor with unsaved changes
opt-applied = settings applied
opt-saved = saved to { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Not a yes-or-no setting: Enter types a value
opt-settings-hint = ↑↓ choose · type to narrow · Enter or F4 value · Space flip · Del default · Esc close
opt-language-set = language: { $name }
sort-name = Name
sort-ext = Extension
sort-size = Size
sort-mtime = Modified
sort-btime = Created
sort-atime = Accessed
sort-descr = Description
sort-owner = Owner
sort-git = Git status
key-sort-descr = Descr
key-sort-owner = Owner
sort-none = Unsorted
sort-set = sort: { $name }
btn-ok = OK
pick-hint = ↑↓ choose · type to filter · Enter apply · Esc close
pick-empty = (nothing to choose)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } is pinned · nothing pushes it out now
hist-unpinned = { $line } is no longer pinned
hist-is-pinned = { $line } is pinned · Ins unpins it first
hist-clear-body = Empty this history of everything that is not pinned?
hist-cleared = the history is empty but for what is pinned
history-hint = ↑↓ choose · Enter apply · Ins pin · Shift+Del one · Del all · Esc close


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = New connection
site-edit = Edit connection
site-name = Name
site-host = Host
site-port = Port
site-user = User
link-title = Create a link
link-name = The link
link-kind = Kind
link-kind-relative = symbolic, relative — survives the tree being moved
link-kind-absolute = symbolic, absolute — survives the link being moved
link-kind-hard = hard — a second name for the same file
menu-colors = Co&lours…
palette-title = Colours of { $theme }
palette-hint = Enter a colour · F2 save · Esc close
palette-ask = A colour for { $key }
palette-bad-color = { $value } is not a colour (a name, 0–255, or #rrggbb)
palette-saved = the colours were written into the { $theme } theme
palette-unsaved = the colours changed on the screen only — F2 writes them
opt-viewer-status-line = Show the top line (name and position)
opt-viewer-scrollbar = Show a scrollbar
opt-viewer-key-bar = Show the key bar
opt-viewer-wrap = Long lines
opt-viewer-wrap-off = do not wrap
opt-viewer-wrap-char = wrap at the edge
opt-viewer-wrap-word = wrap at a word
opt-viewer-tab-width = Tab width
opt-viewer-remember = Remember the position and the code page of each file
opt-editor-tabs = Tabs
opt-editor-tabs-keep = leave them as they are
opt-editor-tabs-typed = expand what is typed
opt-editor-tabs-all = expand them all when a file is opened
opt-editor-tab-width = Tab width
opt-editor-auto-indent = Keep the indent of the line above
opt-editor-whitespace = Show spaces and tabs
opt-editor-cursor-after = The cursor lands after the text a search found
opt-editor-beyond-eol = The cursor may go past the end of a line
opt-editor-trim-trailing = Take trailing blanks off when saving
editor-not-a-character = that key has no character to insert
paste-one-line =
    { $count ->
        [one] pasted the first line only · one more line left out
       *[other] pasted the first line only · { $count } more lines left out
    }
clip-names =
    { $count ->
        [one] one name on the clipboard
       *[other] { $count } names on the clipboard
    }
clip-paths =
    { $count ->
        [one] one path on the clipboard
       *[other] { $count } paths on the clipboard
    }
clip-line = the command line is on the clipboard
clip-selection = what was marked out is on the clipboard
clip-off = the clipboard is turned off — `ui.clipboard` in the settings
history-gone = { $name } is not there any more
hl-new = New colour group
hl-edit = Colour group
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Differences
menu-diff = &View file differences
diff-needs-two = Put the cursor on a file in each panel
diff-not-here = { $name } is not a file on this disk
diff-too-big = { $name } is larger than { $most }
diff-same = The two files are the same
diff-where = hunk { $at } of { $count } · -{ $removed } +{ $added } lines
diff-no-next = The last difference
diff-no-previous = The first difference
diff-hint = Alt+↑↓ hunks · Tab side · F5 take · F2 write · W space · C case · E line ends · Esc close
diff-hint-git = Alt+↑↓ hunks · S stage the hunk · Tab side · F5 take · F2 write · Esc close
diff-hint-git-staged = Alt+↑↓ hunks · S unstage the hunk · Tab side · F5 take · F2 write · Esc close
diff-stage-edited = the diff has merges not written: F2 first, then S
diff-merged-left = The left file took that difference
diff-merged-right = The right file took that difference
diff-no-hunk-here = No difference at the top of the screen — Alt+↓ puts one there
diff-nothing-to-write = Nothing has been merged
diff-written = { $count } written
diff-unwritten = A merge has not been written — F2 writes it, Esc again leaves it
diff-ignore-space-on = Blanks ignored
diff-ignore-space-off = Blanks count again
diff-ignore-case-on = Case ignored
diff-ignore-case-off = Case counts again
diff-ignore-eol-on = Line endings ignored
diff-ignore-eol-off = Line endings count again

grab-hint = ↑↓←→ move · Shift+↑↓←→ mark · Ctrl+A all · Enter copy · Esc close
grab-copied = { $rows } rows of the screen copied

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Code pages
cp-name-title = What to call this code page
cp-set = Reading as { $name }
menu-panel-names = Names code page…
names-title = Read the names as
names-system = As the system gives them
names-set = Names read as { $name }
names-system-set = Names as the system gives them
names-not-here = A name on this system is not bytes, so there is no second way to read it
cp-marked = { $name } is one F8 walks
cp-unmarked = { $name } is no longer one F8 walks
cp-hint = ↑↓ choose · Enter read as it · Ins mark for F8 · F4 rename · Esc close

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Mask group
mg-name = Name
mg-mask = Masks
mg-hint = Ins adds a group, Del takes one away, F4 edits · a `·` marks the three that are built in
mg-saved = <{ $name }> is in force; Save setup writes it
mg-removed = <{ $name }> is gone
mg-bad = <{ $name }>: { $problem }
mg-needs-name = a name without spaces, commas or angle brackets
mg-needs-mask = a group that stands for nothing matches nothing
mg-built-in = <{ $name }> is one of the built-in three · F4 gives it other masks instead

sg-title = Sort group
sg-name = Name
sg-mask = Masks
sg-place = Where
sg-place-front = in front of the rest
sg-place-back = behind the rest
sg-hint = Ins adds a band, Del takes one away, F4 edits, Ctrl+↑↓ moves one
sg-saved = The band is in force; Save setup writes it
sg-removed = { $name } is gone
sg-bad = { $name }: { $problem }
sg-needs-mask = a band with no mask would hold nothing
hl-name = Name
hl-mask = Mask or masks
hl-kind = Applies to
hl-kind-both = files and folders
hl-kind-file = files only
hl-kind-dir = folders only
hl-size = Size (1M-10M, -4K, 1G-)
hl-age = Age in days (<=1, >=30, 7-30)
hl-date-field = The age is about
hl-attrs = Attributes (h r x s b, - for must not)
hl-fg = Ink (#d7875f, red, file.archive)
hl-bg = Background, written the same way
hl-hint = ↑↓ choose · Enter/F4 edit · Ins new · Del remove · Ctrl+↑↓ move · Esc close
hl-saved = the group is in force · Options → Save setup writes it to the file
hl-removed = { $name } is gone · Options → Save setup writes it to the file
hl-bad = { $name }: { $problem }
filter-off = No filter: every name
filter-hint = ↑↓ choose · Enter use · F4 edit · Ins new · Del remove · Esc close
filter-new = New filter
filter-edit = Filter
filter-name-hint = with a name it is kept in the filters menu; without one it is used once
filter-applied = filter on: { $name }
filter-cleared = filter off
filter-saved = { $name } is on the panel and in the filters menu · Options → Save setup writes it to the file
filter-removed = { $name } is gone · Options → Save setup writes it to the file
editor-no-block = nothing is selected
editor-unique = { $count } repeated lines taken out
editor-no-bracket = no bracket here, or nothing matches it
editor-tabs-expanded = tabs were expanded on { $lines } lines — the save will write that
site-protocol = Protocol
site-protocol-sftp = SFTP (over SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP with TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV over HTTPS
site-auth = Authentication
site-auth-agent = SSH agent
site-auth-key = key file
site-auth-ask = ask for a password
site-key-file = Key file
site-start-dir = Remote folder
site-group = Group
site-keepalive = Keep-alive (s)
site-compress = Compression
site-page-session = Session
site-page-environment = Environment
site-page-directories = Directories
site-page-sftp = SFTP
site-password-note = No password is stored: it is asked for when connecting.
site-env-eol = Translate end of line (not yet)
site-env-utf8 = Force UTF-8 names (not yet)
site-jump = Jump host
site-jump-note = Reached through this host: host, user@host or user@host:port. Empty means straight at the server.
site-sftp-ciphers = Choose the ciphers (not yet)
btn-connect = Connect
site-needs-host = a connection needs a host
site-needs-key = choose the key file, or another way to authenticate
site-bad-port = `{ $port }` is not a number
site-stored = connection stored: { $name } (Shift+F9 to keep it)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Struct
key-markdown = MD
key-table = Table
key-filter = Filter
key-decompile = Decomp
key-next = Next
key-prev = Prev
key-codepages = CodePg
key-goto = GoTo
key-to-panel = Panel
key-blame = Blame
viewer-structure = Structure
viewer-structure-hint = ↑↓ row · Enter go to · Tab content · F11 close
viewer-structure-hint-tab = Tab moves the keys here
viewer-no-structure = nothing to decode: this is not a format the pane knows
viewer-no-target = this row points nowhere in the file
viewer-form-opened = The form, as the text Delphi shows for it. F4 goes back to the bytes.
viewer-metadata-opened = The whole text, laid out to read. F4 goes back to the bytes.
viewer-went-to = went to { $offset }
viewer-in-panel = the panel is on { $name }
viewer-not-in-panel = { $name } is not in this panel
viewer-no-more-files = no more files in the panel
editor-unsaved-for-viewer = save first: the viewer reads the file on disk
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = running: { $command }
prompt-panelize = Command whose output names the files
prompt-viewer-filter = Command whose output to read instead of the file
learn-title = Which keys this terminal sends
learn-hint = Press the keys. A lit cell has arrived; Esc twice leaves.
learn-last = last: { $key } — { $command }
learn-unbound = bound to nothing
recode-title = Write again as
recode-what = { $count } files
recode-from = What they are now
recode-to = Write them as
recode-auto = work it out per file
recode-eol = Line endings
recode-eol-keep = leave them alone
recode-working = writing { $count } files again
recode-done = { $changed } written, { $same } already as asked, { $failed } refused
menu-recode = Write again as…
panelize-title = { $command }
panelize-done = { $count } files, { $dropped } lines skipped
panelize-nothing = { $command }: no line named a file that is there
cmd-interrupted = interrupt sent to the command
cmd-done = { $command } finished
cmd-failed = { $command } exited with { $code }
cmd-busy = a command is already running
prompt-apply = Command to run on each of { $count } files:
cmd-apply-no-ask = a command that asks for a value cannot be applied file by file
cmd-apply-progress = { $command }  ({ $done } of { $total })
cmd-apply-stopped = stopped after { $done } of { $total }: { $command } failed
cmd-not-remote = a command needs a local directory; this panel is on a server
cmd-no-history = the command history is empty
cmd-no-previous = nowhere to go back to
cmd-no-such-dir = { $path }: no such directory
cmd-no-such-path = { $path }: no such file or directory
cmd-no-selection = nothing is selected
cmd-condition-false = not run: { $command }
cmd-screen-cleared = screen cleared
cmd-stack-empty = the directory stack is empty
cmd-stack-cleared = directory stack cleared
cmd-pushed = { $count } directories on the stack
cmd-environment = environment
cmd-set = { $name } = { $value }
cmd-not-set = { $name } is not set
user-screen-title = Output — { $lines } lines, from line { $at }
user-screen-empty = (nothing has run yet)
viewer-no-branch = this row has no address to follow
viewer-no-back = nothing to go back to

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Find folder
folders-reading = Reading the folders…
folders-read = { $count } folders
folders-capped = { $count } folders, as far as the limit
folders-local-only = the folder list is of this machine; a server has its history and the hotlist
find-title = Find file
find-mask = File mask or masks
find-size = Size (100M, 1M-1G, -4K)
find-text = Containing text
find-case = Case sensitive
find-words = Whole words only
find-hex = The text above is a hex pattern
find-not = Files that do NOT contain it
find-scope = Search in
find-scope-panel = this folder
find-scope-tree = this folder and below
find-scope-selection = the selected folders
find-scope-root = from the root of this drive
find-scope-other = the other panel and below
find-links = Follow symbolic links
find-dirs = Folders in the results too
find-archives = Inside archives
find-results = Found { $count } · { $files } files and { $dirs } folders looked at
find-done = { $count } found in { $files } files · { $elapsed }
find-none = nothing found
find-stopped = stopped after { $files } files
find-hint = ↑↓ choose · Enter go to · F3 view · F4 edit · F5 panelise · Esc stop
find-hint-done = ↑↓ choose · Enter go to · F3 view · F4 edit · F5 panelise · Esc close
find-panelised = { $count } results in the panel
find-needs-mask = a mask is needed: `*` means every file
find-bad-size = `{ $text }` is not a size — { $error }

# ------------------------------------------------- replace across files
replace-title = Replace in files
replace-needle = Find what
replace-with = Replace with
replace-case = Case sensitive
replace-words = Whole words only
replace-pattern = The text above is a regular expression
replace-hex = Both are hex bytes
replace-scope = { $count } files selected
replace-needs-needle = there is nothing to look for
replace-confirm-title = Replace
replace-confirm-body = { $count } occurrences in { $files } files would change.
replace-nothing = nothing to replace: no file would change
replace-done = { $count } occurrences replaced in { $files } files

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Writing { $name } back into the archive
arc-move-out = to take something out of an archive, copy it with F5
arc-add-files-only = only files go in one at a time; Shift+F1 puts a whole directory in

rename-title = Rename several
rename-name = Name
rename-ext = Extension
rename-search = Search for
rename-replace = Replace with
rename-regex = Regular expression ($1 in the replacement)
rename-case = Letters
rename-case-same = as they are
rename-case-lower = lower case
rename-case-upper = UPPER CASE
rename-case-title = Each Word Capitalised
rename-case-sentence = First letter only
rename-counter = Counter: start, step, digits
rename-fields = [N] name  [E] extension  [N1-3] part  [C] counter  [d] [t] date, time  [P] folder
rename-not-in-archive = several files are renamed in a folder, not inside an archive: F6 renames one entry there
rename-bad-counter = the counter is three whole numbers, start, step, digits: { $counter } is not
rename-cannot = cannot rename: { $why }
rename-problem-empty = no name left
rename-problem-invalid = not a name a file can have
rename-problem-twice = another file gets this name too
rename-problem-exists = a file of this name is already there
rename-more = … and { $count } more
rename-blocked =
    { $count ->
        [one] one file cannot take its new name: nothing is renamed until it can
       *[other] { $count } files cannot take their new names: nothing is renamed until they can
    }
rename-summary = { $count } of { $total } change
rename-nothing = no name changes
rename-nothing-to-undo = there is no multi-rename to undo
rename-done =
    { $count ->
        [one] one file renamed · undo it from the Files menu
       *[other] { $count } files renamed · undo them from the Files menu
    }
rename-undone =
    { $count ->
        [one] one name put back
       *[other] { $count } names put back
    }
pack-title = Add to archive
pack-name = Archive
pack-format = Format
pack-level = Compression
pack-solid = Solid blocks (7z: smaller, slower to read one file)
pack-delete-after = Delete the files after archiving
pack-level-normal = normal
pack-level-fast = fast
pack-level-best = best
pack-scope = { $count } files selected
pack-needs-name = the archive needs a name
pack-format-read-only = { $format } is read here, not written: choose another format or another name
pack-single-one-file = a { $format } holds one file: pick one, or choose { $tar } for a folder or several files
unpack-title = Extract from archive
unpack-to = Extract to
unpack-scope = from { $name }
unpack-not-an-archive = not an archive we can read

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Quick view
panel-info = Info
panel-tree = Tree
compare-done = { $left } on the left, { $right } on the right differ
cmp-subdirs = Go into the subdirectories
cmp-depth = How deep to go (0 = all the way down)
cmp-trees-title = What differs
cmp-trees-working = comparing the trees: { $seen } files, { $read } read
cmp-trees-done = { $differ } differ, { $left } only on the left, { $right } only on the right, of { $pairs } pairs
cmp-trees-same = the two trees are the same ({ $pairs } pairs, { $same } checked)
menu-duplicates = Find duplicates
dup-title = Duplicate files
dup-working = looking for duplicates: { $seen } files, { $hashed } read
dup-done = { $groups } groups of copies, { $files } files, { $wasted } wasted
dup-none = nothing here is a copy of anything else ({ $seen } files)
dup-stopped = stopped after { $seen } files
dup-partial = stopped early
cmp-title = Compare
cmp-case = Match the names with their case
cmp-size = A different size is a difference
cmp-time = A different time is a difference
cmp-seconds = Times this many seconds apart are the same moment
cmp-contents = Read the bytes
cmp-when-time = Read them only where the times differ
cmp-fragment = Read only the first so many KiB (0 = all of it)
cmp-ignore-eol = Line endings alone are not a difference
cmp-selected = Only what is selected
cmp-stop = Stop at the first difference
cmp-newer = Select only the newer copy
menu-compare-setup = Compare w&ith options
compare-contents-eol-ignored = { $count } differ only in their line endings, ignored
compare-contents-trusted = { $count } taken as the same without reading
compare-contents-working = Comparing contents: { $done } of { $total }
compare-contents-done = { $differ } of { $pairs } pairs differ
compare-contents-same = all { $pairs } pairs are identical
compare-contents-none = no file has the same name on both sides
compare-contents-local = comparing by content needs both panels on a local filesystem
compare-contents-eol = { $count } only in the line endings
compare-contents-only = { $count } only on one side
compare-contents-unread = { $count } could not be read
compare-contents-stopped = comparison stopped
quick-view-directory = { $name } is a directory — F3 measures it
quick-view-not-here = the bytes are not here: F3 brings them out first
info-path = Path
info-kind = Kind
info-kind-local = a directory on this machine
info-kind-remote = on { $host }
info-kind-archive = inside { $name }
info-kind-list = a list of files
info-device = Device
info-folder = Folder
info-device-value = { $total }, { $free } free, { $cluster } clusters
info-slack = Slack
info-filesystem = Filesystem
info-mount = Mounted
info-git = Repository
info-git-clean = as the index has it
info-git-differing = { $count } of { $tracked } differ from the index
info-git-uncounted = not counted
info-git-merge = merging
info-git-rebase = rebasing
info-git-cherry-pick = cherry-picking
info-git-revert = reverting
info-git-bisect = bisecting
info-memory = Memory
info-memory-value = { $free } free of { $total }
info-memory-total = { $total }
info-contents = Holds
info-contents-value = { $files } files, { $dirs } directories, { $bytes }
info-selected = Selected
info-selected-value = { $count } files, { $bytes }
info-cursor = Cursor
info-size = Size
info-modified = Modified
info-description = Description
key-mark = Mark
key-asm = Asm
key-block = Block
key-screens = Screens
key-site-edit = EditSes
key-site-new = NewSes
key-site-forget = DelSes
key-ai-edit = Edit
key-ai-new = New
key-ai-remove = Remove
key-ai-forget = Forget
key-ai-threads = History
hex-needs-a-file = this buffer has no file to read bytes from
hex-save-first = save first: the two editors cannot hold each other's unsaved changes

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Block
hex-op-write = Write the { $size } to a file
hex-op-read = Read a file in at { $at }
hex-op-insert = Insert zero bytes at { $at }
hex-op-copy = Copy bytes from elsewhere to { $at }
hex-op-move = Move bytes from elsewhere to { $at }
hex-op-fill = Fill the { $size } with a byte
hex-op-xor = Exclusive-or the { $size } with a key
hex-op-delete = Delete the { $size } — { $was } becomes { $becomes }
hex-write-title = Write the block to
hex-read-title = Read which file
hex-fill-title = Fill with which byte (hex)
hex-xor-title = Exclusive-or with which key (hex)
hex-insert-title = How many zero bytes
hex-copy-title = Copy from where — offset and length (hex)
hex-move-title = Move from where — offset and length (hex)
hex-not-hex = that is not a run of hex digits
hex-not-a-count = that is not a number of bytes
hex-not-a-range = that is not an offset and a length in hex
hex-range-outside = that range is outside the file ({ $size })
hex-move-into-itself = a block cannot be moved inside itself
hex-filled = { $size } filled
hex-xored = { $size } exclusive-ored — this is not encryption
hex-written = { $size } written to { $name }
hex-read-done = { $size } read from { $name }
hex-inserted = { $size } of zeros inserted
hex-deleted = { $size } deleted
hex-device = { $name }: a device of { $size }, open read-only
asm-title = Assemble at { $at } ({ $arch })
asm-no-arch = { $arch } is not an architecture we can assemble
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } bytes longer)
screens-title = Screens
screens-panels = Panels
quit-title = Leave FinalCommander
quit-body = Quit?
screens-quit-title = Unsaved changes
screens-quit-body = These screens hold changes nobody has written:
macro-recording = Recording a macro · Ctrl+. stops it
macro-empty = nothing was recorded
macro-dropped = the recording was thrown away
macro-key-title = Which key
macro-key-body = { $steps } steps recorded. Press the key this macro will answer to, or Esc to throw it away.
macro-describe = What is the macro on { $key } for
macro-saved = macro on { $key } saved, { $steps } steps
macro-none = no macros recorded yet · Ctrl+. records one
macro-list-title = Macros
macro-play-title = How should it play
macro-play-once = Once, from where the cursor is
macro-play-each = Once for every selected file
macro-sure-title = Play this macro
macro-sure-body = The macro on { $key } copies, moves, deletes or overwrites. Play it?
macro-too-deep = a macro cannot play itself for ever
plugin-list-title = Plugin commands
plugin-none-title = No plugins installed
plugin-none-open = Open the plugins folder ({ $dir })
plugin-none-help = How plugins work, and where to find some
plugin-help-heading = Plugins
plugin-why-off = turned off in Options → Plugins configuration
plugin-why-panel = only in the panels
plugin-why-viewer = only in the viewer
plugin-why-editor = only in the editor
plugin-why-local = needs a folder on this machine's disks
plugin-why-files = needs a file under the cursor or selected
plugin-why-masks = only for { $masks }
plugin-why-bad-mask = its mask is wrong: { $error }
plugin-why-here = needs { $names } in this folder
plugin-why-inside = needs { $names } here or in a folder above
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = on { $count } selected file(s)
plugin-failed = { $id }: { $why }
plugin-unknown-command = a plugin asked for `{ $id }`, which is not a command
hex-copied = { $size } copied from { $from }
hex-moved = { $size } moved from { $from }
menu-calculator = Calculator
menu-terminal = Terminal
calc-title = Calculator
calc-decimal = decimal
calc-hex = hex
calc-octal = octal
calc-binary = binary
calc-characters = characters

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Archive commands
arc-password = Password for { $name }
arc-op-test = Test { $name }
arc-op-extract-here = Extract { $name } here
arc-op-extract-to = Extract { $name } to…
arc-op-delete = Delete { $name }
arc-op-add = Add files to an archive…
job-test = Testing archive
msg-dirs-first-on = folders first
msg-dirs-first-off = folders in with the files
msg-selected-first-on = selected files first
msg-selected-first-off = selected files in order

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Disk
drive-removable = Removable
drive-optical = Optical
drive-network = Network
drive-memory = Memory
drive-home = Home
drive-connection = Connection
drive-objects = Object store
drive-netbox = NetBox: stored sessions
drive-processes = Process list
drive-temp = Temporary panel
temp-added = { $count } added to the temporary panel
proc-none = the list of running processes is not readable here
drive-programs = Installed programs
drive-services = Services
svc-none = this machine has no service manager we can ask
temp-not-a-file = the rows here are not files: there is nothing to delete
svc-card = Service
svc-card-id = Name
svc-card-name = Shown as
svc-card-state = State
svc-card-description = Description
svc-card-pid = Process
svc-state-running = running
svc-state-stopped = stopped
svc-state-failed = failed
svc-state-other = other
key-service-start = Start
key-service-restart = RstSrv
key-service-stop = StopSrv
svc-ask-stop = Stop { $name }?
svc-ask-restart = Restart { $name }?
btn-service-stop = Stop
btn-service-restart = Restart
svc-working = { $name }: asking the system…
svc-started = { $name } started
svc-stopped = { $name } stopped
svc-restarted = { $name } restarted
svc-needs-root = { $name }: this needs root — the program will not ask for it on your behalf
svc-failed = { $name }: { $error }
prog-none = nothing here says what is installed
prog-card = Installed program
prog-no-uninstall = nothing says how to remove { $name }
prog-uninstall-ready = the command that removes { $name } is on the command line, unrun
prog-card-name = Name
prog-card-version = Version
prog-card-publisher = Publisher
prog-card-size = Size on disk
prog-card-installed = Installed
prog-card-location = Folder
prog-card-uninstall = Removed by
drive-sockets = Socket list
sock-none = nothing is listening here, or the table is not readable
sock-no-process = nothing will say which process holds this socket
proc-card = Process
proc-gone = { $name } is no longer running
proc-card-name = Name
proc-card-pid = Process id
proc-card-parent = Started by
proc-card-user = User
proc-card-state = State
proc-card-threads = Threads
proc-card-memory = Memory
proc-card-mapped = Mapped
proc-card-bits = Bitness
proc-card-started = Started
proc-card-exe = Executable
proc-card-cwd = Directory
proc-card-command = Command line
proc-card-group = Group
proc-card-sid = SID
proc-card-priority = Priority
proc-card-session = Session
proc-card-uptime = Running for
proc-card-cpu-user = Processor time, its own
proc-card-cpu-kernel = Processor time, in the kernel
proc-card-handles = Open handles
proc-card-faults = Page faults
proc-card-private = Private memory
proc-card-swap = Swapped out
proc-card-io-read = Read
proc-card-io-write = Written
proc-card-memory-peak = Working set peak
proc-card-private-resident = Working set, private
proc-card-swap-peak = Page file peak
proc-card-virtual = Virtual bytes
proc-card-virtual-peak = Virtual bytes peak
proc-card-paged-pool = Paged pool
proc-card-nonpaged-pool = Non-paged pool
proc-card-io-other = Other I/O
proc-card-io-read-ops = Read operations
proc-card-io-write-ops = Write operations
proc-card-io-other-ops = Other operations
proc-kill-title = End process
proc-kill-ask = End { $name } [{ $pid }]? Whatever it has not saved is lost.
proc-killing = Ending { $name }…
proc-killed = { $name } ended
proc-kill-failed = { $name } could not be ended: { $why }
proc-card-env = Environment
proc-card-modules = Loaded modules
proc-card-version = Version
proc-card-description = Description
drives-hint = ↑↓ choose · 0-9 panels · Ctrl+1..9 columns · F9 all of them · Ctrl+R re-read
drives-title = Change drive: what to show
drives-col-type = the type
drives-col-device = the device
drives-col-label = the label
drives-col-fs = the filesystem
drives-col-size = the sizes
drives-col-removable = the removable drives
drives-col-panels = the panels that are not drives
drives-col-optical = the optical drives
drives-col-network = the network shares
drives-shown = showing { $what }
drives-hidden = hiding { $what }
drives-reread = the drives were read again
bookmarks-hint = Enter go · Ins keep · Del remove · F4 name · Esc close

# The search dialog (F7), which asks the whole question in one place.
search-title = Search
search-for = Search for
search-mode = As
search-mode-text = text
search-mode-hex = hex
search-mode-regex = regular expression
search-case = Case sensitive
search-words = Whole words only
btn-find-next = Find next
btn-find-prev = Find previous
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = What this is
help-chapter-panels = The panels
help-chapter-files = Files: copy, move, find, archives
help-chapter-viewer = The viewer
help-chapter-editor = The editor, the hex pane and the assembler
help-chapter-network = Servers
help-chapter-config = Configuration, keys and colours
help-chapter-keys = Keys, as they are bound now
help-chapter-commands = Every command, and its keys
help-chapter-options = Every setting, and its value
help-here-panels = The panels
help-here-viewer = The viewer
help-here-editor = The editor
help-here-hex = The hex editor
help-here-dialog = This dialog
help-in-english = This chapter has not been translated into your language yet, so it follows in English. The keys and the settings it names are the same in every language.
help-about-byline = FinalCommander { $version } — by Matteo Baccan
key-ok = OK
key-cancel = Cancel
key-dialog-move = Move
key-tree = Tree
key-config = Config
browse-local-only = a file is chosen from this machine; a path on a server is typed, and Tab completes it
key-browse = Browse
table-prompt-filter = Rows holding
table-sorted = sorted by { $column }
table-sorted-back = sorted by { $column }, the other way round
table-unsorted = back to the order the file has them in
table-filtered = { $rows } rows left
table-unfiltered = every row again
table-cut-sorted = of the first { $rows } rows, which is all that was read
table-too-many = too many records to sort or filter here
table-copied =
    { $rows ->
        [one] one row on the clipboard
       *[other] { $rows } rows on the clipboard
    }
table-headed = the first line is the heading
table-not-headed = the first line is data: the columns are called by their number
table-named-fields = this format names its own fields: there is no heading to guess

# Splitting and combining (SPEC 03 §9)
split-title = Split file
split-target = Into
split-size = Each piece
split-size-hint = 1440K, 700M, 4G — the last piece is what is left
split-count = Or into
split-count-hint = pieces — a number here wins over the size
split-bad-count = { $count } is not a number of pieces: two or more
split-local-file = Splitting and combining work on a file on this machine
split-bad-size = “{ $size }” is not a size: write it as 700M or 4G
combine-not-a-piece = That is not a piece: a split file's pieces end in .001, .002, … or come with a .crc
pieces-working = Splitting or joining: { $done } of { $total }
pieces-cancelled = Stopped, and what was written is taken back
split-done = { $parts } pieces written, and { $list } beside them
combine-done-checked = { $name } joined from { $parts } pieces, { $bytes }, and it matches its .crc
combine-done-unchecked = { $name } joined from { $parts } pieces, { $bytes } — there was no .crc to check it against
pieces-failed = Not done: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Reading every file under { $path }…
branch-title = Branch: { $path }
branch-done = { $count } files, from every folder below this one · Ctrl+Alt+B goes back
branch-truncated = { $count } files, and the walk stopped there: the branch is bigger than a list can usefully be
branch-not-here = The branch view is of a folder, and this panel is a list

## The release check (SPEC 11 §5).
menu-check-update = Chec&k for updates
opt-update-check = Check for a new release once a day
btn-open-page = Open the page
update-checking = Asking GitHub for the latest release…
update-title = Updates
update-available = FinalCommander { $version } is out (this is { $current }).
update-available-status = FinalCommander { $version } is out: Options → Check for updates
update-current = This is the latest release, { $version }.
update-failed = Could not ask for the latest release: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Find duplicate files
dupes-title = Duplicate files
dupes-recursive = Look in subfolders
dupes-min-size = Smallest file
dupes-min-size-hint = A size as the filters write it: 1, 100K, 5M. Empty files are never compared.
dupes-select = Select every copy but the oldest
btn-search = Search
dupes-not-here = The duplicates search starts from a folder, or from what is selected.
dupes-bad-size = Not a size: { $size }
dupes-looking = Looking for duplicate files…
dupes-progress = Duplicates: pass { $pass } of 3, { $done }/{ $total }
dupes-none = No duplicates among { $count } files.
dupes-panel = Duplicates: { $groups } groups
dupes-done = { $groups } groups of duplicates, { $size } in the copies.
dupes-truncated = Stopped early: { $groups } groups so far, { $size } in the copies.

## SQLite queries (SPEC 04 §4b).
sql-empty = Write a query after sql:, such as sql: SELECT * FROM t
sql-no-database = sql: needs an SQLite database: go inside one, or put the cursor on one.
sql-running = Running the query…
sql-failed = The query failed: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = No AI service is set up: add an [[ai]] block to fcmd.toml (name, url, model, api, token_env).
ai-title = Ask an AI
ai-no-thread = no answer to follow up: ask with `ai:` first, or put the cursor on one in the answers' folder
ai-nothing-to-explain = nothing to explain here: open a file in the viewer or the editor
ai-explain-question = What is this, from { $name }? Explain it briefly.
ai-tokens = { $in } tokens in, { $out } out
ai-none-panel = No AI service yet — F7 adds one
ai-service-removed = { $name } removed
ai-probing = Logging in to { $name } at { $host }…
ai-probe-ok = { $host } answered HTTP { $status } in { $ms } ms, { $models } models — type a question and press Enter
ai-probe-no-model = (the model { $model } is not among them)
ai-row-context = context
ai-row-conversation = conversation
ai-session-title = AI: { $name } · { $model } — { $files } files in context
ai-context-added = { $count ->
        [one] One file
       *[other] { $count } files
    } in the context
ai-context-nothing = Nothing to add: only files on this disk go into the context
ai-command-ready = the command is on the line — read it before pressing Enter
ai-commit-question = Write a commit message for the staged changes.
ai-staged-diff = the staged diff
ai-send-diff = The staged diff goes to { $host }
ai-send-files = { $count } files, { $size }, will be sent to { $host } with the question.
ai-asking = Asking { $name }…
ai-busy = { $name } is thinking… { $secs }s · Esc stops
ai-conversation-new = A new conversation: the next question starts it.
ai-live-empty = Type a question on the command line: it goes to { $name }, and the conversation shows here as it goes.
ai-failed = The AI did not answer: { $why }
ai-stopped = stopped: the question to { $name } was dropped, and nothing was written
ai-context-title = Files in the context
ai-context-all = All of them
ai-context-dropped = { $count ->
        [one] one file out of the context
       *[other] { $count } files out of the context
    }
ai-context-hint = Enter or Del: take it out of the context — the file stays on the disk · F3: what is sent · Esc
ai-threads-title = Conversations
ai-threads-hint = Enter: go on with this conversation · Esc
ai-thread-resumed = going on with { $name }
ai-next-estimate = next ≈ { $tokens }
ai-next-over = next ≈ { $tokens }, more than the model holds
ai-send-estimate = About { $tokens } tokens in all, with the conversation so far.
ai-send-estimate-of = About { $tokens } tokens in all, with the conversation so far, of the { $max } the model holds.
ai-send-over = About { $tokens } tokens in all: more than the { $max } the model holds, so the start may be cut off. F8 takes a file out.
ai-send-question = The question goes to { $host }.
btn-send = Send

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = What changed
git-log = The log
git-blame = Blame { $name }
git-branches = Branches
git-local-only = git works on a folder on this disk.
git-running = Asking git…
git-failed = git: { $why }
git-clean = Nothing has changed.
git-status-title = Changed: { $staged } staged, { $unstaged } not
git-side-conflict = conflict
git-side-staged = staged
git-side-changed = not staged
git-side-untracked = untracked
git-kind-modified = modified
git-kind-added = new
git-kind-deleted = deleted
git-kind-renamed = renamed
git-kind-copied = copied
git-kind-type = type changed
git-kind-conflict = to resolve
git-as-tree = Show what changed as a tree
git-as-list = Show what changed as a list
git-version-head = last commit
git-version-index = staged
git-version-disk = on disk
git-discard-title = Discard changes
git-discard-body = Throw away the changes of { $count ->
        [one] one file
       *[other] { $count } files
    }?
git-discard-worktree = back to the index:
git-discard-both = back to the last commit:
git-discard-added = out of the index, kept on disk:
git-discard-trash = to the wastebasket:
git-discard-warning = What goes back to the index or the last commit cannot be undone.
git-hint = Enter: choose · Esc: close

## The editor's character list and word completion.
charmap-title = Characters
charmap-hint = Type to narrow (a character or its code, U+00E9) · Enter: insert · Esc: back
editor-no-completion = No other word in the file starts that way.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Copied { $url }

## The system log (SPEC 06 §16).
menu-syslog = System log
syslog-reading = Reading the system log…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Take out the lines matching (! first: keep only those)
editor-lines-deleted = Lines taken out: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = overwrite when the size differs

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = If one cannot be copied
copy-error-ask = Ask
copy-error-skip = Skip it and go on
copy-error-stop = Stop the copy

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Times copied: Ctrl+Shift+V puts them into another file's dialog
attr-times-none = Nothing copied yet: Ctrl+Shift+C in a file's dialog first
attr-times-pasted = Times pasted: OK sets them

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Replacements
replace-tpl-hint = Enter runs it · Ins keeps a recent one by name · Del forgets a kept one · Esc
replace-tpl-name = Keep this replacement as
replace-tpl-kept = Kept as {$name}
replace-tpl-empty = No replacement yet: Ctrl+F7 first

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Each folder keeps its own view and sort

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Speed limit, KB/s (empty: none)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (mail, read only)
site-protocol-imaps = IMAPS (mail over TLS, read only)

# The AI services as a list, like the sessions
ai-services-title = AI services
ai-services-hint = Enter: the one ai: asks · Ins new · F4 change · Del remove · Esc
ai-service-title = AI service
ai-field-name = Name
ai-field-api = Protocol
ai-field-url = Address
ai-field-model = Model
ai-field-token-env = Key in variable
ai-field-token-note = Empty: the key is asked for once per session and never written down.
ai-field-system = What it is told first
ai-service-incomplete = A service needs a name and an address
ai-service-default = ai: now asks {$name}
ai-key-prompt = The key for {$name} (kept until you quit)
ai-api-openai = OpenAI and compatible
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (on this machine)
menu-ai = AI services

# The secrets file
vault-password = The password of fcmd's secrets file
vault-new = Keep this key in fcmd's encrypted secrets file: choose its password (empty: only for this session)
vault-refused = The secrets file did not open: {$why}

# The programs from elsewhere
drive-tools = External programs
drive-ai = AI
drive-git = Git
tools-found = found
tools-missing = not installed
tools-summary = { $found } of { $total } external programs found; F3 says what each is for, F4 installs a missing one
tools-card = External program
tools-card-program = Program
tools-card-where = Where
tools-card-version = Version
tools-card-for = Used for
tools-card-install = To install
tools-card-why = Why fcmd runs it rather than doing it itself: F1, "Programs from elsewhere".
tools-install-any = install the package that provides { $name } and make sure it is on the PATH
tools-already = { $name } is already here
tools-for-archives = archives: { $what }
tools-for-decompile = Shift+F6 in the viewer on { $what }
tools-for-git = the Git panel (Ctrl+Alt+G) and the branch in the prompt
tools-for-video = the frames of a video in the viewer and the quick view (F3, Ctrl+Q)
tools-for-open = opening a link in the browser
tools-for-syslog = Commands → System log
tools-for-services = the services panel
tools-card-site = Website
tools-no-package = No package manager on this machine is known to have { $name }. Open its download page?
tools-install-ask = Install { $name } by running, in a terminal:
tools-install-how = The terminal is where sudo asks for your password. Ctrl+Alt+E goes back to the panels, and Ctrl+R in this panel looks again.
tools-install-else = If it fails: { $url }
tools-install-failed = fcmd: { $name } was not installed. Download it from { $url }
tools-installing = installing { $name } in a terminal: Ctrl+Alt+E goes back to the panels, Ctrl+R there looks again
btn-install = Install

# Plugins configuration
plugins-config-title = Plugins
plugins-config-hint = Enter: turn on or off · F3 what it is and what it brings · Esc
plugins-broken = did not load — F3 says why
plugins-cannot-toggle = a plugin that did not load cannot be turned on: F3 says what is wrong with it
plugins-on = { $id } is on: its commands are in F11 now
plugins-on-restart = { $id } is on: its commands are in F11 now, the rest it brings from the next start
plugins-off = { $id } is off: its commands have left F11
plugins-off-restart = { $id } is off: its commands have left F11, the rest it brings goes at the next start
plugins-state-on = on
plugins-state-off = off
plugins-card-id = Id
plugins-card-name = Name
plugins-card-version = Version
plugins-card-author = Author
plugins-card-about = About
plugins-card-dir = Folder
plugins-card-state = State
plugins-card-brings = Brings
plugins-card-commands = Commands (F11):
plugins-card-notes = Said while loading:
plugins-none = no plugins found · they live in { $dir }, one folder each with a plugin.toml

# The git panel
git-file-log = The log of { $name }
git-commit = Commit what is staged…
git-pull = Pull
git-push = Push
git-pushing = Pushing…
git-pulling = Pulling…
git-commit-prompt = The commit message
git-commit-empty = A commit wants a message.
git-already-on = { $branch } is the branch you are on.
git-switch-ask = Switch to { $branch }? The files the two branches disagree on change on disk.
git-done = git: { $said }
git-branches-title = Branches: { $count }
git-log-title = Log: { $count }
git-log-of-title = Log of { $of }: { $count }
btn-switch = Switch
key-git = Git
key-git-diff = Diff
key-git-stage = Stage
key-git-unstage = Unstage
key-git-discard = Discard
key-git-commit = Commit
key-git-show = Show
key-git-log = Log
key-git-changes = Changes
key-git-pull = Pull
key-git-push = Push
key-git-branches = Branch

undo-title = Undo
btn-undo = Undo
menu-undo = Undo last operation
menu-strip-meta = Strip picture metadata…
undo-nothing = nothing to undo: no copy, move, delete to the wastebasket or new folder since the program started
undo-copy =
    { $count ->
        [one] Send the file the copy made to the wastebasket:
       *[other] Send the { $count } files the copy made to the wastebasket:
    }
undo-move =
    { $count ->
        [one] Move the file back where it was:
       *[other] Move the { $count } files back where they were:
    }
undo-trash =
    { $count ->
        [one] Put the file back from the wastebasket:
       *[other] Put the { $count } files back from the wastebasket:
    }
undo-mkdir = Remove the new folder, if it is still empty:
strip-title = { $count ->
        [one] Metadata of one picture
       *[other] Metadata of { $count } pictures
    }
strip-all = All of it — text, EXIF, XMP, IPTC, comments, C2PA
strip-ai = Only what an image model wrote — prompt, workflow, settings
strip-camera = All but the camera’s EXIF and the copyright — no GPS
strip-hint = Enter: take it out; the picture is not re-encoded, Ctrl+Alt+Z puts it back · Esc
strip-nothing = no PNG, WebP or JPEG on this disk selected or under the cursor
strip-done = { $count ->
        [one] one picture
       *[other] { $count } pictures
    } without their metadata ({ $blocks } blocks out), { $clean } had none to take out · Ctrl+Alt+Z puts them back
undo-restore = { $count ->
        [one] Put the picture back with its metadata:
       *[other] Put the { $count } pictures back with their metadata:
    }
undo-done =
    { $count ->
        [one] one name back where it was
       *[other] { $count } names back where they were
    }
slow-drives = the drive list took { $secs } s
slow-drives-late = the drive list waited { $secs } s: { $drives } did not answer in time and are shown without details — a network drive whose server is not there?
slow-listing = reading { $what } took { $secs } s
# editor search and undo (0.41)
btn-find-all = All matches
btn-replace-all = Replace all
btn-replace-one = Replace next
editor-replace-title = Replace
editor-replace-style = Preserve style (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = In a regular expression $1 and { "${name}" } are what a group caught, $$ a dollar
editor-matches-title = Lines with `{ $query }`: { $count }
editor-matches-hint = Type to narrow · Enter: go to the line · Esc: back


# sorting and extended attributes (0.41)
opt-sort-case = Upper and lower case in the sort
opt-sort-case-auto = as this system does
opt-sort-case-yes = tell them apart
opt-sort-case-no = one alphabet
copy-keep-xattrs = Keep the extended attributes

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Write the file as
editor-convert-hint = Type to narrow · Enter: convert (F2 writes it, Ctrl+Z takes it back) · Esc: back
editor-convert-eol = End of line: { $eol }
editor-convert-bom-on = Byte order mark (BOM): written
editor-convert-bom-off = Byte order mark (BOM): not written
editor-convert-page = Code page: { $page }
editor-converted-eol = { $name }: lines will end with { $what } at the next save
editor-converted-bom-on = { $name }: the next save writes a byte order mark
editor-converted-bom-off = { $name }: the next save writes no byte order mark
editor-converted-page = { $name }: the next save writes { $what }
editor-convert-same = The file is already written that way
editor-convert-no-bom = { $page } has no byte order mark: only UTF-8 and UTF-16 do
editor-convert-lossy = { $page } cannot hold every character of { $name }: nothing was changed
btn-reload = Reload
btn-what-changed = What changed
editor-diff-buffer = in the editor
editor-diff-hint = This comparison only shows what changed — Esc goes back to the question


# job journal and saved syncs (0.41)
journal-left = Copies left unfinished by the last session: { $count } · Alt+Shift+F6 lists them
journal-title = Unfinished jobs
journal-hint = Enter go on with it · Del discard · Esc close
journal-none = No job was left unfinished
journal-row = { $kind }: { $what } → { $target } · files done: { $done }
journal-row-unreadable = cannot be read: { $file } ({ $error })
journal-unreadable = { $file }: not a job journal this version can read ({ $error }). Alt+Shift+F6 lists it, and Del there removes it
journal-resumed = Going on with { $kind }: { $done } files were done already
journal-discarded = The unfinished job was discarded, with what it had half written
journal-cannot-resume = That journal cannot be read; Del removes it
menu-jobs-left = Unfinished jobs
sync-report = Write a report of the run to (a file)
sync-name = Save this pair as (a name)
sync-saved-button = Saved…
sync-saved-title = Saved synchronisations
sync-saved-hint = Enter run · F4 change · Del forget · Esc close
sync-saved-none = No synchronisation is saved yet: give one a name in the dialog (Ctrl+Shift+Y)
sync-saved-removed = { $name } is no longer saved
sync-report-local = The report has to be a file on this machine
sync-report-written = The synchronisation's report is in { $path }
sync-report-not-written = The report could not be written to { $path }: { $error }
sync-report-title = Synchronisation report
sync-report-name = Name: { $name }
sync-report-left = Left: { $path }
sync-report-right = Right: { $path }
sync-report-to-right = Copied left → right: { $count }
sync-report-to-left = Copied right → left: { $count }
sync-report-cut-right = Taken out on the right: { $count }
sync-report-cut-left = Taken out on the left: { $count }
sync-report-skipped = Left alone though they differ: { $count }
sync-report-failed = Failed: { $count }
sync-report-stopped = Stopped part of the way, so these may or may not have been done: { $count }
sync-report-not-done = Not done, because an earlier step failed or was stopped: { $count }
menu-sync-saved = Saved synchronisations

# hex editor search
hex-search-wrapped = `{ $query }`: found by starting again from the other end of the file
hex-replace-title = Replace
hex-replace-with = Replace with
btn-replace = Replace
hex-replace-ask = Replace { $was } at { $at } with { $with }?
hex-replace-length = the replacement is { $with } bytes long and the match { $was }: that would move everything after it — press Ins for insert mode first

# pixels and video (0.41)
opt-viewer-pixels = Pictures
opt-viewer-pixels-auto = real pixels where the terminal draws them
opt-viewer-pixels-kitty = kitty graphics protocol
opt-viewer-pixels-iterm = iTerm2 inline images
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = half blocks always
video-needs-program = this is a video ({ $what }): its frames are drawn through `{ $program }`, which is not installed on this machine — F11 still reads the container
video-no-frame = no frame from this video: { $why }
video-playing = playing at { $fps } frames a second, without sound — Space pauses
video-paused = paused at { $at } — Space plays on, PgUp and PgDn step by a tenth
video-ended = the video has ended — Space plays it again


# finding things
menu-saved-searches = Saved searches
drive-searches = Saved searches
find-keys = F2 keeps this search under a name · F9 the kept ones
find-save-name = Keep this search as
find-saved = search kept as { $name }
find-saved-none = no search has been kept yet: F2 in the find dialog keeps one
find-saved-title = Saved searches — Enter opens, F4 edits, Del forgets
find-saved-running = searching for { $name }…
find-saved-gone = search { $name } forgotten
find-loaded = search { $name } loaded
find-fuzzy = Approximate name: the letters above, in order, best match first
find-meta = Metadata (artist=bowie; camera=)
find-index = Ask the system's index instead of walking (may miss new files)
tools-for-index = the index Alt+F7 may ask instead of walking
menu-frecent = Frequent folders
frecent-none = no folder has been visited yet
frecent-gone = { $path } taken out of the frequent folders
frecent-hint = type letters of the path · Enter go there · Del forget · Esc close

# ai third round
ai-redact-off = Redaction is off: everything goes as it is
ai-redact-none = No secret found to mask
ai-redact-count = Secrets masked before sending: { $count } ({ $kinds })
ai-redacted = secrets masked: { $count }
ai-preview-title = What goes to { $name } ({ $host })
ai-preview-instruction = Instruction
ai-preview-earlier = The conversation so far
ai-preview-question = The question and its files
ai-preview-key = F3 shows what is sent
ai-preview-reading = Reading what would be sent…
key-ai-redact = Redact
ai-redact-now-on = Redaction is on: keys, tokens, private keys and passwords are masked before anything is sent
ai-redact-now-off = Redaction is off: what is sent goes as it is
ai-panel-redact-on = secrets masked
ai-panel-redact-off = secrets not masked
