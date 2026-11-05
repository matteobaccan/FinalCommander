# What this is

FinalCommander is a two-panel file manager that works the same way on Linux, macOS and
Windows. One program, one set of keys, one configuration file — on every machine you sit at.

That sameness is the point. The orthodox file manager is a forty-year-old idea that every
platform reinvented separately: Norton Commander, then FAR on Windows, Midnight Commander on
Unix, half a dozen others on each. Each is good; none of them is everywhere, and the keys are
never quite the same twice. So the habits learned on one machine stop working on the next.

This rebuilds the idea portably instead of porting one of them. The keys are FAR's by default
because that is the most complete of them, with a Midnight Commander profile for the fingers
that learned those; the panel is drawn in FAR's own colours; and the parts that were only ever
on one platform — the archive handling, the viewer's disassembler, the hex editor — are here on
all three.

# Where to start

`F1` opens the help for where you are: the keys of that screen, each with a line saying what
it does, grouped by subject. `Right` from there is the chapter that explains the screen at
length, `Esc` is the table of contents, and leaving the help puts you back where you were.
`F7` anywhere in the help looks for a word in every chapter at once and lists the lines that
have it; `Enter` on one opens its chapter there.
`F9` is the menu bar, which names every dialog and the key that opens it: it lights up without
covering anything, and `Enter`, `↓` or the letter drawn on a title pulls that menu down —
`Esc` rolls it back up and a second one gives the panels back. `F10` leaves whatever you are
in.

The key bar along the bottom always says what `F1` to `F12` do *right now* — in the panel, in
the viewer, in the editor — and it follows the modifier you are holding, so holding `Shift`
shows the shifted row.

Nothing is hidden behind a mouse: every action in the program is a named command, every
command can be bound to a key, and the "Keys" and "Commands" pages of this help are generated
from the keymap the program is actually running, so they cannot drift from the truth.

# Coming in, and going out

`fcmd --view FILE` starts on the viewer, `fcmd --edit FILE` on the editor and `fcmd --diff A
B` on the two files side by side; leaving that screen ends the program rather than dropping
into the panels. That is what makes it usable where a file manager is *called*: as
`$VISUAL`, as `git difftool`, as a pager in a script. A name that is not there opens an empty
editor, as `Shift+F4` does.

Leaving the program can leave the shell where the panel was, as Midnight Commander's does. A
program cannot change the directory of the shell that started it, so it takes a small function
in the shell, which `fcmd --shell-init` prints for `bash`, `zsh`, `fish` or `powershell`. One
line in the shell's startup file is all of it:

    eval "$(fcmd --shell-init bash)"

From then on `fcmd` is the function: it runs the program with `-P FILE`, which writes down the
directory the active panel ended in, and goes there. Quitting inside an archive leaves you in
the folder that holds it; a panel on a server leaves the shell where it was.

