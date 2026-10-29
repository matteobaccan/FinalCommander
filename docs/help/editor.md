# The editor (F4)

Lines, with a block, undo over the whole session, and a save that writes a new file beside the
old one and renames it over the top — so an interrupted save never leaves half a file.

`F2` saves, `Shift+F2` saves under another name, `F7` opens the same search dialog the viewer
has — text, hex or a regular expression, case, whole words, forwards or backwards — and
`Ctrl+F7` replaces.
`Ctrl+Z` and `Ctrl+Y` undo and redo. `F6` hands the file to the viewer at the same line.

A column block is a rectangle rather than a run of lines, which is how a column of numbers is
moved without touching what is beside it.

# Colours

A file whose name says what it is — Rust, C, C++, Java, C#, JavaScript, TypeScript, PHP,
Python, Ruby, Go, shell, SQL, HTML, XML, CSS, Markdown, JSON, TOML, YAML, INI, Make,
Dockerfile, Lua, Perl — is coloured as you read it, and the status line names the language so
you can see at once when a file is being read as something you did not mean. The colours come
from the theme, so they follow it. `ui.syntax = false` in `fcmd.toml` turns them off.

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
instruction the processor will see.

Addresses are written the way a disassembly prints them: `mov eax, [rbx]`, `[rbx+8]`,
`[rbx+rcx*4+8]`, `[0x401000]`, and `[rip+0x2e35]` with the displacement counted from the end of
the instruction, so a line read off the screen can be typed straight back. When no register
says how wide the thing at the address is, a size word does: `mov dword [rbx], 1`, or
`byte ptr` if you write masm. Without one it asks rather than guess, because one byte and eight
are not the same patch. What it will not do — segment prefixes, far pointers, the
floating-point and vector registers — it refuses by name.

`Alt+=` is the calculator: one line of arithmetic answered in decimal, hex, octal, binary and
as characters at once. It reads the literals binary tools print — `0x1f`, `1Fh`, `0b1010`,
`'PE'` — and the same reader takes the numbers typed into the assembler and the block
operations.

# Devices

On Linux and macOS a disk is a file, so the hex editor opens `/dev/sdb` read-only and the
status line says what it is and how long it measured. The bytes are written *out* to a file,
never back over the disk.

# Tabs, whitespace and the odd key

`F9 → Options → Editor settings` has the five that matter. **Tabs** is three choices and not a
box: leave them as they are, turn a newly typed tab into spaces, or expand every tab in the file
when it is opened — the last one changes the file, so it says how many lines it touched and the
next save writes that. **Show spaces and tabs** draws a space as `·` and a tab as `→` and its
padding, which is how a mixed indent becomes visible. **The cursor lands after the text a search
found** is for typing over what was found rather than reading it.

`Ctrl+Q` puts the next key in as itself: a control character, or a real tab where tabs are being
expanded. There is no other way to get either into a file.

`Alt+I` and `Alt+U` move the block — or, with nothing selected, the line the cursor is on — one
indent right and left. `Tab` still types a tab, because that is the key one presses while
writing.

`Ctrl+Shift+Ins` adds the block to what is already on the clipboard instead of replacing it, so
a few pieces can be gathered from here and there and pasted in one go. `Ctrl+N` and `Ctrl+E` put
the cursor on the first and the last line *on the screen*, leaving the screen where it is.
