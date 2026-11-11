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

`Shift+F` follows the file as it grows, the way `tail -f` does: once a second the viewer looks,
and when something has been written it shows the new end. Scroll back to read and it leaves
you there until more arrives; `Shift+F` again stops it.

In the hex dump the arrows set how many bytes a row holds, since sideways means nothing else
in a dump: `→` widens it and `←` narrows it, **eight bytes at a time** — 8, 16, 24, 32 and so
on up to 64 — because eight is the group the dump already draws a gap between, so every press
adds one whole group and the columns stay where your eye expects them. A press that would need
a wider window than you have leaves the width alone. `Ctrl+→` and `Ctrl+←` are something else:
they nudge the whole dump along by a single byte, so a structure that does not begin on a
multiple of the row width can be read in line instead of diagonally.

`Gray +` and `Gray -` step to the next and previous file of the panel without leaving the
viewer, and `Ctrl+F10` goes back to the panels with the cursor on the file you are reading —
in the folder the panel shows if it is there, and otherwise in the file's own folder, which is
where a file opened from a search or from `fcmd --view` takes it.

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
column at a time — a table has a unit for sideways that a wall of text has not. `↑` and `↓`
move a cursor from row to row, and the rows scroll under it once it reaches the edge of the
window; `Home` and `End` go to the first and the last row.

Rows are marked the way files are in a panel: `Ins` or `Shift+↓` marks the row under the cursor
and moves down, `Shift+↑` moves up, and the same key on a marked row takes the mark off.
`Ctrl+C` (or `Ctrl+Ins`) puts the marked rows on the clipboard — the row under the cursor when
none is marked — with the heading first, the cells separated by tabs, which is what a
spreadsheet pastes into its own cells; a cell holding a tab, a line break or a quote is quoted
so the paste comes apart in the same places. The rows keep their marks through a sort or a
filter, and are copied in the order you see them.

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
semicolon, a tab, a bar or a caret (`^`), whichever gave a table — and how many rows and
columns there are: that is how you can tell at a glance that the guess was right. A `.tsv` is
not guessed at.

Whether the first line is a **heading** is guessed too, from what is under it: a word over a
column of numbers, or over a column of codes that all have one length, is a heading; a number
over numbers, or a value that turns up again further down, is data. A file with no heading
shows its columns as `#1`, `#2`… and the status line says `no heading`. When the guess is
wrong, `Ctrl+H` turns it round — the heading becomes the first row, or the first row the
heading. JSON Lines and dBase name their own fields, so there is nothing for it to turn. For
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

`F5` disassembles, in colour: jumps, calls and returns stand out from the rest, registers and
numbers are told apart, and each function's name heads its code. The hex view is coloured by
byte — zeros dim, so the code and the strings in a file are where the colour is. x86, x86-64
and AArch64 are decoded in the program itself; on another
architecture it asks `objdump`, and says so instead of guessing. AArch64 instructions are four
bytes each, so a word nothing decodes is one `(bad)` line rather than four, and a branch is
shown as the address it arrives at rather than as the displacement in the encoding. `Enter` on a `call` or a `jmp` follows it and
`Backspace` comes back, with the places you jumped from kept on screen.

Over a disassembly `F7` searches the *instructions*, not the bytes: `call` finds every call,
`mov *, eax` every move into that register, and `*` stands for whatever lies between the pieces
of the pattern.

