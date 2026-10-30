# The viewer (F3)

The viewer never loads a file. It reads the window it is showing, so a forty-gigabyte log
opens at once and `End` is one seek rather than a count of lines.

`F2` cycles the wrapping, `F4` turns the wheel of ways to read the file — text, a byte dump,
hex, a laid-out document, a table — stopping only on the ones this file has a use for, with
the key bar saying where the next turn lands; `Shift+F4` is hex and straight back.

`Shift+F8` is every code page the program offers, as a list you type into: `Enter` reads the
file as one, `Ins` marks a page as one you use — it gets a `*` and comes to the top — and `F4`
gives it a name of your own. `F8` then walks the pages you marked, and the five the program
handles itself while you have marked none.

`F7` opens the search dialog: what to look for, whether it is plain text, a run of hex
bytes or a regular expression, whether case matters, whether only whole words count — and
then *Find next* or *Find previous*, because they are the same question asked twice.
`Shift+F7` and `Alt+F7` repeat it forwards and backwards without asking again. `F6` hands the file to the editor at the same place.

In the hex dump the arrows set how many bytes a row holds, since sideways means nothing else
in a dump: `→` widens it and `←` narrows it, **eight bytes at a time** — 8, 16, 24, 32 and so
on up to 64 — because eight is the group the dump already draws a gap between, so every press
adds one whole group and the columns stay where your eye expects them. A press that would need
a wider window than you have leaves the width alone. `Ctrl+→` and `Ctrl+←` are something else:
they nudge the whole dump along by a single byte, so a structure that does not begin on a
multiple of the row width can be read in line instead of diagonally.

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

# Pictures

A `.png`, a `.jpg`, a `.gif` or a `.bmp` opens as the picture. `F4` goes on from there to the
dump and the hex for the times you came for the bytes — the file is still a file.

It is drawn in half blocks: a cell is about twice as tall as it is wide, so one holding `▀`
with the top half one colour and the bottom another is two pixels that are nearly square, and
the window is a canvas twice as tall in pixels as it is in rows. It is coarse, and it works in
every terminal there is — which is why it comes before kitty's, iTerm2's and sixel's protocols
rather than after them.

What the file *is* decides, not what it is called: a `.txt` that is really a PNG opens as the
picture. A photograph is fitted into the window keeping its shape and a small icon is left at
its own size rather than blown up into coloured squares. What is transparent shows the
viewer's own background through it.

Nothing scrolls — the picture is already the size of the window — and a file that is damaged,
too large, or not a picture says so instead of leaving you looking at a blank screen.

# Base64

`Alt+B` decodes a file that is base64 and shows what came out, keeping the file's own name in
the title. A certificate, a key, an attachment pulled out of a mail, a `data:` URI pasted into
a file: all of them are bytes written down in sixty-four printable characters, and all of them
are unreadable until something undoes that.

What came out decides how it is drawn: text when it is text, the hex pane when it is not,
which is the usual case. The status line says how many bytes came out, which is how you know
the decode was the right one.

Three shapes are read: the bare payload, wrapped or not, padded or not; **PEM armour**, where
the `-----BEGIN …-----` lines are skipped and what is between them is decoded; and a **`data:`
URI**, where everything up to `;base64,` is a label. Both alphabets work — the standard one
and the URL-safe one a token uses.

A file that is only *mostly* base64 is refused, and the message says which byte stopped it and
where: decoding as far as the first bad character would hand you bytes nobody wrote. The key
does not toggle back, because this is the one view that replaces the file rather than drawing
it another way — `Esc` and opening it again are the way back.

# Tables

A `.csv`, a `.tsv`, a `.jsonl` or a `.dbf` opens as a table rather than as the commas, the
braces or the hex: a
heading that stays where it is while the rows scroll under it, columns lined up, one record to
a row. `Shift+F3` switches between the table and the source — the source is sometimes the thing
one came to look at — and `Shift+F4` has the table in the cycle with the text, the dump and the
hex. `F7` searches what is on the screen, so looking for a value finds it in its cell.

`Shift+F3` is not limited to those names: it makes a table of **any** file that reads as one,
so a `.txt` of tab-separated columns or a log laid out in columns opens as a table too. It
refuses only a file with no table in it — one that is not text at all, or one that does not
come apart into two columns or more — and since the same key goes back, a wrong guess costs
one keystroke. What opens *by itself* is the shorter list, because what a file opens as has to
be predictable.

`←` and `→` move the window sideways across a table wider than the screen, and stop where the
last column ends rather than scrolling on into nothing. `Ctrl+←` and `Ctrl+→` jump a whole
column at a time — a table has a unit for sideways that a wall of text has not. `Home` and
`End` go to the first and the last screenful of rows.

