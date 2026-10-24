# The viewer (F3)

The viewer never loads a file. It reads the window it is showing, so a forty-gigabyte log
opens at once and `End` is one seek rather than a count of lines.

`F2` cycles the wrapping, `F4` switches between text and a hex dump, `F8` changes the code page
and `F7` searches — plain text, a regular expression, or a run of hex bytes, with `Shift+F7`
and `Alt+F7` for the next and previous hit. `F6` hands the file to the editor at the same place.

`Gray +` and `Gray -` step to the next and previous file of the panel without leaving the
viewer, and `Ctrl+F10` makes the panel catch up with what you are reading.

# Binaries

`F5` disassembles. x86 and x86-64 are decoded in the program itself; on another architecture it
asks `objdump`, and says so instead of guessing. `Enter` on a `call` or a `jmp` follows it and
`Backspace` comes back, with the places you jumped from kept on screen.

Over a disassembly `F7` searches the *instructions*, not the bytes: `call` finds every call,
`mov *, eax` every move into that register, and `*` stands for whatever lies between the pieces
of the pattern.

`F11` opens the structure of an executable: ELF, PE, Mach-O and `ar`, each header field with the
raw bytes beside the meaning, the sections, the imports and exports, and the symbols — which
then name the functions in the disassembly. `Enter` on an offset takes the bytes there.

Source files, class files and bytecode get a listing of their own where one makes sense: a
`.class` is shown the way `javap -c` shows it, with the constant pool resolved into the
comments.
