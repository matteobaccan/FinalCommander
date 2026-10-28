# SPEC 03 — File operations engine

Everything that touches the filesystem goes through `fcmd-jobs`. The UI never calls
`std::fs::copy` directly.

**Implemented today** (M1): the plan/execute/settle pipeline, copy, move (with the
same-filesystem rename fast path), recursive delete, nested mkdir, the conflict matrix
(`Ask` / `Overwrite` / `OverwriteIfNewer` / `Skip` / `Rename`) with per-conflict and
`…All` answers, error decision points (retry / skip / skip all / cancel), progress with a
per-file and a total bar, cancellation, metadata preservation (mtime, Unix mode), symlinks
copied as links, write-to-`.fcmd-part`-then-rename, the guards in §2, and a dry-run mode.
**Still to come**: the job queue and journal (§5), verification and
resume (§3), attribute dialogs (§7), multi-rename (§8), split and combine (§9), and
synchronise (§10).

## 1. Job model

```rust
pub struct Job {
    pub id: JobId,
    pub kind: JobKind,              // Copy, Move, Delete, Wipe, Mkdir, Link, Attrs,
                                    // Pack, Unpack, Checksum, Split, Combine, Sync, Scan
    pub source: Vec<VPath>,         // as selected, before expansion
    pub target: Option<VPath>,
    pub options: JobOptions,
    pub state: JobState,            // Planning | Running | Paused | AwaitingDecision | Done | Failed | Cancelled
    pub progress: Progress,         // files done/total, bytes done/total, current item, speed, eta
    pub log: Vec<JobEvent>,
}
```

Lifecycle: **plan → execute → settle**.

1. **Plan**: expand the selection (recursive walk, cancellable, streaming), count files and
   bytes, detect the easy cases (same-device rename for Move, CoW/reflink availability,
   server-side copy for the same remote), detect collisions up front where cheap, and estimate.
   Planning is itself a progress-reporting phase: no 10-second freeze on "Preparing…".
2. **Execute**: a worker pool (default: 1 for same-device local, 4 for network/VFS, configurable)
   processes a work queue of `Unit { src, dst, kind }` items. Directories are created first,
   metadata applied after their contents, empty dirs preserved.
3. **Settle**: apply deferred metadata (times on directories), fsync policy, verify pass if
   requested, refresh affected panels, write the journal entry, free temporary state.

## 2. Conflict policy

Per-job default plus per-conflict decision:

```
Overwrite | OverwriteIfNewer | OverwriteIfDifferentSize | Skip | Rename(auto|ask)
| Append | Resume | AskAlways
```

The dialog shows: both sides' size, mtime, attributes and, for text/image files under a size
limit, an optional quick diff/preview. Buttons: Overwrite, All, Skip, Skip all, Rename,
Resume, Cancel. Answers are remembered per job; "All" variants are scoped (all / all of this
extension / all in this directory).

Special conflicts handled explicitly: target is an existing **directory** vs file; target is a
**symlink** (replace the link or write through it — explicit setting, default: replace link);
source == target; target inside source (recursive copy guard); read-only/hidden/system target;
name invalid on the target FS (Windows reserved names, trailing dots/spaces, `:` in names,
length limits, case-insensitive collision) → offered transliteration/truncation with preview.

## 3. Copy semantics

| Aspect | Behaviour |
| --- | --- |
| Buffer | adaptive 64 KiB…8 MiB based on measured throughput; direct I/O never forced |
| Fast paths | `copy_file_range`/`FICLONE` (Linux), `fclonefileat`/`copyfile` (macOS), `CopyFileEx` with `COPY_FILE_NO_BUFFERING` for large files (Windows), server-side copy on SFTP/S3 where supported |
| Sparse files | holes detected (`SEEK_HOLE`) and preserved; option to materialise |
| Metadata | mtime always; atime/btime, mode, owner/group, ACLs, xattrs/ADS, compression/encryption flags per option set (default: mtime+mode) |
| Symlinks | copy as link (default), follow, or ask; relative links rewritten when the target moves inside the copied tree (option) |
| Hard links | links inside one job preserved (inode map) when `preserve_hardlinks = true` |
| Verify | optional read-back hash (blake3) or size+mtime check; mismatch → decision point |
| Interruption | partial target removed (default) or kept for resume; never left under the final name — write to `name.fcmd-part` then atomic rename |
| Permissions | target dir not writable → decision point with an "elevate" action (SPEC 06 §7) |