`Ctrl+F3` sorts the rows by the column you are on: once up, again down, a third time back to
the order the file has them in. `Ctrl+F7` asks what a row has to hold and keeps only those
rows; an empty answer gives them all back. A column of numbers sorts as numbers, so 1000 comes
after 9 rather than before it, and the status line says what the view is doing — `↑name`,
`/tor` — so you can always see it. Neither writes anything: what changes is the order the rows
are read in, the file is untouched, and a search or a bookmark still means the record itself.
A table of more than two hundred thousand records refuses both and says so, because reading a
million records for one keystroke is not what this mode is for. Changing a cell is not here on
purpose: a viewer that writes is an editor, and `F4` is where that lives.

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
apply to it. Dates come out as `YYYY-MM-DD`, true and false as words, a
memo field as what the `.dbt` or `.fpt` beside the table holds, on one line, and the numeric
fields sit on the right of their column, because a column of numbers is read down its last
digit. A **deleted** record is shown rather than left out — a table whose deleted rows vanish
is one whose count does not add up — and the `*` that says so has a narrow column of its own
in front of the fields, where it cannot be mistaken for part of a number.

The text comes out in the code page the header names, DOS ones included: a table written under
DOS in Germany is in code page 437, and `temporäre` reads as `temporäre` rather than as
`tempor„re`.

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

`F5` disassembles. x86, x86-64 and AArch64 are decoded in the program itself; on another
architecture it asks `objdump`, and says so instead of guessing. AArch64 instructions are four
bytes each, so a word nothing decodes is one `(bad)` line rather than four, and a branch is
shown as the address it arrives at rather than as the displacement in the encoding. `Enter` on a `call` or a `jmp` follows it and
`Backspace` comes back, with the places you jumped from kept on screen.

Over a disassembly `F7` searches the *instructions*, not the bytes: `call` finds every call,
`mov *, eax` every move into that register, and `*` stands for whatever lies between the pieces
of the pattern.

`F11` opens the structure of an executable: ELF, PE, Mach-O, `ar`, and the older families — NE,
LE and LX, and the TE that UEFI firmware is made of — each header field with the
raw bytes beside the meaning, the sections, the imports and exports, and the symbols — which
then name the functions in the disassembly. `Enter` on an offset takes the bytes there.

The same key answers for the files that are not programs:

- **a video or a sound** — MP4, MOV, M4A, MKV, WebM, AVI, WAV: how long it is, the tracks, the
  codec of each, the resolution or the sample rate, the language, the tags, and the boxes or
  chunks of the container as rows `Enter` follows;
- **a PDF**: the version, how many pages, the paper of the first one, the title, the author and
  the producer, whether it is encrypted and what it then allows, how many times it has been
  saved since it was written — and the pages themselves, each row going to its object.

Neither shows a picture of the file: for a frame of a video or a page of a PDF rendered as an
image, set up a `[[decompile]]` rule (`ffprobe`, `pdftotext`) and press `Shift+F6`.

Source files, class files and bytecode get a listing of their own where one makes sense: a
`.class` is shown the way `javap -c` shows it, with the constant pool resolved into the
comments.

# Two files side by side

`Ctrl+Shift+D` — *View file differences* in the Commands menu — puts the file under the left
panel's cursor beside the file under the right panel's and marks what differs. *Compare by
content* answers which of a hundred pairs differ; this answers what, in one pair, is
different.

Each column carries its own line numbers, and a line that exists on one side only has a number
on that side and nothing opposite it. Where a line changed rather than appeared or vanished,
the part that actually changed is marked inside it, so the eye goes to the one character that
matters instead of reading both lines.

`Alt+↓` and `Alt+↑` go from one difference to the next, which is how one reads a diff; the last
one says it is the last rather than wrapping round. The arrows, `PgUp`, `PgDn`, `Home` and
`End` move by rows, `←` and `→` scroll both columns together, and `Tab` changes which column is
the one being read. `F12` lists this screen with the open viewers and editors, `F1` opens this
chapter, and `Esc`, `F3` or `F10` closes it.

Three keys change what counts as a difference, and each lines the two files up again at once:
`W` ignores blanks, `C` ignores case, `E` ignores how the lines end. Line endings start from
what the comparison dialog was last told, so the two agree about what "the same" means — the
same checkout on Windows and on Linux differs in every line and in nothing that matters.

Two files that are the same open all the same, and the screen says so rather than leaving you
scrolling for a difference that is not there.

Both files have to be on this disk and under 16 MiB: the whole of each is read into memory to
line them up, so a file on a server or inside an archive says so by name rather than fetching
a copy you did not ask for.
