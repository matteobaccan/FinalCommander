# Copying, moving, making, deleting

`F5` copies, `F6` moves or renames, `F7` makes a directory, `F8` deletes. The destination of a
copy or a move is the other panel, and the dialog says so before anything happens. After `F7`
the cursor is on the new folder, ready for `Enter`.

Every one of them is a job: it runs in the background with a progress bar, `Esc` stops it, and
what it has already done stays done. A conflict — a file that is already there — stops and
asks, with "all" and "none" answers so a long copy is not a thousand questions.

The copy dialog has more in it than the destination, and all of it is optional — type a path,
press `Enter`, and nothing below the first line applies:

* **If it is already there**: ask, overwrite, overwrite when the source is newer, overwrite when the size differs
  (the quick way to finish a copy that stopped), skip it,
  give the copy a free name, or add to the end of it.
* **Only files newer than the ones there** is a box beside that choice rather than one of its
  values, and the two are not the same thing: the box holds even when the choice says
  overwrite, and even after you have answered "overwrite all" to a question.
* **If one cannot be copied**: ask (the default), skip it and go on, or stop the copy. Chosen
  before the copy starts, so a long one left alone does not wait all night on its first
  unreadable file; every file skipped is listed in the summary.
* **Only the names that match** takes a mask, so only what matches travels. It applies to
  files and never to directories, and a directory is only created once something under it has
  matched — so `*.rs` out of a source tree gives you the directories that have Rust in them
  and no others. A mask with a `/` in it looks at where a file is rather than at its name:
  `src/*.rs` takes the Rust under a `src` folder, and `*|*/node_modules/*` takes everything
  but what is in `node_modules` — a folder left out that way is not even read. The same holds
  in the synchronisation's filter.
* **Copy what a link points at, not the link** is off, and deliberately: a copy that turned
  links into copies of their targets would duplicate a tree the first time you tried it.
* **Keep the times** and **Keep the permissions** are on.
* **Keep the extended attributes** is on too. These are the named values a filesystem keeps
  beside a file's bytes, and where a desktop puts what it knows about a file: the tags you
  gave it in Dolphin or the Finder, its coloured label, the address it was downloaded from.
  A copy that carried only the bytes would arrive untagged. They are copied for files and
  for folders, and a move to another disk — which is a copy and then a delete — takes them
  along as well. A destination with nowhere to keep them (a FAT stick, a share, a server, an
  archive) gets the file without them and the copy is not an error for it: they are a
  courtesy, as the times are.

  On Linux what is copied is the `user.` attributes, which is where tags and the like are
  written. The `security.`, `system.` and `trusted.` ones — SELinux labels, capabilities,
  ACLs — say what a file is allowed on the system it was on, and stay there, as they do with
  `cp`. On macOS everything is copied, as the Finder does it, and that includes the mark that
  says a file came from the internet: a downloaded file is still a downloaded file after it
  has been copied. Windows has nothing of this kind — NTFS streams are another thing, and are
  not copied.

  What the three *Keep* boxes start as is `operations.preserve` in the configuration.
* A copy that was interrupted goes on where it stopped. A file is written under
  `name.fcmd-part` until it is whole; `Esc` takes that away, but a failure, a crash or a
  killed program leave it, and copying the same file to the same place again continues from
  it — after checking that its last bytes are the source's, so a part left by some other file
  is written over instead. That is also what `F5` in the job list does for a big file that
  failed. Local files only, for now.
* **Read every file back and compare it** is off. On, each file is read again on both sides
  after it is copied and compared by SHA-256: a copy that differs is removed and reported, and
  a move takes the original away only after its copy compared the same. The summary says how
  many were verified, and how many could not be read back here — a server or a bucket — and
  so stand unchecked. It reads everything twice, which is why it is a box.

* A file bigger than the destination's filesystem can hold — over 4 GiB onto a FAT stick —
  is refused before a byte is written, with the limit named, and the rest of the copy goes on.

Whatever you answer is what the next dialog opens on.

`Tab` on the destination line finishes the path, the way a shell does: one match is written
out whole — with its separator when it is a folder, so the next `Tab` goes inside it — and
several are taken as far as they agree, with the count in the status line. Where there is
nothing to add, `Tab` goes to the buttons as it always did. The same works on the folder of an
extraction, the name of an archive and a key file.

The destination line takes more than one destination, separated by `;` — quote a path that has
a `;` in its own name. They run one after the other rather than at once, and only the first of
a *move* is a move: after it the source is gone, so the rest are copies of what landed first.
Stopping one stops the ones behind it.

`Shift+F5` and `Shift+F6` copy and move *into the same directory*, which is how a file is
duplicated or renamed without touching the other panel. `Shift+F8` deletes without the
question, and says so in its own title.

`Alt+F6` makes a link. `Ctrl+G` runs one command over everything selected, with the name of
each file put into the line where you mark it.

*Use the system copy routine* in the system settings is off by default and worth knowing about
if you copy large files. On, the kernel makes the copy instead of this program reading the
bytes and writing them back: on btrfs, XFS and APFS a copy inside one filesystem becomes a
reflink — a second name for the same data, written in a moment however large the file — and
over a network share the server copies the file without the bytes crossing the network twice.
The reason it is off is the other half of the truth: on macOS and Windows the system copies a
whole file in one call, so `Esc` is answered when that call finishes rather than within the
next few kilobytes, and that file's progress arrives in one piece. On Linux the copy is still
made a piece at a time, so neither applies.

# Jobs, in the background and in the queue

A copy, a move, a delete and the other file operations each run as a *job*, with its box on
the screen. `B` in that box sends it to the background: the box goes, the job goes on, and the
top row counts the jobs nobody is watching — `⧗2`, with a `?` when one of them is asking
something. A job in the background that has a question brings it up over the panels at once;
over the viewer or the editor it waits, and the status line says so.

A job for a disk that is busy with another one waits for it. Two copies onto the same disk
fight over it and both end later than one after the other would, so by default there is one
job per device — a copy onto a memory stick does not wait for one onto the network.
`general.job_queue` changes that: `one` runs one job at a time whatever the devices, `off`
starts every job at once.

`Alt+Shift+F5` — *Commands → Jobs* — lists them: running, waiting, paused and finished, with
how far each one got.

- `Enter` brings a running job's box back; on a finished one it shows its errors.
- `Space` pauses a running job at its next step, and lets it go again. On a waiting one it
  holds it back, and the one behind it goes first.