Move: same-filesystem → `rename`; otherwise copy+verify+delete, and the delete step only runs
after a successful verify. A cross-device Move that fails halfway leaves every file either at
the source or fully at the target — never both-partial.

## 4. Delete, trash, wipe (shipped)

- **`F8`** hands each thing to the system's own wastebasket when `general.use_trash` says so,
  which it does by default — as it does in FAR and in Total Commander, because the one key
  everybody presses by accident is the one that should land in a safety net. The platform half
  is the `trash` crate: the XDG trash spec on Linux (the file moves and a `.trashinfo` beside
  it records where it came from), `NSFileManager` on macOS, `IFileOperation` on Windows. Each
  of those is what that desktop's own file manager shows and restores from, which is the whole
  point — a hidden directory we invented would be a promise nothing else knows about.
- It is **a job kind of its own** (`JobKind::Trash`) rather than an option on a delete, because
  the shape of the work is different: a wastebasket takes a directory with everything in it in
  one operation, so there is no tree to walk and no file to count. One unit per thing the user
  chose, and each one is a rename as far as the filesystem is concerned.
- Only this machine has one. A server, an archive and a bucket do not, so those delete outright
  — and the dialog says which of the two is about to happen rather than leaving it to be
  guessed. If the wastebasket refuses a path it could have taken (on Linux, a device with no
  trash directory), the error dialog names the key that deletes it outright.
- **`Shift+Del`** (`file.delete.permanent`) never uses the wastebasket, whatever the setting
  says, which is what that key means on every desktop. `F8` with the setting off is the same
  thing.
- `file.delete.wipe` (`Alt+Del`) → overwrite with a configurable pattern (default: one pass of
  zeros), truncate, rename to a random name, unlink. Documented honestly: on CoW/flash/journaled
  filesystems this is *not* a guarantee, and the UI says so.
- Recursive delete asks once, with counts and the first three paths.

Still to come: browsing the wastebasket as a panel, and putting something back from inside the
program.

## 5. Queue and background

- Jobs are listed in a **queue panel** (`job.queue.show`): running, queued, paused, finished
  with results. Jobs can be reordered, paused, resumed, cancelled, cloned, retried for failed
  items only.
- Default concurrency: one job per *target device*, others queued (TC-style "copy by queue"),
  configurable. Network targets get their own lane with a bandwidth cap option.
- A job survives panel navigation, tab switching and viewer/editor use. Quitting with running
  jobs asks; `--detach` is not supported in M2 (jobs are in-process), so the dialog offers
  "wait" or "cancel".
- **Journal**: each job writes an append-only record (`$STATE/jobs/<id>.jsonl`) with planned
  units and completion marks, so after a crash `fcmd` can report what had been done and offer to
  resume a Copy/Move.

## 6. Progress UI

One compact widget, two densities:

```
Copying 1 248 / 9 310 files · 4.1 / 22.7 GB · 182 MB/s · ETA 1m 42s
/home/mb/src/project/target/debug/deps/libfoo.rlib
[=============>              ] 43 %   total  [====>        ] 18 %
```

Rules: current item path elided from the middle; two bars (current file, total) when the job
has more than one file; speed smoothed over 3 s; ETA hidden until 5 s of data; `Tab` moves the
job to the background; `Esc`/`Ctrl+C` cancels after confirmation.

## 7. Attributes, permissions, timestamps

One dialog, OS-aware:

