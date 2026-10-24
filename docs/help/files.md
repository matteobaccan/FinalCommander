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

Everything that reads a file works in there: the viewer, the hex editor, the search. Writing
into an archive is not there yet, so the panel refuses rather than pretending.

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
