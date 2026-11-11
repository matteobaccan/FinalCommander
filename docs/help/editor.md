# The editor (F4)

Lines, with a block, undo over the whole session, and a save that writes a new file beside the
old one and renames it over the top — so an interrupted save never leaves half a file.

`F2` saves, `Shift+F2` saves under another name. `F6` hands the file to the viewer at the same
line.

`F7` opens the same search dialog the viewer has — text, hex or a regular expression, case,
whole words, forwards or backwards — with one button more. **All matches** lists every line
that has a hit, with its number: type to narrow the list, `Enter` goes to the line, and
`Shift+F7` goes on from there like after any other search. *Whole words only* means the hit is
not a piece of a longer word: no letter, digit or underscore stands right before it or right
after it. That is true of `set` in `set = 1` and not in `offset`, and it works for something
that is not a word at all, like `++`.

`Ctrl+F7` replaces, and asks everything in one dialog: what to look for, what to put there,
text or regular expression, case, whole words, and *Preserve style*.

- In a **regular expression** the replacement can use what the groups caught: `$1` is the
  first group, `${name}` a named one, `$$` a dollar. Write `${1}x` when a letter follows the
  group, since `$1x` would be read as a group called `1x`. In a plain search a dollar is just
  a dollar.
- ***Preserve style*** gives a plain replacement the capitals of what it replaces: replacing
  `foo` with `bar` turns `Foo` into `Bar` and `FOO` into `BAR` as well. Three shapes are
  recognised — all lower case, all upper case, a capital first — and a word written any other
  way (`fooBar`) gets the replacement exactly as you typed it. It only matters when the search
  is not case sensitive.
- **Replace all** does the whole file, says how many it changed, and is one `Ctrl+Z`.
  **Replace next** changes the hit the cursor is on, or the next one, and selects the one
  after it, for going through a file one hit at a time.

The dialog opens on what you last looked for and how. Every replacement that ran is
remembered, and `Alt+Shift+F7` lists them — the ones kept under a name first, then the recent
ones — for `Enter` to run again without retyping a regular expression; `Ins` keeps a recent
one under a name (it goes into `fcmd.toml` as an `[[editor.replace]]` block), `Del` forgets a
kept one. One of these runs the way the dialog was last set: as text or as a pattern, with or
without whole words.

`Ctrl+Z` undoes and `Ctrl+Shift+Z` redoes. A run of typing is one step, as in FAR: the word
you just typed goes with one `Ctrl+Z`, not letter by letter. So is a run of `Backspace`, and a
run of `Del`. A step ends when you do something else — move the cursor, press `Enter`, give a
command, or change from typing to deleting. The editor keeps the last 1000 steps of each file;
`editor.undo_depth` in the settings (*Options → All settings*) changes that for the files
opened afterwards.

A column block is a rectangle rather than a run of lines, which is how a column of numbers is
moved without touching what is beside it.

# When the file changed underneath

If something else wrote the file while you were editing it — a build, a `git pull`, another
editor — `F2` does not save over it. It asks, and the question has four answers:

- **Overwrite** writes your text over what is there now.
- **Reload** reads the file again and drops your edits, which is also what `Ctrl+R` does at
  any time.
- **What changed** puts the two side by side: the file as it is on the disk on the left, your
  text with its unsaved edits on the right, on the same screen `Ctrl+Shift+D` compares two
  files with — `Alt+↓` and `Alt+↑` go from one difference to the next, `W`, `C` and `E` ignore
  spaces, case and line endings. It only shows: nothing is merged and nothing is written from
  there. `Esc` closes it and the question is asked again, now that you know what each answer
  would lose.
- **Cancel** goes back to the editor with everything as it was; `Shift+F2` from there saves
  your text under another name and leaves the file alone.

The arrows or `Tab` move between the buttons and `Enter` takes the one that is lit.

# End of line, byte order mark and code page

A save writes a file the way it was read: the same line endings, the same code page, a byte
order mark if it had one, and no newline after the last line if there was none. The status
line says which — `LF`, `CRLF` or `CR`, then the code page, then `BOM` when the file starts
with one.

`Shift+F8` is where you change it. The list has three groups, and in each the row that is true
now is marked: the end of line (LF, CRLF, CR), the byte order mark (written or not), and every
code page, in the order the viewer's list has them. `Enter` on a row converts; type to narrow
the list, which is the quick way to a page among forty.

A conversion changes nothing on the disk by itself. The file becomes modified, the status line
shows the new answer, and `F2` writes it; `Ctrl+Z` takes it back, one conversion at a time,
like any other edit.

- A file with **mixed line endings** has none of the three rows marked. Choosing one gives
  every line the same ending.
- Only UTF-8 and UTF-16 have a **byte order mark**. Converting to UTF-16 adds it, because
  without one nothing but a guess tells the two byte orders apart; converting to a page that
  has none drops it.
