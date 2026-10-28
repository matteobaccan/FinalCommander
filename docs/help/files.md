# Copying, moving, making, deleting

`F5` copies, `F6` moves or renames, `F7` makes a directory, `F8` deletes. The destination of a
copy or a move is the other panel, and the dialog says so before anything happens.

Every one of them is a job: it runs in the background with a progress bar, `Esc` stops it, and
what it has already done stays done. A conflict — a file that is already there — stops and
asks, with "all" and "none" answers so a long copy is not a thousand questions.

The copy dialog has more in it than the destination, and all of it is optional — type a path,
press `Enter`, and nothing below the first line applies:

* **If it is already there**: ask, overwrite, overwrite when the source is newer, skip it,
  give the copy a free name, or add to the end of it.
* **Only files newer than the ones there** is a box beside that choice rather than one of its
  values, and the two are not the same thing: the box holds even when the choice says
  overwrite, and even after you have answered "overwrite all" to a question.
* **Only the names that match** takes a mask, so only what matches travels. It applies to
  files and never to directories, and a directory is only created once something under it has
  matched — so `*.rs` out of a source tree gives you the directories that have Rust in them
  and no others.
* **Copy what a link points at, not the link** is off, and deliberately: a copy that turned
  links into copies of their targets would duplicate a tree the first time you tried it.
* **Keep the times** and **Keep the permissions** are on.

Whatever you answer is what the next dialog opens on.

The destination line takes more than one destination, separated by `;` — quote a path that has
a `;` in its own name. They run one after the other rather than at once, and only the first of
a *move* is a move: after it the source is gone, so the rest are copies of what landed first.
Stopping one stops the ones behind it.

`Shift+F5` and `Shift+F6` copy and move *into the same directory*, which is how a file is
duplicated or renamed without touching the other panel. `Shift+F8` deletes without the
question, and says so in its own title.

`Alt+F6` makes a link. `Ctrl+G` runs one command over everything selected, with the name of
each file put into the line where you mark it.

# Checksums

*Checksums…* in the Files menu writes a list of digests for what is selected: SHA-256, SHA-1
or MD5. One list for the lot — named after the directory, or anything else you type — or one
beside each file. The format is the one `sha256sum` and friends write, so the list can be
checked by whoever you send it to with the tool they already have, and the one they send you
can be checked here.

*Check against a list* works on the list under the cursor. Which hash made it is read off the
digests inside it rather than off its name, so a file called `SHA256SUMS`, `sums.txt` or
`release.md5` all work. What does not match is named in the report **and left selected in the
panel**, because the next thing you do with a bad download is fetch it again or delete it, and
both of those work on the selection.

`Esc` stops a long run.

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

# Finding things

`Alt+F7` searches for files: a mask, a size, text inside them, and where to look. The results
are a list you can view, edit or jump to — and `F5` turns the whole result into a panel, so
anything that works on a directory works on a search.

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

What is not there: an entry's permissions and times cannot be changed from the panel, and
`F7` does not make a directory inside an archive — nothing stores an empty one worth keeping.
Nor can an archive that is not a file on this disk be written into, for the reason above.

# What a file is

`Ctrl+A` opens the attributes dialog: what the file *is*, rather than what is in it.

On Linux and macOS that is the mode, as twelve boxes — owner, group and world, each with read,
write and run, plus setuid, setgid and sticky — and the line above them says what the file is
now, both as `rw-r--r--` and as `0644`, with who it belongs to above that. On Windows it is
read-only, hidden, system, archive, temporary, offline and whether the indexer may look at it
— and a dim line says what else the file is, where it is compressed, encrypted, sparse or a
reparse point: those are set by doing the thing, not by ticking a box. Read-only is on both,
because on Unix it is the write bits.

Below the boxes are the times — *Changed*, *Last read* and *Created* — as
`2026-10-24 17:42:05`. A time left blank is left alone; a time typed in is set. *Times in UTC*
reads and writes them in that clock instead of this machine's, which is how a file here and
its copy on a build server are compared. The three buttons under them fill the rows in without
answering the dialog: *Original* puts back what the file carries, *Current* writes this moment,
*Blank* empties them. On Unix a fourth time is shown and never offered — the moment the file's
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

What is not there yet: the owner and the group, and Windows' advanced bits.

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
