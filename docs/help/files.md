# Copying, moving, making, deleting

`F5` copies, `F6` moves or renames, `F7` makes a directory, `F8` deletes. The destination of a
copy or a move is the other panel, and the dialog says so before anything happens.

Every one of them is a job: it runs in the background with a progress bar, `Esc` stops it, and
what it has already done stays done. A conflict — a file that is already there — stops and
asks, with "all" and "none" answers so a long copy is not a thousand questions.

`Shift+F5` and `Shift+F6` copy and move *into the same directory*, which is how a file is
duplicated or renamed without touching the other panel. `Shift+F8` deletes without the
question, and says so in its own title.

`Alt+F6` makes a link. `Ctrl+G` runs one command over everything selected, with the name of
each file put into the line where you mark it.

# Finding things

`Alt+F7` searches for files: a mask, text inside them, and where to look. The results are a
list you can view, edit or jump to — and `F5` turns the whole result into a panel, so anything
that works on a directory works on a search.

Replacing across files is the same dialog with a replacement, and it counts the files and the
matches before it writes anything. The count comes from the same engine that then does the
work, so the two cannot disagree.

# Archives

`Shift+F1` packs what is selected — and can delete the originals once the archive is really
written, which is a box in the dialog and never the default. `Shift+F2` unpacks the archive
under the cursor into a directory of its own. `Shift+F3` is the archive menu: test it (every
entry is decompressed and checked, nothing is written), extract it here, extract it
elsewhere, delete it, or add files to a new one.

`Enter` on an archive opens it as if it were a directory — zip,
tar, and tar with gzip, bzip2, xz or zstd — and the path says where you are:

    /home/me/sources.tar.gz!/src/main.rs

Everything that reads a file works in there: the viewer, the hex editor, the search.

Writing works too, and it works the only way these formats allow. None of them can be edited
in place — a `.tar.gz` is one compressed stream from end to end — so a change means writing
the archive again: the old one is streamed into a new file beside it with the change applied,
the new one is read back and checked, and only then does it replace the original. An
interrupted change therefore leaves the archive exactly as it was and a `.part` file to
delete. Entries nobody touched are copied across as the compressed bytes they already are, so
taking one file out of a hundred-megabyte zip is a copy and not a recompression.

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

# What a file is

`Ctrl+A` opens the attributes dialog: what the file *is*, rather than what is in it.

On Linux and macOS that is the mode, as twelve boxes — owner, group and world, each with read,
write and run, plus setuid, setgid and sticky — and the line above them says what the file is
now, both as `rw-r--r--` and as `0644`. On Windows it is read-only, hidden, system and archive
instead. Read-only is on both, because on Unix it is the write bits.

Below the boxes are the two times, as `2026-10-24 17:42:05` in local time. A time left blank
is left alone; a time typed in is set. With a folder among what is selected there is one more
box: *and everything inside the folders*, which takes the whole change down the tree. The
directories themselves are changed after what is in them, so taking the write bit off a folder
cannot lock the program out of its own contents halfway through.

It reports like a copy — a progress box, a count, `Esc` — and a file that refuses is listed
with the reason rather than passed over quietly. A file on a server gets its mode and its
modification time, which is what SFTP carries. An entry inside an archive has the archive's
permissions and an object in a bucket has the policy of its bucket, so neither has anything to
set, and both say so instead of pretending.

What is not there yet: the owner and the group, and Windows' advanced bits.

## Comparing two folders

`F9 → Commands` has both of them, and they answer different questions.

**Compare folders** is FAR's own, and it is instant: it matches names, compares the size and
the time, and leaves selected in each panel what the other side does not have or has an older
copy of. What stays selected is what a copy would have to carry.

**Compare by content** reads the files. Every pair with the same name is opened on both sides
and the bytes are compared — a pair whose sizes already differ is settled without opening
anything, and the rest are read until the first byte that differs, which for two files that
are not the same is usually in the first block. It is the question to ask about two trees that
were built from the same source, or copied by something that did not keep the timestamps,
where the date proves nothing.

Both panels have to be on a local filesystem for it, and anything with no counterpart on the
other side is selected too, because it is a difference. A long comparison says how far it has
got in the status line, and `Esc` on an empty command line stops it.
