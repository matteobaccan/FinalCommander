# The editor (F4)

Lines, with a block, undo over the whole session, and a save that writes a new file beside the
old one and renames it over the top — so an interrupted save never leaves half a file.

`F2` saves, `Shift+F2` saves under another name, `F7` searches and `Ctrl+F7` replaces.
`Ctrl+Z` and `Ctrl+Y` undo and redo. `F6` hands the file to the viewer at the same line.

A column block is a rectangle rather than a run of lines, which is how a column of numbers is
moved without touching what is beside it.

# Bytes instead of lines

`F4` on a binary opens the same editor on its bytes, and `Alt+H` switches either way. Two
panes, the hex and the text; `Tab` moves between them and both type. `Insert` chooses between
overwriting and inserting — in a binary a shifted byte usually breaks the file, so overwriting
is what it starts in.

Changed bytes are drawn as changed until they are written, with the list of what changed beside
them: offset, old byte, new byte. `Ctrl+Z` takes one back. `F2` writes, atomically.

`F3` marks a block and `F12` says what to do with it: write it out to a file, read a file in,
fill it with a byte, exclusive-or it with a key, copy or move bytes from elsewhere in the same
file, insert zeros, delete. Each one says what the file's length will become before it happens.

`F9` assembles one instruction where the cursor is — `ret`, `xor eax, eax`, `mov rax, 1`,
`jmp 0x401010`, `call rax`. A branch is written as the address you want to arrive at, not as a
displacement. Whatever it encodes is decoded again before it is shown, so what you see is the
instruction the processor will see. Memory operands are refused by name rather than guessed at.

`Alt+=` is the calculator: one line of arithmetic answered in decimal, hex, octal, binary and
as characters at once. It reads the literals binary tools print — `0x1f`, `1Fh`, `0b1010`,
`'PE'` — and the same reader takes the numbers typed into the assembler and the block
operations.

# Devices

On Linux and macOS a disk is a file, so the hex editor opens `/dev/sdb` read-only and the
status line says what it is and how long it measured. The bytes are written *out* to a file,
never back over the disk.
