# The two panels

Each panel shows one directory. `Tab` moves between them; most operations go from the panel
with the cursor to the other one, which is what makes a two-panel manager faster than a tree:
the destination is already on screen.

`Enter` enters a directory or runs a file, `Backspace` or `..` climbs. `Alt+F1` and `Alt+F2`
open the change-drive dialog for the left and right panel, over the panel it belongs to: a
table of everything mounted — what each one is (disk, removable, optical, network, memory),
what it is called, what it is formatted as, how big it is and how much is left. Which of
those columns are drawn is yours to choose, with `F9` inside the dialog.

Under a rule, on `Ctrl+1` to `Ctrl+9`, are the things that are not filesystems:

* the **stored connections** and the **object stores** written in the configuration, which
  connect when chosen;
* the **processes** running on the machine, as a panel: `F3` on one is its card, which is a
  page — what it is, whose it is, what it has taken and spent, its command line, its whole
  environment and every library it has loaded. Nothing here ends a process: the key that
  deletes a file says so rather than doing it;
* the **services** the machine runs in the background, with the state of each: `F3` is its
  card, `F5` starts it, `F6` restarts it and `F8` stops it — the last two ask first. An
  action the system refuses is reported as needing root; this program will not ask for root
  on your behalf;
* the **sockets**, listening or connected, with the address at each end and the process
  holding it, where `F3` is that process's card;
* the **programs installed** on the machine, with their version and publisher: `F3` is the
  card, and choosing one puts its own uninstall command on the command line, unrun, for you
  to read before pressing `Enter`;
* a **temporary panel**, which is a list rather than a directory: `Ctrl+Shift+Ins` adds what
  is selected to it from anywhere, and the files in it can come from a dozen directories at
  once.

`Ctrl+R` inside the dialog reads the drives again, for a stick that was plugged in after it
opened. What the system runs on — anything under `/proc`, `/sys`, `/dev`, `/run` — is left
out of the table, because none of it is somewhere to put a file.

# Looking at a directory

`Ctrl+1` to `Ctrl+5` choose how rows are drawn: brief, medium, full, wide, detailed. The
multi-column views run down a column and then across, as the originals do. The detailed view
adds, to what the full one shows, the date the file was made and the date it was last read,
and the attributes after them — on a panel too narrow for all of that the extra dates go
first, so the name stays readable. The full view keeps
the extension in a column of its own, against the size: names down the left, extensions down
the right. A folder with a dot in its name, a dotfile and an extension too long for the column
keep their name whole.

`Ctrl+8` shows who the files belong to: name, size, owner, group. The two names take the room
the date had, and on a narrow panel the group goes before the owner does. Finding out costs a
lookup per file — and on Windows a call that may reach a domain controller — so it is done in
the background and only while this view is the one on screen: the columns fill in a moment
after the listing, and a row nothing has come back for yet is blank. A panel on a server or
inside an archive has no owner to show.

`Ctrl+9` puts how many names each file has beside it, which is how a hundred copies that are
really one file stop looking like a hundred files' worth of space, and `Ctrl+0` is the full
view with the sizes written out digit by digit — `1 234 567` where the full view says
`1.18 M` — for when the difference between 1.2M and 1.9M is the thing you came to see.

Each of the ten is a list of columns you can rewrite: a `[[column_set]]` block in
`fcmd.toml` whose `id` is the view's own digit is what that view draws, with the columns,
widths, alignments and formats you choose — FAR's own `Column types` strings included. The
configuration chapter has the shape of it.

`Ctrl+F3` to `Ctrl+F11` sort: by name, extension, write time, size, unsorted, creation time,
access time, description and owner — all nine of FAR's keys. Choosing the order already in
force reverses it, and `Ctrl+F12` offers them as a list. The last two sort by something the
listing does not carry, so they are fetched like the views that show them: the panel sorts
without them and sorts again when the answer lands, keeping the cursor on the file it was on.
A file nothing has said anything about sorts after every file that has.

Two files the sort key cannot separate are settled by `panel.sort_chain` in the
configuration — `["size", "name"]` sorts by size and settles a tie by name — and the name has
the last word in any case, so the order never depends on the order a directory came back in.