- `Ctrl+↑` and `Ctrl+↓` move a waiting job up and down: the order is the order they start in.
- `Del` takes a waiting job out, stops a running one, and clears a finished one from the list.
- `F5` runs a job that failed again, for what it did not do: what is already at the
  destination is skipped, and a file the job was halfway through is never there, because the
  engine does not leave half a file behind.

Quitting with jobs running asks first and names them: they are part of this program and end
with it, so the choice is to stop them or to stay.

## A copy that survives a crash

A copy or a move that is running keeps a **journal**: a small file, in `fcmd-jobs/` beside the
settings, that says what was asked — the sources, the destination, the options — and which
files are finished. It is removed when the job ends, whether it ended well, with errors or by
`Esc`, and the folder goes with the last one. So a journal that is still there when the
program starts belongs to a job that never reached its end: the program was killed, or the
machine went down under it.

When that happens the status line says so once — *Copies left unfinished by the last session*
— and `Alt+Shift+F6`, *Commands → Unfinished jobs*, lists them, each with how many files it
had done.

- `Enter` goes on with one. It is queued again as it was asked, the files its journal names as
  done are passed over without a question, and the file it was in the middle of continues from
  its `name.fcmd-part`. What a move had already taken away is not looked for again.
- `Del` discards one: the journal goes, and so does the half-written part it names. What was
  copied whole stays where it is.

The journal is written when a file is finished and at most a few times a second, not for every
byte, so a file completed in the very last instant before a crash may not be in it. The job
then meets that file as one that is already there, and does what the copy was told to do about
those — which by default is to ask.

A journal that cannot be read — damaged, or written by a later version — is reported when the
program starts and listed with the others, where `Del` removes it. It is never a reason not to
start. A second copy of the program does not offer the jobs the first one is still running.

Only the job that was running is remembered: one that was still waiting in the queue, and the
later steps of a copy to several places or of a synchronisation, have to be asked for again.

# Renaming many files at once

