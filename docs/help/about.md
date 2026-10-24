# What this is

SupremeCommander is a two-panel file manager that works the same way on Linux, macOS and
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

`F1` opens this help, from anywhere. `F9` is the menu bar, which names every dialog and the
key that opens it. `F10` leaves whatever you are in.

The key bar along the bottom always says what `F1` to `F12` do *right now* — in the panel, in
the viewer, in the editor — and it follows the modifier you are holding, so holding `Shift`
shows the shifted row.

Nothing is hidden behind a mouse: every action in the program is a named command, every
command can be bound to a key, and the "Keys" and "Commands" pages of this help are generated
from the keymap the program is actually running, so they cannot drift from the truth.