- Unix: rwx matrix + numeric field, setuid/setgid/sticky, recursive with separate dir/file
  masks (mc's `-R` behaviour), owner/group pickers (name completion), ACL editor (M3).
- Windows: R/A/S/H checkboxes, compressed/encrypted/not-content-indexed, inheritance note,
  owner (M3).
- Timestamps: modified/accessed/created(birth) with "set to now", "copy from other panel",
  and recursive apply.
- Tri-state checkboxes for multi-selection (FAR's `[?]`), so an unchanged box stays unchanged.

## 8. Multi-rename

Pattern language (TC/Krusader-compatible in spirit, documented as ours):

```
[N]        full name              [N3-5]  characters 3..5 of the name
[E]        extension              [C]     counter  [C10,2,3] start,step,width
[d]/[t]    date / time of the file (strftime sub-pattern: [d:%Y-%m])
[P]        parent directory name  [X]     clipboard line
[=meta:width]  plugin/metadata field (exif, id3, …)
```

Plus an ordered list of **regex rules** (`s/pat/repl/flags`), case transforms
(lower/upper/title/sentence), transliteration, space/dash normalisation, and a dedup rule.
The dialog shows a live before/after table with collisions marked in red; applying writes a
rename plan that handles cycles (`a→b, b→a`) via temporary names, and the plan can be exported
and re-imported. Undo restores the previous names from the journal.

## 9. Checksums (shipped), split, combine

**Checksums.** MD5, SHA-1 and SHA-256 over what is selected, written as a list of digests, and
a list checked against the files beside it. The dialog asks three things: which hash, whether
one list holds them all or each file gets its own, and what the list is called.

- The format written is the one `md5sum`, `sha1sum` and `sha256sum` write and read —
  `<hex>  <name>`, two spaces — because the point of a checksum file is that somebody else can
  check it with the tool they already have. The formats *read* are wider: the same line with
  ` *name` (what those tools call binary mode), and BSD's `SHA256 (name) = <hex>`, which is
  what macOS and FreeBSD produce. Blank lines and `#` comments are skipped.
- **Which hash made a list is read off the digests in it**, not off the file's name: 32 hex
  characters is MD5, 40 is SHA-1, 64 is SHA-256. A list is called `SHA256SUMS` or `sums.txt`
  or nothing in particular, and a name can lie where a length cannot. A list whose digests do
  not agree on one length is not guessed at — it is reported as a list this cannot check.
- The name offered is `<directory>.<hash>`, which is what a commander writes, and it is a text
  row because a Linux release writes `SHA256SUMS` instead and both are read by `sha256sum -c`.
- **What does not match is left selected** in the panel, the way a comparison by content leaves
  what differs: the next thing one does with a bad download is fetch it again or delete it, and
  both of those work on the selection. The report names the first twenty and says how many
  more.
- Reading the bytes is `JobFs::read_chunks` — the reading half of a copy with the writing half
  left out — so when a provider implements it, checksums work there with nothing else changed.
  Today only the local filesystem does, and anything else is refused before the dialog opens
  rather than halfway through a run. `Esc` stops a run, as it stops a comparison.
- The three hashes come from `md5`, `sha1` and `sha2`, all three of which were already in the
  tree under the SSH client, so this costs nothing to build. CRC32 and `.sfv` are not here:
  that is a different file format, and `docs/BACKLOG.md` says so.

**Split and combine** are still to come: into volumes by size or count, with an optional
self-recombining script; combine detects `.001`/`.part1`/`.zNN` series automatically.

## 10. Compare and synchronise

Three ways in, one engine: `tool.compare.dirs` compares the two listings by name, size and
time; `tool.compare.contents` reads the bytes of every pair; `tool.compare` (Commands menu)
opens the dialog of options in front of either, and §11 says what the options are. All three
end the same way — what differs is selected in both panels, and one line says what was found.

### 10a. Synchronise (shipped)

`tool.sync.dirs` (`Ctrl+Shift+Y`, and the Commands menu) is three things in a row, and the
order matters: a dialog that says how to decide, a **table** that says what was decided, and
only then the jobs. Nothing is written until `F2` over the table, so what is on screen is the
whole of what is about to happen, and every row of it can be changed first.

**Deciding** is `fcmd_jobs::sync`, which writes nothing at all. It walks both sides, pairs
them up by the path they have in common, and gives each pair a verdict and a default action:

| | |
|---|---|
| only on the left | `→` copy it over |
| only on the right | `=` leave it, or `←` both ways, or `→☒` take it out when that was asked |
| both, the same | `=` |
| both, different | `→` one way; both ways, the newer side wins |
| both, different, no rule to pick a side | nothing happens until somebody says which |

Two directories both sides have are **walked**, not paired: what differs is inside them, and a
row saying "these two directories differ" is a row nobody can act on. A directory only one
side has is **one row**, because the copy takes its tree with it and a row per file inside it
would be a hundred answers to one question.

The same is decided by size and time, with **two seconds of slack** (`sync::TIME_SLACK`): FAT
stores a time to the nearest two seconds and so does a zip's own directory, so a file that has
been to a memory stick and back comes home one second different through nobody's fault.
Treating that as a change would make every sync to a stick copy everything, every time —
which is the one thing a sync must not do. *Compare the bytes* instead hashes both sides and
compares the digests: two streams cannot be read side by side through a filesystem that pushes
bytes at a callback rather than letting one pull them, so each is read once rather than both at
once. It is the only option that sees a change of the same size at the same time.

**Doing** is the copy and the delete that already exist — there is no `JobKind::Sync`. The
table hands out up to four ordinary jobs, copies first so that a row saying "take this away"
cannot run before the row that was going to put something in its place, and queued one at a
time because two jobs writing at once would fight over the progress box and over the disk. The
copies overwrite without asking, deliberately: the table *was* the question, answered row by
row, and asking again about every file would undo the point of having reviewed it. A delete
goes to the wastebasket when `general.use_trash` says so, exactly as `F8` does, because a sync
that shredded what `F8` would have kept would be a nasty surprise.

The one thing that made this fit in the existing engine is `JobOptions::relative_to`: a copy
normally puts a source at `target/name`, which is right for files picked out of a panel and
wrong for a set that came from walking a tree. With a root to be relative to, `a/b/c.txt`
lands at `target/a/b/c.txt`, so one job carries a whole tree at the paths it had.

Both sides have to be on this machine for now: the walk reads both and one of them is written,
and it is said before the dialog opens rather than halfway through a run.

Still to come: saving a pair of directories as a named job, and the dry run as a *report*
rather than as the table (the table is already the dry run, but it does not write one down).

### 10b. The rest

- A named **job** that remembers a pair of directories and its rules, so a backup is one key
  rather than a dialog each time, and the queue of §5 to run it in.

## 11. What counts as the same file (`tool.compare`, shipped)

FAR has this as a plugin — `AdvCmpEx` — with a dialog of its own, and the dialog is the point:
"are these two folders the same" has no answer until somebody says what *the same* means, and
that changes with the job. A tree copied by something that dropped the timestamps has to be
compared by its bytes. Two checkouts of one repository differ in every line ending and in
nothing else. A memory stick keeps the time to the nearest two seconds. A directory of disk
images cannot be read end to end to answer a question that the first megabyte settles.

So the dialog is the front end of both comparisons, and every row of it is one of those cases.
It writes its answers into `[compare]` (SPEC 07 §2c-quater), so the next comparison is the one
that was set up rather than the default again:

| Row | What it changes |
| --- | --- |
| Match the names with their case | `Readme.md` and `README.MD` are two files rather than a pair |
| A different size is a difference | off, and only the time decides |
| A different time is a difference | off, and only the size decides |
| Times this many seconds apart are the same moment | two is the useful one: FAT, and a zip's own directory, keep the time to the nearest two seconds |
| Read the bytes | the slow, certain comparison rather than the listing's word |
| Read them only where the times differ | what makes a large tree finish: same size at the same moment is taken as the same |
| Read only the first so many KiB | do these two *start* the same way — for recordings, images, anything where reading all of it would take a day |
| Line endings alone are not a difference | off by default, because the comparison counts such a pair separately and says so either way |
| Only what is selected | the dozen files one cares about out of a directory of ten thousand |
| Stop at the first difference | the answer to "are these the same at all" does not need the other nine thousand files read |
| Select only the newer copy | on: what one wants before `F5`. Off selects both sides of every difference, for looking rather than copying |

The sentence at the end says everything the comparison decided and nothing it did not: how many
pairs were read, how many differ, how many differ only in their line endings (and whether that
counted), how many were taken as the same without being read, how many files were on one side
only, and how many could not be read at all.

Still to come: into the subdirectories, which needs the tree walked in a worker as the search
is, and the third thing the plugin does — find the duplicate files under a directory.
