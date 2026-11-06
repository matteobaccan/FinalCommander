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

`Alt+Del` is the third one: it overwrites the bytes before unlinking the file. Its dialog is
honest about what that is worth on an SSD, on a copy-on-write filesystem and with snapshots
about.

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

# Where the space goes

`Alt+Shift+F3` — *Commands → Disk usage* — lists what is in this folder by size, largest
first, each with a bar against the folder's total and its share of it. Every folder in it is
measured, and the list fills in as each answer arrives: one still being walked shows `…`
rather than counting as empty. `Enter` on a folder goes into it, where the same key asks the
same question one level down; on a file it puts the panel's cursor there. The sizes stay in
the panel afterwards, so sorting it by size agrees with the list.

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

The formats this program does not read itself — `.rar`, `.cab`, `.iso`, `.wim`, `.lha`, `.arj`,
`.msi` — open through a program already on the machine: `7z`, by
default. `Enter` goes in exactly as it does into a zip, the frame names the format, and the
viewer, the search inside archives and `Ctrl+A` on an entry all work. `F3` and `F5` on an
entry run the program again, into a temporary folder that is removed afterwards, and `Esc`
stops it. Such an archive is **read only**, and on purpose: nothing driven that way ever
creates an archive, adds to one or deletes from one, so the pack dialog keeps offering only
the formats this program writes itself. Where the program is not installed, the panel says
which one to install instead of calling the archive broken. Another program, or other switches
for `7z`, go in `fcmd.toml` as an `[[archiver]]` block — see *Formats of your own*.

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

`Ctrl+Alt+G` — *Commands → Git* — in a folder that is in a git repository opens the git menu,
and everything in it is a panel you work in with the function keys. It is the `git` installed
on the machine that answers.

**What changed** turns the panel into the changed files, each row starting with its status as
`git status --short` writes it: the **left letter is what is staged**, the right one what is
changed and not staged (`M ` staged, ` M` not yet, `MM` both, `??` new, `D ` deleted). The
title counts both. They are the real files, and the keys are the ones the bar shows:

| Key | In *what changed* |
| --- | --- |
| `F3` | what changed in the file — in the work tree, or what is staged when only that; a new file is shown as it is |
| `F4` | edit the file |
| `F5` | stage the selected files, or the one under the cursor (`git add`) |
| `F6` | unstage them (`git restore --staged`) |
| `F7` | commit what is staged, after asking for the message |
| `Shift+F7` | the same, with a message the AI drafted from the staged diff (see *Asking an AI*) |
| `F8` | the file's own log, every commit that touched it |
| `F2` | the git menu: the other panels, blame, pull and push |
| `Ctrl+R` | ask git again |

**Branches** is a panel of the local branches with the current one marked `*`. `Enter` on
another switches to it, after asking — the files the two branches disagree on change on disk —
and `F3` is that branch's log. **The log** is a panel of commits, newest first — hash, date,
author, subject — and `Enter` or `F3` on one shows it whole, with its patch, in the viewer; the
log of one file comes from `F8` in *what changed* or from the menu. **Blame** of the file under
the cursor opens in the viewer.

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

`F3` and `F4` are always *ours*: the viewer and the editor in this program, whatever any
block says. A key that sometimes opened something else would be a key nobody could trust.
What a block names for reading and writing is on `Alt+F3` and `Alt+F4` — FAR's keys for the
other viewer and the other editor — and with nothing named there either, those two fall back
to `$PAGER` and to `$VISUAL` or `$EDITOR`, which are already set on a machine that has an
opinion. (On some desktops `Alt+F4` closes the window before the program sees it; that is the
desktop's key, and the Commands menu has the same entry.)

`Ctrl+PgDn` is still the way *into* an archive and `Enter` on one still opens it as a
directory: an association for `*.zip` would stop that, so archives are checked first.

**Open with** — in the Commands menu — lists every block that matches the file under the
cursor, plus the desktop, and runs the one chosen. It is the way to use the `view` command of
a block without rebinding anything.

# The system log

*Commands → System log* opens the latest entries of the system's log in the viewer —
`journalctl` on Linux, `log show` on macOS, the Windows event log's *System* channel — where
`F7` searches it and `End` is the newest line.