`F11` (or `Shift+F11`, which FAR's habits leave free) opens the structure of an executable: ELF, PE, Mach-O, `ar`, and the older families — NE,
LE and LX, and the TE that UEFI firmware is made of — each header field with the
raw bytes beside the meaning, the sections, the imports and exports, and the symbols — which
then name the functions in the disassembly. `Enter` on an offset takes the bytes there.

It also says what the program says about itself:

- **a Windows program** (PE): the version resource — file and product version, company,
  description, copyright, original name — the execution level its manifest asks for (whether
  starting it raises a UAC prompt), the mitigations it opts into (ASLR, DEP, Control Flow
  Guard), the PDB its debug information went to, and whether it is **signed** and by whom;
- **a Linux program** (ELF): the build id, the oldest kernel it runs on, the compilers that
  built it, the file its debug information was split into, a Go build id or the package it
  came from;
- **a macOS program** (Mach-O): the UUID, the oldest macOS or iOS it runs on and the SDK, the
  tools, each library's version, and the **code signature** — identifier, team, ad-hoc or not,
  hardened runtime.

A signature is read and **not verified**: the pane names the certificates, and says so. Whether
a signature is valid depends on the machine that will trust it, so that check is left to the
system's own tools.

The same key answers for the files that are not programs:

- **a video or a sound** — MP4, MOV, M4A, MKV, WebM, AVI, WAV: how long it is, the tracks, the
  codec of each, the resolution or the sample rate, the language, the tags, and the boxes or
  chunks of the container as rows `Enter` follows;
- **a PDF**: the version, how many pages, the paper of the first one, the title, the author and
  the producer, whether it is encrypted and what it then allows, how many times it has been
  saved since it was written — and the pages themselves, each row going to its object;
- **a photo** — JPEG, PNG, GIF: the size, and for a JPEG the EXIF — camera, lens, when it was
  taken, exposure, aperture, ISO — and **where it was taken**, on a row of its own, because
  that is the thing in a photo people most often do not know they are sending; a PNG's text
  (compressed or not) and time; a WebP's size, animation and EXIF;
- **a picture an image model made** — an **AI generation** section with what it was made
  with and how: the tool (ComfyUI, AUTOMATIC1111, Forge, Fooocus, SwarmUI, InvokeAI, NovelAI),
  the model, its LoRAs and their weights, the text encoders and the VAE, the prompt and the
  negative one, the seed, the steps, cfg and Flux guidance, the sampler, the scheduler and the
  size. ComfyUI's graph is followed back from the sampler — `KSampler`, or Flux's
  `SamplerCustomAdvanced` through `RandomNoise`, `FluxGuidance`, `BasicScheduler`,
  `UNETLoader` and `DualCLIPLoader` — so the summary is what actually ran. `Enter` on the
  *prompt (graph)* or *workflow* row opens the whole of it, the JSON laid out, and `F4` goes
  back. A picture with **content credentials** (C2PA) says who signed the claim and whether it
  calls the picture generated. To take any of this out, **Files → Strip picture metadata**
  (`file.strip_metadata`) on the pictures in the panel: all of it, only what the image model
  wrote, or all but the camera's EXIF and the copyright (the GPS position goes). The picture
  is not encoded again — its pixels are the same bytes — and `Ctrl+Alt+Z` puts the metadata
  back;
- **an office document** — `.docx`, `.xlsx`, `.pptx`, `.odt`, `.ods`, `.odp`: the title, the
  author and who saved it last, when it was made and changed, how many revisions, pages, words
  or slides, the company and the program that wrote it;
- **music** — MP3, FLAC, Ogg Vorbis, Opus: the title, the artist, the album, the year, the
  track, the genre, whether there is a cover, and for FLAC and Ogg the stream and its length.

Neither shows a picture of the file: for a frame of a video or a page of a PDF rendered as an
image, set up a `[[decompile]]` rule (`ffprobe`, `pdftotext`) and press `Shift+F6`.

`Shift+F5` does the same thing without a rule: it asks for a command, hands it this file, and
reads what the command printed instead of the file — `strings`, `xxd`, `pdftotext`, `objdump
-d`, `jq .`, a script of your own. The file is put at the end of the line unless the line
already names it (`!.!` is where it goes if you want it in the middle), the whole of it is run
by your shell, so pipes work, and `Backspace` goes back to the file itself. It is Midnight
Commander's filtered view, and it is the answer to every format nobody has written a rule for.

`Ctrl+Alt+G` — the panel's git key — asks git who wrote the file: `git blame`, the commit, the
author and the date in front of every line, opened on the line that was at the top so what you
were reading is still on the screen. `Backspace` goes back to the file.

Source files, class files and bytecode get a listing of their own where one makes sense: a
`.class` is shown the way `javap -c` shows it, with the constant pool resolved into the
comments.

# Two files side by side

`Ctrl+Shift+D` — *View file differences* in the Commands menu — puts the file under the left
panel's cursor beside the file under the right panel's and marks what differs. *Compare by
content* answers which of a hundred pairs differ; this answers what, in one pair, is
different.

The left file is the *before* and the right one the *after*, and the screen reads like a
diff: what only the left side has is painted on red, the whole line, with a `-` beside its
number, what the right side brings is painted on green with a `+`, and a line that changed is both — red on the left, green on the
right — with the part that actually changed marked inside it, so the eye goes to the one
character that matters. Each column carries its own line numbers, and a line that exists on one
side only has a number on that side and nothing opposite it. The foot says which difference of
how many is on screen, and how many lines went and came (`-3 +5`).

`Alt+↓` and `Alt+↑` go from one difference to the next, which is how one reads a diff; the last
one says it is the last rather than wrapping round. The arrows, `PgUp`, `PgDn`, `Home` and
`End` move by rows, `←` and `→` scroll both columns together, and `Tab` changes which column is
the one being read. `F12` lists this screen with the open viewers and editors, `F1` opens this
chapter, and `Esc`, `F3` or `F10` closes it.

`F5` takes a difference across: the column you are in takes the other one's version of the
difference at the top of the screen, so `Tab` chooses which file changes and `Alt+↓` chooses
which difference. Whole lines move — a line only the other side has is added, a line only this
side has goes — and a line coming out of a Unix file into a DOS one is given DOS line endings,
so you do not end up with the one odd line in it. On a line both files agree about, `F5` says
there is no difference here rather than taking the next one by surprise.

Nothing is written until `F2`. The file that took something carries a `*` beside its name,
`F2` writes it — beside the original and renamed over it, so an interruption leaves what was
there whole — and once written the `*` goes, so a second `F2` has nothing to do and says so.
`Esc` with something unwritten says so and leaves at the next press, which is how a screen with
no dialog of its own asks whether you meant it.

Three keys change what counts as a difference, and each lines the two files up again at once:
`W` ignores blanks, `C` ignores case, `E` ignores how the lines end. Line endings start from
what the comparison dialog was last told, so the two agree about what "the same" means — the
same checkout on Windows and on Linux differs in every line and in nothing that matters.

Two files that are the same open all the same, and the screen says so rather than leaving you
scrolling for a difference that is not there.

Both files have to be on this disk and under 16 MiB: the whole of each is read into memory to
line them up, so a file on a server or inside an archive says so by name rather than fetching
a copy you did not ask for.

# Delphi

Delphi's own files are read the way Delphi shows them.

**Forms.** A `.dfm`, `.fmx`, `.lfm` or `.xfm` saved as text is coloured — `object`, `end`,
the properties, the strings, the binary data in `{ … }`. One saved in **binary**, which starts
with `TPF0` and which older projects keep, opens on the text Delphi's *View as Text* would
show: the objects nested, `inherited` and `inline` where they are, every property `Name =
value`, sets in `[ … ]`, lists in `( … )`, collections in `< … >` and binary data as hex. `F4`
goes to the bytes, and back.

**Programs.** `F11` on an `.exe`, `.dll` or `.bpl` built with Delphi or C++Builder has a
*Delphi* part: which of the two built it, the packages it needs, how many units it holds and
their names, and every form inside it by name and class. `Enter` on a form opens it as text,
as if it were its own `.dfm`.

**Sources and projects.** `.pas`, `.dpr`, `.dpk`, `.lpr`, `.pp` and `.inc` are coloured as
Pascal, with Delphi's own reserved words and directives as keywords in any case, `$FF` and
`#13` as numbers, and `{$…}` directives apart from comments; a member after a dot —
`E.Message` — is a name, not the directive it is spelt like. A thin line runs down each
`begin … end`, `try … end`, `case`, `repeat … until` and declared `class` or `record`, as the
IDE draws it, and down each `object … end` of a form. A `{ … }` of binary data in a form, or
any comment opened above the window, keeps its colour when its first line scrolls off. The
theme `delphi-dark-2027` gives the whole program the IDE's dark colours.

**Projects.** `.dproj`, `.groupproj` and `.cbproj` are coloured as XML; `.dof` and `.dsk` as INI.

**What the IDE leaves behind.** The group `<delphi>` — `*.~*`, `__history`, `__recovery`,
`*.dcu`, `*.identcache`, `*.local`, `*.dsk` and the rest — is drawn dim, as temporary files
are, and `Gray +` with `<delphi>` selects it all for `F8`.
