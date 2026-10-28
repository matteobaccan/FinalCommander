# The viewer (F3)

The viewer never loads a file. It reads the window it is showing, so a forty-gigabyte log
opens at once and `End` is one seek rather than a count of lines.

`F2` cycles the wrapping, `F4` switches between text and a hex dump, `F8` changes the code page
and `F7` opens the search dialog: what to look for, whether it is plain text, a run of hex
bytes or a regular expression, whether case matters, whether only whole words count — and
then *Find next* or *Find previous*, because they are the same question asked twice.
`Shift+F7` and `Alt+F7` repeat it forwards and backwards without asking again. `F6` hands the file to the editor at the same place.

`Gray +` and `Gray -` step to the next and previous file of the panel without leaving the
viewer, and `Ctrl+F10` makes the panel catch up with what you are reading.

Three keys give the window to the file. `Ctrl+B` takes the key bar away, `Ctrl+Shift+B` the top
line with the name and the position, and `Ctrl+S` puts a scrollbar down the right edge — each
row you take away is a row of text you get, which is what a long line or a wide hex dump wants.
They apply to the file you are reading and nothing is written down: `F9 → Options → Viewer
settings` is where you say what every file should open with, along with the wrapping and the
tab width.

# Markdown

A `.md` file opens laid out rather than as its source: the marks come off and the space they
held is put to use. A heading is bold with a rule under it, a list is bullets with hanging
indents and a task list is boxes, a quote gets a bar down its left side, a link shows its text
and its address, a fenced block is indented and — where the fence named a language — coloured
as that language, and a table is drawn with its columns lined up and aligned the way its
delimiter row asked for.

`Shift+F2` switches between the laid-out document and its source, and `Shift+F4` has it in the
cycle with the text, the dump and the hex. Searching a laid-out document searches what is on
screen, not the marks behind it, so looking for a word in a table finds it in the cell.

This is the one place the viewer reads more than it shows: a table's columns cannot be worked
out from the part of it that happens to be on screen, so the document is rendered whole. Over
four megabytes it stays the source, and says so. *Interface → Markdown laid out to be read*
turns the first screen back into the source for good.

# Tables

A `.csv`, a `.tsv`, a `.jsonl` or a `.dbf` opens as a table rather than as the commas, the
braces or the hex: a
heading that stays where it is while the rows scroll under it, columns lined up, one record to
a row. `Shift+F3` switches between the table and the source — the source is sometimes the thing
one came to look at — and `Shift+F4` has the table in the cycle with the text, the dump and the
hex. `←`/`→` move the window sideways across a table wider than the screen, and `F7` searches
what is on the screen, so looking for a value finds it in its cell.

The status line says what the file was read as, which delimiter was *guessed* — a comma, a
semicolon, a tab or a bar, whichever gave a table — and how many rows and columns there are:
that is how you can tell at a glance that the guess was right. A `.tsv` is not guessed at. For
JSON Lines the columns are the keys of the first two hundred records, in the order they are
written in the file; a record with a key that turned up later shows it in a last column called
`…` rather than losing it, and a line that is not an object is counted at the end of the status
line rather than shown as a row of nonsense.

A `.dbf` is different and better: a dBase table says in its own header how many records it
has, how long one is and what the columns are called, so nothing is read but the window you are
looking at — a two-gigabyte table opens as fast as a log, and the size limit below does not
apply to it. Dates come out as `YYYY-MM-DD`, true and false as words, a memo field as the block
number it holds (reading the `.dbt`/`.fpt` beside the table is not here yet), and a **deleted**
record is shown with a `*` in front of it rather than being left out: a table whose deleted
rows vanish is one whose count does not add up.

Like a laid-out Markdown document, a text table is read whole — a column's width cannot be
known from the part of the file on screen — so over sixteen megabytes the file opens as text
and says so. *Interface → Open a .csv or a .jsonl as a table* turns the first screen back into the
source for good. A cell longer than forty columns is cut with a `…`; the horizontal scroll is
how the rest of it is read.

What this mode is not: a spreadsheet and a query tool. There is no editing, no sorting and no
filtering, because each of those wants an answer about writing the file back, which a viewer
does not have.

Text is coloured by the language the file's name implies, as it is in the editor. The viewer
reads only what it shows, so the colouring starts at the top of the screen: a comment that
was opened higher up in the file is not carried down to it. That is the price of opening a
forty-gigabyte log at once, and only the viewer pays it — the editor has the whole file and
colours it exactly.

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