`Shift+F11` — `Group` on the key bar — turns the **sort groups** on and off. They are bands
written in the configuration as `[[sort_group]]` blocks, each with a mask and a side, and they
pull some kinds of file in front of the listing and push others behind it whatever the sort
key is: sources first and build output last, in a directory that holds both. Inside a band the
ordinary sort still applies, folders keep their own place at the top, and `reverse` turns each
band round rather than swapping the bands over. With no blocks written the key says so.

You do not have to write them by hand: `F9 → Options → Sort groups` is the same list the
colour groups have — the bands in the order they cut the listing, `Ins` for a new one beside
the row you are on, `Del` to take one away, `F4` to edit it, `Ctrl+↑` and `Ctrl+↓` to move one
past its neighbour. Both panels re-sort the moment you accept, and *Save setup* writes the
blocks into the configuration.

`Space` measures what a folder holds — and with something selected, every selected folder at
once, which is how you find out which of five trees is the big one. The figure then takes the
place of the word `Folder` in the size column, the totals bar at the bottom counts it, and
sorting by size puts that folder where it belongs. Re-reading the directory forgets it, since
the figure was about the directory as it was.

`Alt+F10` finds a folder by name: a list of every folder this program knows under the root you
are on, which narrows as you type, and `Enter` goes there. It starts from what the tree has
read — kept between sessions — and reads the rest behind the dialog while you type. In the
copy dialog the same `Alt+F10` fills the destination in.

`Ctrl+H` shows the hidden files. `Ctrl+P` hides the panel you are not in, `Ctrl+F1` and
`Ctrl+F2` hide the left or the right one to give the other the whole screen, and `Ctrl+O`
takes both off to show what the commands have printed. With the panels off everything else
still works — the command line, the menus, the keys — and `PgUp` and `PgDn` turn pages through
the output; `Ctrl+O` brings the panels back.

The panels go off by themselves while a command runs, so what it prints can be watched
arriving line by line rather than turning up all at once at the end, and they come back when
it finishes. `Ctrl+O` gives the screen back early and the command carries on behind them;
*Options → Interface → Show a command's output while it runs* turns the whole habit off, and
the output still fills the screen as it is printed, for whenever you press `Ctrl+O`.

# What a file is

A directory can keep a line about each of its files, in a plain file called `descript.ion`
beside them — FAR's habit, and 4DOS's before that. `Ctrl+Z` writes one: the dialog opens on
what the directory already says about the file under the cursor, and an empty answer takes the
line away. `Ctrl+6` shows the names with their descriptions, `Ctrl+7` adds the size and the
time, the information panel (`Ctrl+L`) spells out the one under the cursor, and the panel's
own status line shows it beside the size and the date.

The file travels with the directory, which is the point of it: a folder copied to a memory
stick takes its descriptions along. And a description follows its file: `F5` writes one at the
other end as well, `F6` takes it with the file, a rename takes it to the new name, and `F8`
takes it away — a description that outlived its file would land on the next file to be given
that name, even when the files came from several directories at once, as they do out of a
search result. All of it works on a server and in a bucket as well; inside an archive the line
is read but not written, because writing one there means building the archive again.
*Options → Panel settings* turns the whole habit off for somebody who keeps no descriptions.

`Ctrl+R` re-reads. `Ctrl+L` turns the other panel into an information panel — what the device
holds and what is left on it, how big its clusters are and what they cost this directory, how
much memory the machine has, and the row under the cursor — `Ctrl+Q` into a
preview of the file under the cursor — or, on a folder, into the same block of device and
directory figures the information panel shows — `Ctrl+T` into a directory tree.

The preview is the viewer itself, with its frame taken off. While the files have the focus it
only follows the cursor; give it the focus with `Tab` and its keys are the reader's: `F2`
wraps, `F4` walks text, dump, hex, document and table, `Shift+F4` is hex and back, `F5`
disassembles, `F6` decompiles, `F7` searches, `F8` changes the code page, `F11` opens the
structure pane. The key bar says so while it has the focus. What you type still goes to the
command line. And beside a process, socket or programs list the preview holds that row's
card — the same page `F3` opens there, which is longer than the panel and scrolls.