- A **code page** that has no byte for one of the file's characters is refused, and nothing
  changes: `中` does not fit in Windows-1252, and a save that turned it into `?` would be a
  word quietly lost.

In the viewer the same key reads the bytes as another page. Here the text is already read, so
the page you choose is the one it will be written in.

# Colours

A file whose name says what it is — Rust, C, C++, Java, C#, JavaScript, TypeScript, PHP,
Python, Ruby, Go, shell, SQL, HTML, XML, CSS, Markdown, JSON, TOML, YAML, INI, Make,
Dockerfile, Lua, Perl, Pascal and Delphi — is coloured as you read it, and the status line names the language so
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

On Windows the same works with the names Windows uses: `\\.\PhysicalDrive0` for a disk and
`\\.\C:` for a volume, typed in rather than found in a panel — the filesystem does not list
them. The length comes from the device itself, since there is nothing to measure by seeking,
and the reading is done in whole sectors because that is all such a handle accepts. It needs
Administrator rights, and says so if it does not have them.

# Brackets, joining and sorting

`Alt+B` jumps to the bracket that matches the one under the cursor, or the one just before it —
`()`, `[]`, `{}` and `<>`, across lines — and again jumps back; it is mcedit's key. Brackets in
strings and comments count like any others: this is a file manager's editor, not one
language's. `Alt+J` joins the next line to this one, its indent turned into one space.
`Alt+Shift+S` sorts the lines of the block, and nothing outside it; `Alt+Shift+U` takes out
every line that repeats one above it — the first of each stays where it was, so the block does
not have to be sorted first — and says how many went; `Alt+Shift+R` turns the block's lines
upside down; `Alt+Shift+N` puts a number in front of each, from 1, lined up on the widest.
`Alt+Shift+T` takes the spaces and tabs off the end of every line. Each is one `Ctrl+Z`.

**Take trailing blanks off when saving**, in the editor settings, does the last one at every
`F2`. It is off by default, because a save that changes lines nobody touched is a diff nobody
asked for.

# Tabs, whitespace and the odd key

`F9 → Options → Editor settings` has the six that matter. **Tabs** is three choices and not a
box: leave them as they are, turn a newly typed tab into spaces, or expand every tab in the file
when it is opened — the last one changes the file, so it says how many lines it touched and the
next save writes that. **Show spaces and tabs** draws a space as `·` and a tab as `→` and its
padding, which is how a mixed indent becomes visible. **The cursor lands after the text a search
found** is for typing over what was found rather than reading it.

**The cursor may go past the end of a line** is off by default. On, the arrows go into the
columns to the right of the text instead of jumping to the next line, which is what you want to
line up a column of numbers or to mark the same columns down lines of different lengths. Moving
about there writes nothing: the spaces appear at the keystroke that puts a character in one of
those columns, as part of the same edit, so one `Ctrl+Z` takes the character and its padding
away together. `Backspace` and `Del` out there move the cursor and leave the file alone.

`Ctrl+Q` puts the next key in as itself: a control character, or a real tab where tabs are being
expanded. There is no other way to get either into a file.

`Alt+I` and `Alt+U` move the block — or, with nothing selected, the line the cursor is on — one
indent right and left. `Tab` still types a tab, because that is the key one presses while
writing.

`Ctrl+Shift+Ins` adds the block to what is already on the clipboard instead of replacing it, so
a few pieces can be gathered from here and there and pasted in one go. `Ctrl+N` and `Ctrl+E` put
the cursor on the first and the last line *on the screen*, leaving the screen where it is.

# A project's `.editorconfig`

When a file you open has a `.editorconfig` above it, what it says about that file wins over
the settings: spaces or tabs and how wide (`indent_style`, `indent_size`, `tab_width`), and
whether trailing blanks go on saving (`trim_trailing_whitespace`). For a new file, its line
ending and final newline as well (`end_of_line`, `insert_final_newline`); a file that already
exists keeps the ones it has.

# Finishing a word, and characters you cannot type

`Alt+/` finishes the word before the cursor with another word of the file that starts the same
way, the nearest one first; press it again for the next one, and after the last you are back to
what you typed. `Alt+Shift+C` lists the characters a keyboard does not have — accents, dashes
and quotes, currencies, arrows, mathematics, FAR's frame lines, Greek — each with its code:
type `U+20AC` or part of it to narrow the list, `Enter` to insert, `Esc` to go back.

# Taking out the lines that match

`Alt+Shift+D` asks for a regular expression and takes out every line that matches it — in the
block if there is one, in the whole file otherwise. Start the pattern with `!` to keep only the
lines that match and take out the rest: `!ERROR` leaves the errors of a log. One `Ctrl+Z` puts
them all back.
