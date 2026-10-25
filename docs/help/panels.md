# The two panels

Each panel shows one directory. `Tab` moves between them; most operations go from the panel
with the cursor to the other one, which is what makes a two-panel manager faster than a tree:
the destination is already on screen.

`Enter` enters a directory or runs a file, `Backspace` or `..` climbs. `Alt+F1` and `Alt+F2`
open the change-drive dialog for the left and right panel: a table of everything mounted —
what each one is (disk, removable, optical, network, memory), what it is called, what it is
formatted as, how big it is and how much is left. Underneath, numbered `0` to `9`, are the
things that are not filesystems: the open connections and the object stores that are
configured. The number takes you there without the arrows. What the system runs on —
anything under `/proc`, `/sys`, `/dev`, `/run` — is left out, because none of it is
somewhere to put a file.

# Looking at a directory

`Ctrl+1` to `Ctrl+5` choose how rows are drawn: brief, medium, full, wide, detailed. The
multi-column views run down a column and then across, as the originals do.

`Ctrl+F3` to `Ctrl+F6` sort by name, extension, time and size; choosing the order already in
force reverses it. `Ctrl+F12` offers the orders as a list.

`Ctrl+H` shows the hidden files, `Ctrl+F1` and `Ctrl+F2` hide a panel to give the other one
the whole screen, and `Ctrl+O` hides both to show the output of the commands that have run.

`Ctrl+R` re-reads. `Ctrl+L` turns the other panel into an information panel, `Ctrl+Q` into a
preview of the file under the cursor, `Ctrl+T` into a directory tree.

`Ctrl+←` and `Ctrl+→` move the line between the two panels; `Ctrl+↑` and `Ctrl+↓` take rows
off both of them and give them to the output of the last command, which is how you watch a
build go by without leaving the panels. Both are settings, so `Shift+F9` keeps them.

# A real terminal

The command line under the panels runs a command and keeps what it printed, which is what you
want nine times in ten. The tenth is `vim`, `top`, `ssh` or anything else that draws a whole
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
aliases and your prompt are the ones you know. `[term] shell` in `scmd.toml` names another,
as a command line: `bash -l`, or a path with spaces in quotes. It starts in the directory the
active panel is looking at, when that is a real directory — a panel inside an archive or on a
server has none, and the shell starts where the program did.

When the program ends, its last screen stays up to be read, and the next key closes it.

# Choosing files

`Insert` selects the row and moves on. `Gray +` and `Gray -` select and unselect by a file
mask; `Gray *` inverts. `Ctrl+A` of a mask, `Alt+letters` to find a name as you type.

What is selected is what the next operation acts on. With nothing selected it is the row under
the cursor — so `F5` on one file copies that file, and nobody has to select it first.