A local panel also re-reads itself: a file that arrives from a build, a download or another
program appears without being asked for, and the cursor stays on the name it was on. The
setting is `Refresh the panel automatically` in `Options → Panel settings`
(`panel.auto_refresh`); `Ctrl+R` is still there for a server, an archive, and the times a
watch has nothing to report.

`Ctrl+←` and `Ctrl+→` move the line between the two panels. `Ctrl+↑` brings the panels' bottom
edge up — they get shorter, and the rows they give up show the output of the last command,
which is how you watch a build go by without leaving the panels; `Ctrl+↓` gives the rows back.
Both are settings, so `Shift+F9` keeps them.

`Ctrl++` and `Ctrl+−` ask the terminal for a larger or smaller font, and `Ctrl+Alt+0` for the
one it started with — `Ctrl+0` itself is the tenth view mode, as it is in FAR. Whether
anything happens is the terminal's business and not ours: a program in a terminal does not
own its font, it can only ask. xterm and the terminals that copy it
answer; on most others (GNOME Terminal, Konsole, Windows Terminal, alacritty, foot) those keys
are the emulator's own zoom and never reach us at all — which gets you the same thing by a
shorter road.

# Folders kept to hand

`Ctrl+Shift+0` to `Ctrl+Shift+9` put the directory you are in on a digit; `Alt+0` to `Alt+9`
take you back to it. Ten of them, as FAR has — and `Alt` rather than FAR's right-hand control
key because no terminal can tell the two control keys apart.

A digit holds one folder: setting it again somewhere else moves it.

`Ctrl+D` opens the whole list, where `Ins` keeps the directory you are in, `Del` gives one up
and `F4` gives it a name, so the list can read `work`, `logs`, `server` instead of three
paths. The ones with a digit come first, and the digit takes you there with one press.

A folder on a server or in a bucket can be kept like any other; going to it opens the
connection first.

# Tabs

A panel can hold more than one directory at a time. `Ctrl+Alt+T` opens another tab — on the
directory under the cursor if the cursor is on one, and otherwise on the one you are in, which
is a duplicate and what you want before going two different ways from the same place.
`Ctrl+Alt+N` and `Ctrl+Alt+P` step along the row, which is a ring: past the last tab is the
first one. `Ctrl+Alt+W` closes one. A panel always has a tab, so the last one stays.

A tab remembers everything the panel does — where it is, how it is sorted, what is selected,
where the cursor is, where it has been — so coming back to a tab is coming back to the work
you left in it.

`Ctrl+Alt+M` hands the tab to the other panel. It is a move: the tab leaves this side.

`Ctrl+Alt+K` locks a tab to its directory. A locked tab will not go anywhere — not with
`Enter`, not with the history, not driven from the other panel — and says so when asked,
rather than doing nothing. It is marked in the row. The one thing that unlocks it is the
connection under it closing: a tab cannot keep a promise about a directory it can no longer
reach.

The Left and Right menus hold all of that, and *Name this tab*, which is how a row comes to
read `src  build  logs` instead of three directory names. An empty name takes it off again.

The row of tabs is only drawn by a panel that has more than one: one tab looks exactly as it
always did, rather than costing a row of files to say that there is one tab.

Tabs come back when you start the program again, with their names and their locks. The
directory the shell was in still wins for the tab you land on — that is what running a
program in a directory means — unless that tab is locked. A tab that was on a server or in a
bucket is not restored, because the connection it needs is not open yet.

# A real terminal

The command line under the panels runs a command and keeps what it printed, which is what you
want nine times in ten. With something typed on it, `↑` and `↓` walk back and forth through
what was run before, as a shell does; on an empty line the arrows belong to the panel, as
every shared key does. `Alt+F8` shows the whole history as a list — and so do `Alt+F11` for the files you have read
or edited and `Alt+F12` for the folders this panel has been in. All three are lists you type
into: what you type narrows them, and the frame says what it was. `Ins` pins the row under the
cursor — a pinned row is marked with a `*`, and neither the size limit nor the lifetime can
push it out — `Shift+Del` takes one row out of the history for good, and `Del` empties the
list of everything you have not pinned, after asking. A line nobody has pinned and nobody has
used for ninety days goes by itself; *Options → System settings* is where that number lives.