`Ctrl+Shift+F6` renames what is selected from one pattern — *Files → Rename from a pattern*.
The dialog has a mask for the name and one for the extension, and under them a preview that
follows every key: each file, the name it would get, and what is wrong where something is.

    [N]        the name without its extension     [N1-3]  its characters 1 to 3
    [N3]       its third character                [N3-]   from the third to the end
    [N2,4]     four characters from the second    [E]     the extension, the same ways
    [C]        a counter                          [C10,5,3]  from 10, by 5, three digits
    [d] [t]    the file's date and time           [d:%Y-%m]  in any strftime spelling
    [P]        the name of the folder             [[      a [ of its own

So `[d]_[N]` puts each photo's date in front of its name, and `holiday_[C]` with the counter at
`1,1,3` gives `holiday_001.jpg` onwards. The counter counts in the order the panel shows the
files: sorted by date, it numbers them by date. *Search for* and *Replace with* then work on
the new name, as plain text or, with the box ticked, as a regular expression where the
replacement can say `$1`. *Letters* changes the case last.

Nothing is renamed unless every file can be. A file that would end with no name, with a slash
in it, with the same name as another row, or with the name of a file that is already there and
is staying, is marked `✗` with the reason, first in the list, and `OK` leaves the dialog open
until it is fixed. Names that trade places are fine — `1.txt` and `2.txt` swapped is done
through a temporary name, which is never left behind — and if a rename fails halfway, the ones
already done are put back, so the folder is either all in the new names or as it was.

*Files → Undo the last multi-rename* puts the names back, after a restart too: the last one is
kept in the state file.

## Saying it in words

`Ctrl+Alt+U` — in the panel on what is selected, or inside the dialog above — asks for the
new names **in words**: `dates to ISO, lowercase`, or `the date, then the title`. An AI
service (see *Asking an AI*) turns the words into a pattern for this dialog, and the dialog
opens with it filled in: the masks, the search and its replacement, the letters. From there it
is the dialog you know. The preview shows what every file would be called, every field is
yours to change, a name that collides is marked `✗`, and **nothing is renamed until you press
`OK`**; *Undo the last multi-rename* undoes it like any other.

What goes to the service is said first, in a dialog with *Cancel* lit: your words and up to
twenty of the **names** — never anything from inside the files — and the host they go to.
`F3` there shows it exactly. The model answers with a pattern, not with a list of names, so
the pattern also covers the files it was not shown; when it answers with something that is
not a pattern, the status line says so and nothing happens. If you have moved on to another
screen by the time the answer comes, it does not interrupt: the pattern is what
`Ctrl+Shift+F6` opens with next.

# Checksums

*Checksums…* in the Files menu writes a list of digests for what is selected: SHA-256, SHA-1,
MD5 or CRC32. One list for the lot — named after the directory, or anything else you type — or one
beside each file. The format is the one `sha256sum` and friends write, so the list can be
checked by whoever you send it to with the tool they already have, and the one they send you
can be checked here.

*Check against a list* works on the list under the cursor. Which hash made it is read off the
digests inside it rather than off its name, so a file called `SHA256SUMS`, `sums.txt` or
`release.md5` all work. What does not match is named in the report **and left selected in the
panel**, because the next thing you do with a bad download is fetch it again or delete it, and
both of those work on the selection.

**CRC32 is written as a `.sfv`**, because that is the only shape anything reads it in: the
name first and the eight hex digits after it, `;` for a comment, and `file.sfv` beside a
single file. It is an error check rather than a hash — two files with the same CRC32 are easy
to make on purpose — so it answers *did this arrive intact* and nothing more. A `.sfv` someone
sends you is checked the same way as any other list.

`Esc` stops a long run.

# Splitting a file, and joining it again

*Files → Split file* cuts the file under the cursor into pieces of the size you give — `700M`,
`4G`, `1440K` — in the other panel's folder: `film.mkv.001`, `film.mkv.002` and so on, the last
one shorter, and beside them `film.mkv.crc`, which says what the pieces make. These are Total
Commander's own pieces, so either program joins what the other cut. Or say how many pieces
you want in the row under it: a number there decides, and the pieces are all one size with the
last one shorter — a count no single size can make (ten bytes into six) is refused with the
nearest one that can.

*Files → Combine files* on any of the pieces, or on the `.crc`, puts them back together in the
other panel's folder, from `.001` up for as long as they follow on, and checks the result
against the `.crc`: a missing or damaged piece is reported and leaves no half-made file behind.
Pieces with no `.crc` still join, and the status line says there was nothing to check them
against.

Nothing is written over, either way. `Esc` stops a split or a join and takes back what it had
written.

# Deleting, and the wastebasket

`F8` hands what you chose to the system's own wastebasket — the same one your desktop's file
manager shows, so what goes in can be taken back out from there. A directory goes in whole.
The question it asks says *wastebasket* rather than *delete*, so the two are never confused.

`Shift+Del` deletes outright, past the wastebasket, which is what that key means everywhere
else. *Use the system recycle bin* in the system settings turns the wastebasket off, and then
`F8` is the same thing.

Only this machine has a wastebasket. A file on a server, an entry in an archive and an object
in a bucket are deleted outright, and the dialog says so before you answer. If the wastebasket
refuses something it ought to have taken — on Linux, a file on a disk with no trash directory
of its own — the error says which key deletes it instead.

The wastebasket is also a place to go: the *Wastebasket* row of `Alt+F1` / `Alt+F2`, or
*Commands → Wastebasket*, shows what is in it as a panel — the name, the folder it was in, its
size, and when it was thrown away in the date column, so the usual sort keys order the list.

| Key | In the wastebasket |
| --- | --- |
| `F3`, `Enter` | the item's card: where it was, when it went, how big it is |
| `F5` | put back what is selected, where it came from (`Shift+F5`: the row under the cursor) |
| `F8`, `Del` | delete for good, after asking |
| `F7` | empty the wastebasket, after asking — *Cancel* is the button lit; also *Commands → Empty the wastebasket* |
| `Ctrl+R` | read it again |

Putting back asks nothing unless something already stands where an item was. Then it asks,
one item at a time: **Overwrite** sends the one that is there to the wastebasket and the item
takes its place, so nothing is lost; **Rename** puts the item back beside it as
`name (2).ext`; **Skip** leaves it in the wastebasket; **Cancel** leaves all that remain.

This works where the system lets a program read its wastebasket: Linux and Windows. On macOS
only the Finder can, and the row says so.

`Alt+Del` is the third one: it overwrites the bytes before unlinking the file. Its dialog is
honest about what that is worth on an SSD, on a copy-on-write filesystem and with snapshots
about.

# Taking an operation back

`Ctrl+Alt+Z` — *Files → Undo last operation* — undoes the last copy, move, delete to the
wastebasket or new folder, after a dialog that says what it is about to do and to which files.
Up to twenty are remembered, the newest first, while the program runs:

| What was done | How it is undone |
| --- | --- |
| a copy | what it made goes to the **wastebasket** — nothing is deleted outright; a file the copy wrote over cannot be given back, so it is left alone |
| a move or a rename with `F6` | each name goes back where it was, through the queue, asking as any move does if something is in the way |
| a delete to the wastebasket | the names come back from it, where the system lets a program do that (Linux and Windows; on macOS the Finder's *Put Back* does it) |
| a new folder (`F7`) | it is removed, if it is still empty |

Only what ended well is remembered: a job that was stopped or that ended with errors cannot be
taken back from here. Neither can anything on a server, in a bucket or inside an archive, nor a
real delete (`Shift+Del`), which is what that key is for. The multi-rename has its own undo,
*Files → Undo multi-rename*, kept across a restart.

# Keeping two folders the same

`Ctrl+Shift+Y` synchronises the two panels' directories. It happens in three steps, and the
middle one is the point:

1. A dialog asks how to decide — one way (the left panel is the source) or both ways with the
   newer file winning, whether to go into the subdirectories, whether to offer to take out
   what the other side does not have, a mask, and whether to compare the bytes rather than the
   size and the date.
2. A **table** of every pair, with an arrow in the middle saying which way that file is about
   to go. `Space` walks through the answers a row allows and steps down, `←` and `→` say which
   way, `=` leaves it alone. Rows that will do nothing are dim; a clash — the same date and a
   different size, or a file against a folder — is marked and does nothing until you choose.
3. `F2` does it. **Nothing** is written before that, so the table is also the dry run.

Two seconds of difference counts as the same file, because that is what a trip through a
memory stick or a zip does to a timestamp, and treating it as a change would copy everything
every time. If you need to catch a change that kept the same size and the same date, tick
*Compare the bytes* — it reads both sides.

The copies overwrite without asking: the table was the question, and you answered it row by
row. A row that takes a file out puts it in the wastebasket, the same as `F8` does.

Both folders have to be on this machine for now.

## A synchronisation you run again

The dialog has two more rows, both empty unless you fill them in.

**Save this pair as** takes a name. With one in it, `OK` keeps the two folders and everything
the dialog says — which way, how deep, the mask, the report — under that name, in `fcmd.toml`,
and goes on to the table as usual. The same name again replaces what was saved under it.

`Ctrl+Alt+Y` — *Commands → Saved synchronisations*, and the *Saved…* button of the dialog —
lists what has been saved, each with its two folders and an arrow for one way or both.

- `Enter` runs one: its own two folders, wherever the panels are, walked now. The table still
  comes first and `F2` still starts it — what the folders hold today is not what they held
  when the pair was saved, and taking files out is not something to do unseen.
- `F4` opens its dialog, filled in, to change it and save it again.
- `Del` forgets it.

**Write a report of the run to** takes a file. When the last job of the synchronisation ends,
a short text is written there and the status line says where it is: what was copied from left
to right and from right to left, what was taken out on each side, the files that differ and
were left alone, and what failed, each with its path. If the run was stopped or a step failed,
the report is written at that point and says which rows were never reached — and, for a step
stopped halfway, that its rows may or may not have been done, rather than guessing. A name
with no folder in front of it is a file in the active panel's folder; the folder has to exist.
Every run writes the file anew.

# Where the space goes

`Alt+Shift+F3` — *Commands → Disk usage* — lists what is in this folder by size, largest
first, each with a bar against the folder's total and its share of it. Every folder in it is
measured, and the list fills in as each answer arrives: one still being walked shows `…`
rather than counting as empty. `Enter` on a folder goes into it, where the same key asks the
same question one level down; on a file it puts the panel's cursor there. The sizes stay in
the panel afterwards, so sorting it by size agrees with the list.

`Tab` in that list asks about the whole tree under the folder at once and draws it as a map:
every row of the folder is a rectangle as large as its share of the space, in the colour its
name has in the panel — an archive is the colour of an archive, a folder of a folder — and a
folder's rectangle shows what the folder is made of, one level down. The tree is walked once,
in the background; until the answer is there the screen says how many files it has seen and
`Esc` stops it. Then:

| Key | What it does |
| --- | --- |
| the arrows | from one rectangle to the one beside it |
| `Enter` | into the folder under the cursor; on a file, closes the map with the panel's cursor on that file |
| `Backspace` | back to the folder above, with the cursor on the one just left |
| `Tab` | the same folder as a list, largest first, with bars and shares — and back to the map |
| `G` | closes the map with the panel in the folder under the cursor, or on the file |
| `Esc` | closes the map |

What is too small to have a cell of its own is one rectangle that says how many rows it stands
for; `Enter` on it opens the list, where they can be told apart. A folder of thousands of
files keeps its 256 largest by name and counts the rest as one row, so nothing is missing
from a total. Going in and out of folders never touches the disk again: the sizes were added
up once.

# Files that are the same

*Commands → Find duplicate files* looks under the folder the panel is in — or under what is
selected, folders and files alike — for files with the same content, whatever they are called.
The dialog asks whether to look in subfolders, how small a file still counts (in the filters'
size words: `100K`, `5M`; empty files are never compared), and whether to select the copies.

The answer is a panel of groups, the group that wastes the most space first. Each row is the
group's number and the file's path, so the groups stay together however the panel is sorted;
the oldest file of each group comes first. With *Select every copy but the oldest* on, the
rest are selected, and `F8` sends them to the wastebasket; `F3`, `F4`, `F5` and the rest work
on the real files as in any list of results. The search compares sizes first, then the first
64 KiB, then the whole file by SHA-256, so it reads little of what is not a copy; it runs in
the background, its progress on the status line.

# Finding things

`Alt+F7` searches for files: a mask, a size, text inside them, and where to look. The results
are a list you can view (`F3`), edit (`F4`) or jump to (`Enter`) — and `F5` turns the whole
result into a panel, so anything that works on a directory works on a search. When you
searched for text, `F3` and `F4` open the file where it was found: the viewer at that place,
the editor on that line (or on that byte, for a file that opens in hex).

**Inside archives** is a box, and it is off until you tick it: ticked, the search steps into
every `.zip`, `.7z` and `.tar.*` it meets and looks at what is in them — by name, and by text
where you gave one. A result then reads `src.zip!/doc/readme.md`, which `Enter`, `F3` and `F5`
treat like any other file. It is slower, which is why it is a box: opening every container of
a whole drive is a different search. A file inside an archive bigger than 64 MB is not read
for text — it is counted as a problem instead, because an archive has to be unpacked to be
read and that one would not fit anywhere sensible.

The size row is what finds the files worth deleting when a disk fills up: `100M` means "from a
hundred megabytes up", `1M-1G` a range, `-4K` everything smaller than four kilobytes. Leave it
empty for any size. `K`, `M`, `G` and `T` are powers of 1024, as everywhere else in the
program.

The row under it is how old: *Changed, days ago*. `7` is the last week, `0` today, `>=30` a
month old and older, `7-30` between the two — counted from the last time the file was
written, which is the date the panels show.

**Saying it in words.** With a language model set up (see *Asking an AI*), `Ctrl+Alt+Q` — in
the panels, or inside this dialog — asks what you are looking for: *pdfs over 10 MB changed
last week*. A dialog first says what goes and where: your words, and nothing of the panels —
no name, no path. The model's answer fills the rows — here `*.pdf`, `10M-` and `7` — and
**nothing is searched**: read them, change what is wrong, and press `OK` yourself. Where to
look stays as you had it; the model is not asked. What you asked for that the rows cannot
say — *taken in Rome* — is said on the status line rather than guessed at, so you know what
the search will not do. `Esc` at the question goes back to the dialog as it was.

**When you only half remember the name**, tick *Approximate name*: the mask row then takes
letters instead of a mask and finds every name that has them in that order — `mnrs` finds
`main.rs`, and `test app` finds `app_tests.rs`, because each word only has to be in there
somewhere. The list comes best match first: letters side by side before letters scattered,
the start of a word before the middle of one, a short name before a long one.

**The metadata row** finds a file by what it says about itself — exactly what `Shift+F11` shows
you in the viewer: the camera and the date of a photograph, the artist and the album of a
song, the author and the title of a PDF or an office document, the version of a program.
Write `field=value`: `artist=bowie`, `model=canon`, `author=rossi`. Both sides are matched
anywhere and in any case. Leave the field out to look in every row (`=bowie`, or just
`bowie`), leave the value out to ask only whether the file has such a row (`gps=`), and put
`;` between several that must all hold (`artist=bowie; year=1977`). `format=` asks what
kind of file it is: `format=jpeg`. It combines with the mask, the size and the text like
everything else in the dialog.

**A whole disk in a moment.** Tick *Ask the system's index instead of walking* and the
search asks the list of files your system already keeps — `plocate` or `locate` on Linux,
Spotlight (`mdfind`) on macOS, Everything (`es`) on Windows — instead of reading every
folder. Every file the index names is still looked at and put through the rest of the
dialog, so the mask, the size, the text and the metadata mean what they always mean; the
frame of the results says who answered (`· plocate`). Two things to know. An index is as
fresh as its last update, so **a file made a minute ago may not be found** — untick the box
for those. And when there is no index, or it cannot answer (no database yet, a folder on a
server, *Inside archives* ticked), the search simply walks as usual. `find.index` in the
configuration names a command of your own to ask instead, or `off` to take the box away;
the programs are listed, found or not, in `Alt+F1` → *External programs*.

**A search you run every week can be kept.** `F2` in the dialog asks for a name and keeps
the whole of it — every row, and the folder it was about to look in. `F9` in the dialog
lists the ones you kept, and `Enter` there fills the dialog in again. Keeping a search
under a name that is taken replaces it, which is how you change one.

**A kept search is also a folder.** `Alt+Shift+F7` — or *Commands → Saved searches*, or
the *Saved searches* row of `Alt+F1`/`Alt+F2` once you have kept one — opens the same list
from the panels, and `Enter` there makes the panel the files that search finds: a
temporary panel like the one `F5` gives you, named after the search. It looks in the folder
the search was kept for, wherever you open it from, and `Ctrl+R` runs it again — so it is
a folder that always holds "the logs over a hundred megabytes" or "the sources that still
say TODO" as of now. In that list `F4` puts the search into the dialog instead, and `Del`
forgets it.

Replacing across files is the same dialog with a replacement, and it counts the files and the
matches before it writes anything. The count comes from the same engine that then does the
work, so the two cannot disagree.

# Archives

`Shift+F1` packs what is selected — and can delete the originals once the archive is really
written, which is a box in the dialog and never the default. `Shift+F2` unpacks the archive
under the cursor into a directory of its own. `Shift+F3` is the archive menu: test it (every
entry is decompressed and checked, nothing is written), extract it here, extract it
elsewhere, delete it, or add files to a new one.

`Enter` on an archive opens it as if it were a directory — zip, 7z,
tar, and tar with gzip, bzip2, xz or zstd — and the path says where you are:

    /home/me/sources.tar.gz!/src/main.rs

Everything that reads a file works in there: the viewer, the hex editor, the search.

A good many things are zips without saying so, and `Enter` opens those too: `.jar`, `.war`,
`.ear`, `.aar`, `.apk`, `.aab`, `.ipa`, `.whl`, `.nupkg`, `.vsix`, `.xpi` — the packages one
opens to see what is inside. A `.docx` or an `.epub` is a zip as well and is deliberately not
on that list: there the program that reads such a file is the more useful answer, and `Enter`
hands it over to that. If you do want to see inside one, `Shift+F2` unpacks it anyway, and a
renamed archive opens by its bytes once you are already inside a container.

`Enter` also opens the Unix archive — `.a` and `.ar` — and a Debian package, `.deb`,
`.udeb` or `.ddeb`, which *is* one: inside it you find `debian-binary`, a `control.tar.*` and a
`data.tar.*`, and `Enter` on either tar goes on into it, so the files a package would install
are two keystrokes away. These are read here and not written: `ar` and `dpkg-deb` build them,
and a package written without its control file would look like a package without being one. The
pack dialog therefore does not offer the format, and a name like `libfoo.a` typed into it is
refused with a reason rather than half-written.

A file that is only compressed — `notes.txt.gz`, `dump.sql.bz2`, `core.xz`, `blob.zst` —
opens as an archive of one entry: `Enter` goes in and shows the file it holds, named as it was
before it was compressed, and `F3` reads it. gzip keeps that name in its header and it is used
when it is there; the other three keep nothing of the kind, so the name is the file's own with
the suffix taken off. A `.tar.gz` is still a directory tree, and so is a tarball somebody
renamed `backup.gz`: what is inside is looked at before deciding.

`Shift+F1` makes one, from one file — choose `gz`, `bz2`, `xz` or `zst`, or type a name that
ends in one — and the original can go once it is written, which is what `gzip` does. A folder
or a second file is refused while the dialog is still open, because the format has room for
one; the dialog says which tar would take them. Inside one, `F4` on the file and a save write
it again; nothing else can change, since there is no second name to give and no other entry.
Every level of a `zst` is the fastest, which is the only one the compressor here has.

`Enter` also opens a `.cpio` — an initramfs is one — and an `.rpm`, as the files the package
would install. Both are read here, so an rpm opens on a machine with nothing else installed;
neither is written, for the reason a `.deb` is not.

`Enter` opens a Windows cabinet, `.cab`, too, and reads it here: the ones packed with MSZIP
— which is most of them, and all the old ones — and the ones not packed at all, with every
block checked against its checksum when the cabinet is tested. Two things are beyond it. A
cabinet packed with **LZX** or **Quantum**, as Windows updates and drivers are, is handed
whole to `7z` when that is installed; when it is not, the cabinet is still listed — the
list of files is not compressed — and opening a file out of it says which compressor it
wants and which program to install. And a file that runs from one cabinet of a set into the
next is listed and refused by name; the files wholly inside each cabinet are read. A cabinet
is never written.

The formats this program does not read itself — `.rar`, `.iso`, `.wim`, `.lha`, `.arj`,
`.msi` — open through a program already on the machine: `7z`, by
default. `Enter` goes in exactly as it does into a zip, the frame names the format, and the
viewer, the search inside archives and `Ctrl+A` on an entry all work. `F3` on an entry runs
the program again, into a temporary folder that is removed afterwards, and `Esc` stops it.
`F5` on a selection and `Shift+F2` on the whole archive run it once, for everything asked for
(see *Unpacking* below). Such an archive is **read only**, and on purpose: nothing driven that way ever
creates an archive, adds to one or deletes from one, so the pack dialog keeps offering only
the formats this program writes itself. Where the program is not installed, the panel says
which one to install instead of calling the archive broken. A `.rar` wants a `7z` that was
built with RAR in it, which some distributions leave out; `unrar` is not used in its place,
because the free one most of them ship lists differently and tests nothing. Another program, or other switches
for `7z`, go in `fcmd.toml` as an `[[archiver]]` block — see *Formats of your own*.

## Unpacking

Getting files out of an archive — `F5` from inside one, `Shift+F2` on one, *extract here* in
the archive menu — is a copy like any other: the same box, with the files and the bytes
counted as they are written, the same `Esc`, and the same way into the background and the
job queue. Each file is written beside where it is going under a name ending in
`.fcmd-part` and takes its own name only when it is whole, so stopping half way, a damaged
entry or a full disk never leave half a file that looks like a finished one.

An archive read through a program is unpacked by that program in **one run** for everything
you asked for, rather than one run for each file — which in a solid `.rar` used to mean
unpacking the same data again for every file after the first. The program writes into a
hidden folder beside the destination (`.fcmd-unpack-…`), each file moves into place as soon
as the program has finished it, and the bar counts the bytes as they land. `Esc` stops the
program and removes the folder with whatever was half written in it.

## Testing an archive

*Test archive* — in the Files menu and in the archive menu, `Shift+F3` — reads every entry
and writes nothing: the bytes are unpacked, checked and thrown away, in the same box as any
job, with the entries and the bytes counted and `Esc` to stop. What they are checked with is
whatever the format carries: the CRC of every entry in a zip and a 7z; gzip's CRC and
length, bzip2's block checksums, xz's own check and zstd's checksum for a compressed file
and for every entry of a tarball packed with one; the checksum of each header in a tar; and
for the rest, that every entry is as long as it says.

An archive that is whole ends in one line saying so, with what was read. One that is not
ends in a list of **every** entry that did not come out, each with its reason — type to
narrow it, `Enter` goes to the entry inside the archive. A damaged header in a tarball stops
the walk there, since nothing after it can be found, and that is a line of its own.

An archive read through a program is tested by that program — `7z t` — in one run; the
lines it complains in come back against the entries they name. The box has no bar to move
then, only the name and `Esc`: the program says nothing until it has finished.

An archive inside an archive opens too, and so does one on a server or in a bucket. Neither is
a file this program can seek in, so it is copied into the temporary directory first, once, and
read from there — a pause the first time and nothing afterwards, with the path still saying
where the file really is:

    sftp://srv/backups/site.zip!/inner.tar.gz!/etc/nginx.conf

`[arc] max_depth` says how many containers deep that may go and `[arc] max_spill_mb` how large
a container may be before the copy is refused instead of made; both say which limit stopped
them. One thing such an archive cannot do is be written to: the copy goes when the program
does, so a change to it would be a change nobody would ever see again.

Writing works too, and it works the only way these formats allow. None of them can be edited
in place — a `.tar.gz` is one compressed stream from end to end, and a 7z usually packs
several files into one block — so a change means writing
the archive again: the old one is streamed into a new file beside it with the change applied,
the new one is read back and checked, and only then does it replace the original. An
interrupted change therefore leaves the archive exactly as it was and a `.part` file to
delete. Entries nobody touched are copied across as the compressed bytes they already are, so
taking one file out of a hundred-megabyte zip is a copy and not a recompression. A 7z is the
exception: its files share a compressed block, so changing one of them decodes and packs the
rest again, which is the format and not us.

A 7z this program *writes* puts each file in a block of its own, unless you tick **Solid
blocks** in the `Shift+F1` dialog. One block per file is bigger than what 7-Zip writes by
default, and it is the right default here: in a file manager the common act is reading one
file out of an archive, and a solid archive makes that mean decoding everything in front of
it. Ticked, the files share a stream and the archive is smaller; a block is closed at 64 files
or 128 MB, so reading one file never decodes a whole tree. The box is remembered for the next
dialog, and only a 7z does anything with it — a tar is solid whatever anybody asks and a zip
cannot be.

An encrypted 7z — even one whose header is encrypted, so not even the names show — asks for
its password. Type it and the archive opens: the password is kept for as long as the program
runs, so the listing and every file you then open out of that archive ask once, and it is
never written anywhere. A wrong one asks again. A zip encrypted with AES says so instead of
asking: reading those wants a part of the zip library this build leaves out.

What that gives you, from inside an archive:

* `F4` opens an entry in the editor. Saving puts it back in.
* `F6` renames an entry, or moves it to another directory inside the same archive. The dialog
  offers the name it has, because a target in the other panel would mean taking it out of the
  archive, and that is `F5`.
* `F8` takes entries out, the selection included. A directory takes its tree with it.
* `F5` and `F6` *into* an archive — with the other panel inside one — put files in. One file
  at a time, as many as are selected; a whole directory is `Shift+F1`, which walks it in a
  job. `F6` deletes the originals once the archive is written, and only then.

* `Ctrl+A` on an entry sets what an archive keeps about it: the permissions and the
  modification time. Both live in the entry's header, so changing either writes the archive
  again — the dim line in the dialog says so — and everything else is kept as it was. The
  boxes open from what the listing knows, so a dialog you do not touch writes nothing.

An archive inside an archive takes all of that too. It is read from a copy, because every
reader here reads a file, so writing into it means writing the copy and then putting the copy
back into the archive it came out of — as far out as the file on the disk, which is the one
that changes. Three zips deep is three archives written, and the job says when it is done.

What is not there: `F7` does not make a directory inside an archive — nothing stores an empty
one worth keeping. Nor can an archive on a *server* be written into: that one is read from a
copy as well, and nothing would put the copy back.

# SQLite databases

`Enter` on an SQLite database — `.sqlite`, `.sqlite3`, `.db3`, `.s3db`, or a `.db` that is one —
goes inside it as it goes inside an archive. Each table and view is a file named after it,
`people.csv`, whose bytes are its rows as CSV, and `schema.sql` holds the statements that made
them. So `F3` shows a table in the viewer's table mode, with its sort and filter; `F5` writes
it out as a `.csv`; `Alt+F7` looks for a text in every table at once. Nothing writes to the
database: it is opened read-only.

A query of your own is `sql:` on the command line, with the cursor on the database or the panel
inside it: `sql: SELECT name, count(*) FROM orders GROUP BY name`. The rows open in the viewer
as a table. A statement that would change the database — `DELETE`, `UPDATE`, `DROP` — is
refused by SQLite itself, because the database is opened read-only.

# A JSON file as a folder

`Enter` on a `.json` file goes inside it: every object and array is a folder — an array's items
are `[0]`, `[1]`, … — and every other value is a file holding it, a string as plain text. So a
deep configuration or an API's answer can be walked with the arrows and `Enter`, a value read
with `F3`, copied out with `F5`, found with `Alt+F7`. `F3` and `F4` on the file itself still
view and edit the whole document; nothing inside can be changed from the panel.

# Git

`Alt+F1` / `Alt+F2` have a **Git** row, below the other panels, when the panel's folder is in a
git repository — and only then. Choosing it opens the repository's log on that panel.

`Ctrl+Alt+G` — *Commands → Git* — in a folder that is in a git repository opens the git menu,
and everything in it is a panel you work in with the function keys. It is the `git` installed
on the machine that answers.

**What changed** turns the panel into the changed files, each row saying in words which part
it is in and what happened to it — `staged  modified  src/app.rs`, `not staged  deleted
old.txt`, `untracked  notes.md` — in that order: conflicts first, then what the next commit
takes, then what it does not, then the files git does not know. A file staged and then changed
again is a row in each part, each with its own diff. The title counts both. *Show what changed
as a tree* in the `F2` menu groups the rows by folder, a folder heading the files under it in
each part (and back to the list the same way); `F5`, `F6` and `F3` on a folder act on all of
it. They are the real files, and the keys are the ones the bar shows:

| Key | In *what changed* |
| --- | --- |
| `F3` | the two versions side by side, in the internal diff: the index and the file on a row that is not staged, the last commit and the index on one that is; a file git does not know is shown as it is. There `S` stages the hunk at the top of the screen and nothing else — or, in the diff of what is staged, takes it out of the index — and the diff comes back on the hunks that are left |
| `Ctrl+Q` | the quick view beside it shows the diff of the row, in colour, and follows the cursor |
| `F4` | edit the file |
| `F5` | stage the selected files, or the one under the cursor (`git add`) |
| `F6` | unstage them (`git restore --staged`) |
| `F7` | commit what is staged, after asking for the message |
| `Shift+F7` | the same, with a message the AI drafted from the staged diff (see *Asking an AI*) |
| `F8` | throw the changes away, after asking (Cancel is the button lit): a row not staged goes back to what is staged (`git restore`), a staged row back to the last commit, a staged new file out of the index but kept on disk, and a file git does not know to the wastebasket, where `Ctrl+Alt+Z` finds it again. What git puts back cannot be undone |
| `F2` | the git menu: the other panels, blame, pull and push |
| `Ctrl+R` | ask git again |

The panel asks git again by itself whenever a copy, a move or a delete ends — a file copied
into the repository from the other panel shows up as untracked at once.

**Branches** is a panel of the local branches with the current one marked `*`. `Enter` on
another switches to it, after asking — the files the two branches disagree on change on disk —
and `F3` is that branch's log. **The log** is a panel of commits, newest first — hash, date,
author, subject — and `Enter` or `F3` on one shows it whole, with its patch, in the viewer; the
log of one file comes from the menu (`F2`) in *what changed*. **Blame** of the file under
the cursor opens in the viewer.

From the log and the branches panels the rest of git is a function key away:

| Key | In the log and the branches |
| --- | --- |
| `F3` | the commit with its patch (log), or the branch's log (branches) |
| `F4` | what changed |
| `F5` | pull |
| `F6` | push |
| `F7` | commit what is staged, after asking for the message |
| `F8` | the branches (from the log), or the log (from the branches) |

`Ctrl+Q` beside either shows the same thing as `F3`, for the row under the cursor, as you move.

**Pull** and **push** run in the background and the status line says what git said at the
end. They never stop to ask for a password on this screen: one that needs it fails and says so
— run it once in the terminal (`Ctrl+Alt+E`), or give git a credential helper or an ssh agent,
and from then on it works from here. Anything git refuses — a switch over uncommitted changes,
a commit with nothing staged — is git's own message on the status line.

# Mail files

`Enter` on a mailbox — an `.mbox` — lists its messages as files, numbered in the order they
arrived and named after their subject. `Enter` on one of them, or on an `.eml` file, opens the
message itself: `headers.txt` with who sent it, to whom and when, `message.txt` and
`message.html` with the text, and each attachment under its own name, already decoded — `F3`
reads it, `F5` saves it out. Nothing is changed in the mailbox.

# Writing a text file again

Text arrives from somewhere else: a zip off a Windows machine, a dump out of a database, a
file somebody mailed. *Files → Write again as…* takes the selection and writes each file with
the code page and the line endings you choose. What they are *now* is worked out per file, from
the byte-order mark where there is one and from what the bytes look like where there is not —
a `.txt` of nothing but ASCII is every code page at once, so the first row of the dialog is
there to be told when the guess is wrong.

Three things are refused rather than done badly: a file that does not look like text (nothing
good comes of "converting" a disk image), a file over 64 MB (decoding reads the whole of it),
and a target page that cannot hold every character in the file — said by name, instead of
writing a file full of question marks. A file that would come out exactly as it went in is not
written at all: that would change its modification time and tell every backup that it had
changed. What is written goes beside the original and is renamed over it, so an interruption
leaves the original whole, and a script that was executable stays executable.

# What a file is

`Ctrl+A` opens the attributes dialog: what the file *is*, rather than what is in it.

On Linux and macOS that is the mode, as twelve boxes — owner, group and world, each with read,
write and run, plus setuid, setgid and sticky — and the line above them says what the file is
now, both as `rw-r--r--` and as `0644`. Above that is *Owner*, as `user:group`, and on Linux
and macOS it is a row to type a different answer in: either half may be a name or a number,
and a half left empty leaves that half alone — `:staff` changes the group and nothing else.
Giving a file to somebody else needs a privilege an ordinary account has not got, and a group
can only be set to one you are in; what the system refuses is reported by name, and the dialog
does not ask you for a password over it — `sudo` is one line away in the command line under the
panels, where you can see what it is for. On Windows the row is a line to read: ownership there
is an entry in a security descriptor, and the shell's own property sheet is the place that asks
for the privilege properly. On Windows it is
read-only, hidden, system, archive, temporary, offline and whether the indexer may look at it
— and a dim line says what else the file is, where it is compressed, encrypted, sparse or a
reparse point: those are set by doing the thing, not by ticking a box. Read-only is on both,
because on Unix it is the write bits.

Over several files the boxes they do not agree about open as `[?]`, and pressing `OK` leaves
those bits as each file has them: a dialog over a hundred files does not make them agree about
something you were not asked. `Space` walks such a box through *on*, *off* and back to `[?]`,
so one pressed by mistake can be put back to changing nothing. The boxes they *do* agree about
are ordinary ticks, and answer for all of them.

Below the boxes are the times — *Changed*, *Last read* and *Created* — as
`2026-10-24 17:42:05`. A time left blank is left alone; a time typed in is set. *Times in UTC*
reads and writes them in that clock instead of this machine's, which is how a file here and
its copy on a build server are compared. The three buttons under them fill the rows in without
answering the dialog: *Original* puts back what the file carries, *Current* writes this moment,
*Blank* empties them. `Ctrl+Shift+C` keeps the three as they stand and `Ctrl+Shift+V` puts
them into the dialog of another file, to give it the first one's times. On Unix a fourth time is shown and never offered — the moment the file's
own record changed, which no system has a call to set — and the creation time is shown
everywhere and written only on Windows, the one platform with a call for it. With a folder among what is selected there is one more
box: *and everything inside the folders*, which takes the whole change down the tree. The
directories themselves are changed after what is in them, so taking the write bit off a folder
cannot lock the program out of its own contents halfway through.

It reports like a copy — a progress box, a count, `Esc` — and a file that refuses is listed
with the reason rather than passed over quietly. One thing is left alone rather than reported:
a folder's *own* times on Windows, which cannot be set without opening the folder as a handle.
The files inside it get the times you asked for. A file on a server gets its mode and its
modification time, which is what SFTP carries. An entry inside an archive has the archive's
permissions and an object in a bucket has the policy of its bucket, so neither has anything to
set, and both say so instead of pretending.

Something typed in that is not an answer — a date that is not a date, a name no account or
group here goes by — leaves the dialog open with what you typed still in it, and says which
word it could not make sense of.

What is not there yet: Windows' advanced bits, and the owner there.

## Comparing two folders

`F9 → Commands` has four of them, and they answer different questions.

**Compare folders** is FAR's own, and it is instant: it matches names, compares the size and
the time, and leaves selected in each panel what the other side does not have or has an older
copy of. What stays selected is what a copy would have to carry.

**Compare by content** reads the files. Every pair with the same name is opened on both sides
and the bytes are compared — a pair whose sizes already differ is settled without opening
anything, and the rest are read until the first byte that differs, which for two files that
are not the same is usually in the first block. It is the question to ask about two trees that
were built from the same source, or copied by something that did not keep the timestamps,
where the date proves nothing.

**Compare with options** is the same work with the question written out: a dialog of what
counts as a difference, which is the only honest way to compare two directories, because no
two people mean the same thing by *the same*. Whether the names are case-sensitive. Whether
the size counts, whether the time counts and to what accuracy — two seconds is what FAT
keeps, and a file copied to a stick and back is two seconds out through no fault of anybody's.
Whether to read the bytes, and whether to read them only when the time differs, which is the
fast way to check a backup. How much to read: a first fragment of so many kilobytes settles a
pair of large files in a fraction of the time, and says that is what it did. Whether two files
that differ only in their line endings count as different — `CRLF` against `LF` is a
difference in the bytes and usually not a difference in the file. Whether to look only at
what is selected, whether to stop at the first difference, and whether to mark only the newer
side or both.

**Into the subdirectories**, which the same dialog turns on, compares two whole trees: every
file is paired by its path below the two roots, to whatever depth you allow, and the answer
comes back as a panel of its own with each row marked for why it is there — it differs, it is
only on the left, it is only on the right, or it is the same but for the line endings. A
difference found six directories down cannot be a selection in a panel showing one directory,
which is why it is a panel and not a selection.

**Find duplicates** asks the other question: not whether two directories match, but which
files inside one tree are the same file under different names. It compares what is in them —
a size and then a hash — so two names that have nothing in common still land together, and
the answer is a panel of the groups it found.

Both panels have to be on a local filesystem for any of it, and anything with no counterpart
on the other side is a difference. A long comparison says how far it has got in the status
line, and `Esc` on an empty command line stops it.

# What opens a file

`Enter` on a file that is not an archive and not a program runs what an association says to
run. An association is a `[[assoc]]` block in `fcmd.toml`, and it is three commands and a
mask:

    [[assoc]]
    match = "*.md"
    name  = "Markdown"
    open  = "glow -p {file}"
    view  = "bat --style=plain {file}"
    edit  = "hx {file}"

The mask is either a whole name (`Makefile`) or `*.` and an extension (`*.md`), matched
without regard to case; the first block that matches wins, so the specific ones go first.
`{file}` is the path **in quotes** and `{file.raw}` is the bare one — the same substitutions
the user menu and `Ctrl+G` take, so `{name}`, `{dir}`, `{other.dir}`, `{list}` and
`{ask:…}` all work here as well.

With no block to go on, `Enter` hands the file to the desktop — `xdg-open` on Linux, `open`
on macOS, `start` on Windows — which is what `general.desktop_open` turns off for a machine
where that is the wrong thing to do. With that off too, it says there is nothing to open the
file with rather than guessing.

`F3` and `F4` are *ours*: the viewer and the editor in this program, whatever any block
says. A key that sometimes opened something else would be a key nobody could trust.
What a block names for reading and writing is on `Alt+F3` and `Alt+F4` — FAR's keys for the
other viewer and the other editor — and with nothing named there either, those two fall back
to `$PAGER` and to `$VISUAL` or `$EDITOR`, which are already set on a machine that has an
opinion. (On some desktops `Alt+F4` closes the window before the program sees it; that is the
desktop's key, and the Commands menu has the same entry.)

## An editor and a viewer of your own

Between the block and the environment there is a setting: one program for every file.

```toml
[editor]
external = "vim +{line} {file}"    # or "code -w {file}", "notepad++ {file}"

[viewer]
external = "less {file}"
```

`{file}` is the file, quoted when its name needs it, and `{line}` the line to open it at
(1 from a panel); a setting that is only a program's name gets the file after it. `Alt+F4`
and `Alt+F3` then run these — a block for that kind of file still comes first, `$VISUAL`,
`$EDITOR` and `$PAGER` after. The program has the terminal to itself while it runs, as a
full-screen program started from the command line has, and the panels come back, read again,
when it ends. With `code`, write `-w`, or it returns before you have typed a letter.

If the external one is the one you want every day, turn on *Use the external editor for F4*
(`editor.use_external`) or *Use the external viewer for F3* (`viewer.use_external`): the two
keys change places, as in FAR, so `F4` is yours and `Alt+F4` is ours. A file on a server or
inside an archive still opens in our editor, which is the one that knows how to send it
back, and `F3` on a folder still measures it.

Both are in *Options → Editor settings* and *Viewer settings*, in *All settings*, and on the
command line: `fcmd --set editor.external="code -w {file}" --set editor.use_external=true`.

`Ctrl+PgDn` is still the way *into* an archive and `Enter` on one still opens it as a
directory: an association for `*.zip` would stop that, so archives are checked first.

**Open with** — in the Commands menu — lists every block that matches the file under the
cursor, plus the desktop, and runs the one chosen. It is the way to use the `view` command of
a block without rebinding anything.

# Printing

`Alt+F5` prints the files that are marked, or the one under the cursor, after a question that
shows the command it is about to run. The same key prints the file in the viewer, and in the
editor the block — or, with no block, the whole text as it is on the screen, saved or not.
Nothing leaves the screen you are on: the line at the bottom says when the printer has it, or
why it does not.

What runs is `[print] command` in the configuration. Left empty it is the system's own: `lp`,
or `lpr` where there is no `lp`, and Notepad's `/p` on Windows. `{file}` in it stands for one
file, and the command is then run once for each; a command without it gets all the files after
it:

```toml
[print]
command = "lpr -P office"
# command = "pr -h {file} {file} | lp"
```

Only a file on this machine can be printed: copy one off a server first.

# The system log

*Commands → System log* opens the latest entries of the system's log in the viewer —
`journalctl` on Linux, `log show` on macOS, the Windows event log's *System* channel — where
`F7` searches it and `End` is the newest line.