While you type, the line offers what it could become: the names in the directory the panel is
showing, the lines you have run before, and the environment's variables. `↑` and `↓` walk the
list, `Tab` takes what is under the cursor, `Esc` closes it — and `Enter` still runs what you
typed until you have walked into the list, so the list is never in the way. `Ctrl+Space` asks
for it, and wakes the sources that wait to be asked: the executables on your `PATH` are the
one that does, because gathering them reads every directory on it.
*Options → Interface* is where each of the four sources is turned on, off, or set to answer
only when asked.

`Shift` with an arrow marks text out, `Ctrl+Shift` with one marks a word at a time, and
`Shift+Home`/`Shift+End` take it to either end. What is marked is drawn with its colours
swapped; `Ctrl+Ins` then copies *that* rather than the whole line, and typing or pasting
replaces it. Any arrow without `Shift` drops the mark.

The tenth is `vim`, `top`, `ssh` or anything else that draws a whole
screen and asks the system how big it is: those need a *terminal*, and `Ctrl+Alt+E` is one.

It is a screen like the viewer and the editor. `Ctrl+Alt+E` again goes back to the panels and
leaves the program running; `F12` lists it among whatever else is open and brings it back;
pressing `Ctrl+Alt+E` from the panels brings a parked terminal forward rather than starting a
second one. A second one is that key again after parking the first, which is deliberate: a key
that quietly started a process every time you pressed it would leave a dozen shells behind in
a day.

Inside it, every key belongs to the program — `F3` means what `F3` means to whatever is
running, `Tab` completes, `Ctrl+C` interrupts. Exactly one key is ours, and it is the one that
got you in. There is no key bar for the same reason: a bar offering our keys in there would be
telling you something untrue.

The shell is the one your session already uses (`$SHELL`, or `ComSpec` on Windows), so your
aliases and your prompt are the ones you know. `[term] shell` in `fcmd.toml` names another,
as a command line: `bash -l`, or a path with spaces in quotes. It starts in the directory the
active panel is looking at, when that is a real directory — a panel inside an archive or on a
server has none, and the shell starts where the program did.

When the program ends, its last screen stays up to be read, and the next key closes it.

# Choosing files

`Insert` selects the row and moves on. `Gray +` and `Gray -` select and unselect by a file
mask; `Gray *` inverts. `Ctrl+A` of a mask, `Alt+letters` to find a name as you type.

What is selected is what the next operation acts on. With nothing selected it is the row under
the cursor — so `F5` on one file copies that file, and nobody has to select it first.

# The mouse

Off until you ask for it, in `F9 → Options → Interface settings`, and the reason is worth
knowing: while a terminal is reporting the mouse it stops acting on it, so selecting text with
the mouse and the middle-click paste stop working. Those are your terminal's own tools and this
program cannot replace them, so it does not take them away unasked.

With it on: a click moves the cursor to the row and makes that panel the active one, a double
click is `Enter` on it, the right button selects a row, and the wheel scrolls whatever is under
the pointer — three rows a notch, without taking the focus, so you can read the other panel
without leaving this one. The key bar always shows all twelve function keys, with the ones a
screen has no use for left empty, so `F5` is the fifth box along wherever you are. It is
clickable (hold `Shift` and click for `Shift+F5`), so
is the menu bar and a dialog's list. A click outside an open dialog does nothing at all.

# Copying and pasting text

`Ctrl+Ins` copies the names of what is selected — one per line — and `Alt+Shift+Ins` copies
them with their whole paths. With something on the command line, `Ctrl+Ins` copies that line
instead: the same rule as every other shared key, which belongs to the panel only while the
line is empty.

The copying is done by your terminal, through a request called `OSC 52`, and that is why it
also works over `ssh`: the clipboard that fills is the one on the machine you are sitting at,
not the one on the machine the program runs on. A terminal that does not understand the
request ignores it, and it cannot answer either way, so what you are told is what was copied —
not that the clipboard took it. `F9 → Options → Interface settings` turns it off.

Pasting is your terminal's key, not ours — `Shift+Ins` or `Ctrl+Shift+V` in most of them —
and the text arrives here in one piece rather than as a run of keystrokes. The editor takes
all of it, newlines and all; the command line, a dialog row and a prompt are one line each, so
they take the first line and say how many more there were. A command copied from a web page
comes with its newline still attached, and running it before you have read it is exactly what
that rule avoids.
