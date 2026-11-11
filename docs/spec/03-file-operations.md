# SPEC 03 — File operations engine

Everything that touches the filesystem goes through `fcmd-jobs`. The UI never calls
`std::fs::copy` directly.

**Implemented today** (M1): the plan/execute/settle pipeline, copy, move (with the
same-filesystem rename fast path), recursive delete, nested mkdir, the conflict matrix
(`Ask` / `Overwrite` / `OverwriteIfNewer` / `Skip` / `Rename`) with per-conflict and
`…All` answers, error decision points (retry / skip / skip all / cancel), progress with a
per-file and a total bar, cancellation, metadata preservation (mtime, Unix mode), symlinks
copied as links, write-to-`.fcmd-part`-then-rename, the guards in §2, and a dry-run mode.
Since then the queue (§5, 0.31.0), verification and resume (§3), the attribute dialog (§7),
multi-rename (§8), split and combine (§9) and synchronisation (§10) have shipped.
**Still to come**: the journal that lets a job resume after a crash (§5), and what each
section's own *still to come* lists.

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

### 2c. Path masks in the filter

A glob with a `/` (or `\`) in it is matched against the path from the selected item down,
`/`-separated, or any tail of it that starts after a `/`; every other mask still matches the
name (FAR's issue #58). A directory that a path mask after the `|` leaves out wholly is not
read. The synchronisation's filter (§10a) works the same way on the path from the roots.

### 2b. What an error does (`JobOptions::on_error`)

Chosen in the copy dialog before anything starts (FAR's issue #597): `ask` stops on each error
with retry, skip, skip all and cancel; `skip` records it and goes on, as *skip all* would from
the first; `stop` records it and ends the job there, keeping what was already done. The choice
is remembered for the next dialog with the others.

## 3. Copy semantics

| Aspect | Behaviour |
| --- | --- |
| Buffer | adaptive 64 KiB…8 MiB based on measured throughput; direct I/O never forced |
| Fast paths | **Shipped (0.18.0)** behind `general.system_copy`, see §3b: `copy_file_range` (Linux), `fcopyfile` (macOS), `CopyFileExW` (Windows); server-side copy on SFTP/S3 where supported, which the providers already do |
| Sparse files | holes detected (`SEEK_HOLE`) and preserved; option to materialise |
| Metadata | what `operations.preserve` lists, which is what the three *Keep* boxes of the copy dialog start as: `mtime`, `mode`, `xattrs`, all three by default (`Preserve`). Applied after the data and as a courtesy: one that cannot be applied does not fail the copy. Extended attributes are §3d. Not built: atime/btime, owner/group, ACLs, compression/encryption flags |
| Symlinks | copy as link (default), follow, or ask; relative links rewritten when the target moves inside the copied tree (option) |
| Hard links | links inside one job preserved (inode map) when `preserve_hardlinks = true` |
| Verify | **Shipped (0.31.0)** as *Read every file back and compare it* in the copy dialog (`JobOptions::verify`): after each file, both sides are read again and compared by SHA-256, the hash already in the tree for checksums, rather than blake3, which would be a dependency for one option. A copy that differs is removed and reported as that file's error; a move deletes its source only after its copy compared the same. A side that cannot be read a chunk at a time — a server, a bucket, an entry in an archive — leaves the copy standing and counted as *not verified*, which the summary says |
| Interruption | never left under the final name — written to `name.fcmd-part`, then renamed into place. `Esc` removes the part; a failure or a crash leaves it, and the next copy of the same file goes on from it (§3c) |
| Size limit | a file over the target filesystem's own limit (FAT: 4 GiB − 1) is refused before it is opened, as that file's error naming the limit and the filesystem, and the job goes on (FAR's issue #532). The filesystem is the one `mount_of` finds for the target |
| Permissions | target dir not writable → decision point with an "elevate" action (SPEC 06 §7) |

Move: same-filesystem → `rename`; otherwise copy+verify+delete, and the delete step only runs
after a successful verify. A cross-device Move that fails halfway leaves every file either at
the source or fully at the target — never both-partial.

### 3d. Extended attributes (`Preserve::xattrs`)

A copy between two local paths carries the source's extended attributes, so a file tagged in
Dolphin (`user.xdg.tags`) or the Finder arrives tagged. `fcmd_platform::xattr` has the three
calls — list, get, set, each on the path itself and never through a link — for Linux
(`llistxattr` and its kin) and macOS (`listxattr` with `XATTR_NOFOLLOW`); on every other
platform a file lists none. `JobFs::copy_xattrs(src, dst)` is the engine's side of it, with a
default that carries nothing, which is the answer of a server, a bucket and an archive.

* **When.** For a file, after its data and *before* its permissions: a read-only source makes
  a read-only copy, and that could no longer be tagged. For a directory, when it is created
  rather than at the settle with its times — nothing a child does disturbs an attribute, and
  by the settle a move has already removed the source. A move that can rename renames, and
  the attributes travel with the inode; one that cannot (another device) is this copy.
* **Best effort.** A destination that keeps no attributes (FAT, a share, a `tmpfs` mounted
  without them) refuses, and the file is copied all the same, with no question and no error:
  the same courtesy as the times. A destination that refuses with "not supported" is asked
  once per file, not once per attribute.
* **Which, on Linux: `user.*` only.** `security.*` (SELinux labels, file capabilities),
  `system.*` (ACLs) and `trusted.*` say what a file is allowed on the system it was on, and a
  file manager that carried them elsewhere would be granting what nobody granted. `cp` leaves
  them behind too, short of `--preserve=all`.
* **Which, on macOS: all of them**, as the Finder copies: tags, the label, *where from*, the
  resource fork, and `com.apple.quarantine` with them. Keeping the quarantine mark is a
  decision: a file that came from the internet is still one after it has been copied, and
  dropping the mark would make a copy a way round Gatekeeper.
* **Through links.** With *Copy what a link points at* the attributes are the target's, since
  the bytes were. A link copied as a link carries none: Linux keeps no user attributes on one.
* **Not Windows.** NTFS alternate data streams are another model and out of the core
  (`docs/BACKLOG.md`, Settled).

Not covered, and said so: the search-and-replace in files and the editor's save both write a
new file and rename it over the old one, and the attributes of the old one are not carried
across that rename.

### 3c. Going on with an interrupted copy (`JobOptions::resume`, shipped 0.31.0)

A file is written as `name.fcmd-part` and renamed only when it is whole, so an interrupted copy
leaves exactly one thing behind: the part. `Esc` is the user's own stop and takes it away. A
failure — a read or a write that failed, a disk that filled — and a crash or a kill leave it,
and the next copy of the same file to the same place **goes on from it**: the bytes already
there are reported at once, the source is read from that offset, and the rest is appended.

The part is trusted only when it is no longer than the source and its last 64 KiB (or all of
it, when shorter) are the source's bytes at the same offset. A part left by another file of the
same name, or by an older version of this one, fails that and is written over. Checking the end
rather than every byte is what makes resuming cheap; *Read every file back and compare it*
(the verify box) reads the whole file when that is wanted. On by default, because it only ever
applies where a part is there; the system's copy routine (§3b) is not used for a file being
continued, because it copies whole files. The queue's `F5` retry (§5) therefore goes on with a
large file that failed rather than starting it again.

Local files only for now: a server or a bucket copies through its own provider, which writes
its own way (SFTP's resumable reads and FTP's `REST` are backlog lines of SPEC 04 §6).

### 3b. The system's own copy routine (`general.system_copy`, shipped 0.18.0)

Reading a file into a buffer and writing it out again is the default, and it earns that: it
reports progress every chunk, it stops the moment `Esc` is pressed, and it behaves the same on
every filesystem anybody has. What it cannot do is let the *kernel* make the copy, and on the
filesystems people keep large files on that is the whole difference:

| Platform | Call | What it buys |
| --- | --- | --- |
| Linux | `copy_file_range`, a chunk at a time | the bytes never enter this program; on btrfs, XFS and bcachefs it becomes a reflink, and over NFS 4.2 the server copies the file |
| macOS | `fcopyfile(COPYFILE_DATA)` | the routine the Finder uses, which clones on APFS |
| Windows | `CopyFileExW` | the routine Explorer uses, and the one that knows a share's own server-side copy |

Three rules keep it from being a second copy engine rather than a faster one:

1. **It writes the same temporary file.** The destination is `name.fcmd-part` as always, and
   the rename into place, the permissions and the times are the engine's own afterwards. The
   routine moves bytes; it decides nothing.
2. **A refusal is not a failure.** Two different filesystems, a kernel without the call, a
   filesystem that does not implement it — each answers "no" rather than an error, and the
   byte loop then does the work with nobody told anything, because nothing went wrong. Only a
   failure an ordinary copy would hit as well is reported as one.
3. **It is off by default, and what it costs is said in the same breath.** Where the kernel
   copies a whole file in one call — macOS and Windows — `Esc` is answered when that call
   returns rather than within the next few kilobytes, and that file's progress arrives in one
   piece. On Linux the copy is still made a chunk at a time, so neither applies there. A
   setting that is faster *and* answers a cancel later is a choice for the person doing the
   copying, not a default.

Every call that needs `unsafe` or a `#[cfg]` is in `fcmd-platform`, which is the rule the
workspace is built on; the engine asks it one question and takes `None` for an answer.

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

**Built (0.31.0).** Every job `start_job` starts — copy, move, delete, wastebasket, wipe,
attributes, links and the rest of what is a `JobSpec` — is a row of the queue (`App::queue`,
`crates/fcmd/src/queue.rs`) from the moment it is asked for until well after it ends.

- **Lanes.** A job waits while another runs on its *lane*. By default the lane is the device
  of the target — `st_dev` of the target, or of its nearest existing ancestor, on Unix; the
  drive or share on Windows; `scheme://authority` for a server; the archive's own file for a
  path inside one — and of the first source's folder for a job with no target. Two copies
  onto one disk run one after the other, and one onto a memory stick does not wait for one
  onto the network (Total Commander's copy by queue). `general.job_queue` is `device` (the
  default), `one` (one lane for everything) or `off` (no waiting).
- **Background.** `B` in a job's progress box sends it to the background; the top row then
  counts the jobs nobody is watching (`⧗N`, and `?` while one is asking). A question from a
  job in the background comes up at once over the panels, and waits — with a line on the
  status bar — over the viewer, the editor or a dialog. A job that ends in the background
  says so on the status line, never in a box over what the user is doing now.
- **The list** (`job.queue.show`, `Alt+Shift+F5`, *Commands → Jobs*): running, paused, asking,
  waiting, held and finished rows, with the percentage done. `Enter` brings a running job's
  box back or shows a finished one's errors; `Space` pauses a running job and holds a waiting
  one; `Ctrl+↑↓` reorders the waiting ones, which is the order they start in; `Del` takes a
  waiting job out, stops a running one and clears a finished one; `F5` retries a job that
  failed or was stopped, for what it did not do — a copy or a move again with conflicts
  *skipped*, which is exact because the engine never leaves part of a file behind, and
  without the sources a move or a delete already took away. The last twenty finished jobs are
  kept.
- **Pausing** is in `CancelToken`: a paused token makes `is_cancelled()` wait, so every place
  an engine already checks for a cancel is a place it holds, with no engine knowing about
  pauses. A paused job keeps its lane. Its elapsed time keeps counting, so its speed and
  estimate are of the wall clock.
- **Chains.** The next destinations of a copy to several places and the steps of a
  synchronisation ride on the job before them and start when it ends well; a failure or a
  cancel drops them.
- **Quitting** with jobs running or waiting asks, naming them, and the answer is *stop them
  and quit* or *stay*: jobs are threads of this process.

Outside the queue, shown as before and never in the background: the replacement's counting
run (its result is a question), packing and rewriting an archive, archive tests, checksums.

**Still to come:**

- Cloning a finished job into a new one. (A server's speed limit is in since 0.34.0: `max_kbps`
  on the session, SPEC 04 §6.)
- The journal of §5a for what is not running: a job still waiting in the queue, and the chain
  behind a running one.

### 5a. The journal: a copy that survives a crash (`fcmd_jobs::journal`, 0.41)

A Copy or a Move that runs keeps a journal, `fcmd-jobs/<pid>-<id>.journal` beside the
configuration (`Paths::jobs_dir`). It holds what was asked — kind, sources, target, every
option — and, as the job goes, the destinations of the files that are **done** and of the
files that are **open**: begun and not known to be finished, which is the one being written
and any that failed before it. Each open one may have left a `.fcmd-part` (§3c). A dry run, a
job that is not a copy, and a job with a path that is not text keep none.

- **Written whole, renamed into place.** This section used to plan an append-only `.jsonl`.
  It is one text file instead, one fact per line with `end` as its last, rewritten to
  `<name>.tmp` and renamed over the old one: a crash is exactly what the file is for, and an
  append can be half a line long at the moment of one. The engine crate needs no serialiser
  for it, and somebody who finds one can read it.
- **Per file, a few times a second.** The recorder is told of each file begun and finished
  (`execute_recorded`) and writes when 250 ms have passed since the last write — and never
  sooner than twenty times what the last write took, so a journal of a hundred thousand names
  cannot cost more than a twentieth of the job. Inside a large file the engine's progress
  tick gives it its turn, so the file is on record as open long before it ends. It is not
  synced to the disk. The price of all three is the same and is accepted: a file finished in
  the last instant may be unmarked, and the resumed job meets it as a conflict, under the
  policy the job was started with.
- **Removed when the job ends** — clean, with errors, or cancelled — together with the folder
  when that leaves it empty. A job that failed is retried from the queue (§5), in the session
  that still has it. Quitting with jobs running stops them; one whose thread had not got to
  its end when the process went leaves its journal, and is offered at the next start.
- **Alive or dead** is a lock, not a process id: the job holds `<pid>-<id>.lock` locked
  (`File::try_lock`) for as long as it runs, and the system takes a lock away when a process
  dies. A journal whose lock is held belongs to a running job — in this process or in another
  copy of the program — and is not offered.

**At startup** `journal::left_in` reads the folder before the loop starts. Each journal with
no live owner is a job that never ended: the status line says how many, once, and
`job.journal.show` (`Alt+Shift+F6`, *Commands → Unfinished jobs*) lists them. A file there
that is not a journal this version reads — no header, cut short, a line it does not know — is
a startup warning and a row of the same list, never a stop.

- `Enter` **resumes**: the journal's own spec is queued again with `resume` on, carrying the
  dead run's marks. The engine passes over every unit whose destination is marked done —
  counted in the progress, never asked about, never looked at, because whatever happened to
  that file since is the user's doing — and the open file goes on from its part. A Move drops
  the sources that are no longer there (`journal::remaining`): moved is what they are. The
  resumed run writes a journal of its own and removes the old one once that is in place. It
  is not pushed on the undo stack (§14): half of what it wrote was written by another session.
- `Del` **discards**: the journal goes, and the `.fcmd-part` of every open file it names, on
  a worker thread. Finished files stay.

Only the running job is journalled. The jobs waiting behind it, and the chain it carries (the
next destinations, the later steps of a synchronisation), are not: they have to be asked for
again. Resuming *within* a file is §3c's and so is local only; a remote copy taken up again
skips its finished files and starts the open one over.

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
  masks (mc's `-R` behaviour), owner and group as one `user:group` row — a name or a number
  either side, an empty half left alone, the link itself rather than its target, and the
  `chown` made before the `chmod` because a successful one clears setuid and setgid. Name
  completion and an ACL editor are M3.
- Windows: R/A/S/H checkboxes, compressed/encrypted/not-content-indexed, inheritance note,
  owner (M3).
- Timestamps: modified/accessed/created(birth) with "set to now", "copy from other panel",
  and recursive apply.
- Tri-state checkboxes for multi-selection (FAR's `[?]`), so an unchanged box stays unchanged.
  **Shipped (0.19.0).** A box the selected files do not agree about opens at `[?]`; `Space`
  walks it through *on*, *off* and back to *leave it alone*, so a box pressed by mistake can be
  put back. What travels to the engine is two masks — the bits to set and the bits to clear —
  rather than one whole mode, which is what lets a bit nobody answered stay as each file has
  it. The same holds for the read-only box both platforms have and for Windows' own attribute
  word. Before this the dialog wrote the mode of the file *under the cursor* onto everything
  selected, which is a quiet way to make a hundred files agree about something nobody asked
  about.

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

**What is built** (0.31.0, `file.multirename` on `Ctrl+Shift+F6` and *Files → Rename from a
pattern*, with `fcmd_core::rename` and `fcmd_jobs::rename_all` behind it): every field above
but `[X]` and `[=meta]`, one search with its replacement (plain, or a regex with `$1`), the five
case transforms, the counter with dialog-wide defaults that `[C10,2,3]` overrides, and a
preview under the dialog's rows that follows every key, rows with a problem first. A problem is
one of four: nothing left, not a name (a slash, a NUL, `.`, `..`), two rows to one name, or the
name of a file that is there and is not itself being renamed away. A plan with any problem in
it starts nothing. Only the files whose current name is another rename's target take the detour
through a temporary name; a failure halfway undoes the renames already made, in reverse. Undo
(*Files → Undo the last multi-rename*) is the last plan reversed, kept in the state file as
`[last_rename]` so it outlives the session that did it — one step, not a history. The export,
the ordered list of regex rules, transliteration and the dedup rule are still to come.

## 9. Checksums, split, combine (shipped)

**Checksums.** MD5, SHA-1, SHA-256 and CRC32 over what is selected, written as a list of
digests, and a list checked against the files beside it. The dialog asks three things: which
hash, whether one list holds them all or each file gets its own, and what the list is called.

- The format written is the one `md5sum`, `sha1sum` and `sha256sum` write and read —
  `<hex>  <name>`, two spaces — because the point of a checksum file is that somebody else can
  check it with the tool they already have. The formats *read* are wider: the same line with
  ` *name` (what those tools call binary mode), and BSD's `SHA256 (name) = <hex>`, which is
  what macOS and FreeBSD produce. Blank lines and `#` comments are skipped.
- **A CRC32 list is a `.sfv`**, which reads the other way round: `name hex8`, the digest at
  the end of the line, `;` for a comment, and eight hex characters zero-padded because a
  checker that read `1a2b3c4` as a CRC would be checking a different number. It is what the
  Usenet and bulletin-board habit left behind, it is still how a set of parts is posted, and
  nothing reads a `.sfv` written in the `<hex>  <name>` shape — so a CRC32 list is written in
  this one and never in that one. The file beside a single file is `file.sfv`, not
  `file.crc32`. CRC32 is an error check and not a hash: two files with the same CRC32 are
  easy to make on purpose, so it answers *did this arrive intact* and nothing else, and the
  dialog lists it last.
- **Which hash made a list is read off the digests in it**, not off the file's name: 32 hex
  characters is MD5, 40 is SHA-1, 64 is SHA-256, 8 is CRC32. A list is called `SHA256SUMS` or `sums.txt`
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
- The hashes come from `md5`, `sha1`, `sha2` and `crc32fast`, all of which were already in
  the tree — the first three under the SSH client, the fourth under `zip` and `tar` — so this
  costs nothing to build.

**Split and combine** (0.31.0, `file.split` and `file.combine`, *Files → Split file* and
*Combine files*; `fcmd_jobs::split`). The pieces are **Total Commander's**, so a file cut here
is joined there and the other way round: `name.001`, `name.002`, … — three digits, more only
past 999 — and beside them `name.crc`, three `key=value` lines, `filename=`, `size=` and
`crc32=` in upper-case hex, with CRLF ends as Total Commander writes them.

- *Split* asks where the pieces go (the other panel, as a copy would) and how big each one is,
  in the size grammar of the filters (`1440K`, `700M`, `4G`; upper case is binary, lower case
  decimal); the last piece is what is left. A file no bigger than one piece is not split.
  Or how many: a number in the *pieces* row decides when it is there, and the size row is not
  read. The worker, which is where the file's size is known, turns it into the one piece size
  that makes exactly that many (`size_for_count`, `ceil(total / count)`); a count no single
  size can make — ten bytes are four pieces or five, never six — is refused naming the nearest
  one that can, rather than producing a different number of pieces from the one asked for.
  Nothing is written over: a piece or a `.crc` already there stops it before the first byte,
  and a failure or `Esc` takes back the pieces written so far.
- *Combine*, on any piece or on the `.crc`, joins `.001` upward for as long as the pieces
  follow on, into the other panel's folder, under the name the `.crc` gives (kept to a name:
  a path in it is dropped to its last part) or the pieces' own base name. The file is written
  under a temporary name and appears only when it is whole and, where there is a `.crc`, its
  size and CRC32 match: a missing or damaged piece is reported and leaves nothing behind.
  Pieces without a `.crc` still join, and the summary says it could not check them.
- Both run in the background with their progress in the status line, as the checksums do,
  and `Esc` stops them. Local files only: the bytes are read and written here.

Still to come: the self-joining script, and reading
other series (`.part1.rar`, `.zNN`), which are archives' own and opened as archives.

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

**Saved under a name (0.41).** The dialog has a row, *Save this pair as*. A name in it makes
`OK` write the two directories and every answer of the dialog as a `[[sync]]` block of
`fcmd.toml` (SPEC 07 §2b-septies) — in place of a block of the same name — save the file, and
go on to the table as an unsaved run does. `tool.sync.saved` (`Ctrl+Alt+Y`, *Commands → Saved
synchronisations*, and the dialog's own *Saved…* button) lists the blocks: `Enter` walks that
block's two directories, whatever the panels show, and puts the table on screen; `F4` opens
the dialog filled in from it; `Del` removes the block and saves. A saved pair is still
reviewed before it runs. That is deliberate: the table is made from what the directories hold
now, not from what they held when the block was written, and *take out what the other side
does not have* is not an answer to carry out unseen because it was ticked once. So "a backup
is one key" is one key to the table and `F2` — two.

**The report (0.41).** The other new row, *Write a report of the run to*, names a file on this
machine. A run that has one is followed as it goes (`crates/fcmd/src/syncrun.rs`): each of its
jobs is a *step* that carries the table's rows, and `job_done` records how each ended. When
the chain ends — its last job finished, or a step failed or was stopped and the rest were
dropped (§5) — a worker writes the text and the status line says where it is, or why it could
not be written. The text is plain, in the interface's language: a heading, the moment, the
name if there is one, the two directories, and then counted lists of rows:

- copied left → right, copied right → left, taken out on the right, taken out on the left;
- **left alone though they differ** — rows answered `=`, and the clashes nobody resolved;
  rows that are the same on both sides are not listed;
- **failed** — each error with the path it names. A row counts as failed when an error names
  it or anything under it, and is then in none of the lists above;
- for a step stopped part of the way, its rows under a heading that says they *may or may
  not* have been done — the engine's outcome counts files and does not name them, and the
  report does not guess;
- for the steps that never started, their rows as *not done*.

Each run writes the file anew, and its directory has to exist. The table is still the dry
run; what the report adds is a record of what the run then did.

### 10b. The rest

- A saved synchronisation that runs **unattended** — from the command line or on a schedule,
  with the table skipped under a rule that says when that is safe — and a report that is
  appended to rather than replaced, for a history of runs.

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
| Go into the subdirectories · how deep | the whole of both trees rather than the two listings; §11b says what comes back |

The sentence at the end says everything the comparison decided and nothing it did not: how many
pairs were read, how many differ, how many differ only in their line endings (and whether that
counted), how many were taken as the same without being read, how many files were on one side
only, and how many could not be read at all.

### 11b. Into the subdirectories (shipped)

With *Go into the subdirectories* on, the question is no longer about two listings — it is
about two whole trees, and the listings cannot answer it. Both sides are walked in a worker,
with the search engine's own walker (so a link that points back up a tree ends its branch
rather than the comparison) and with the depth the dialog asked for: `1` is the two
directories themselves, `0` is all the way down.

The pairing is by the path each file has **under its own root**, so `a/b/c.txt` is compared
with `a/b/c.txt` and not with whatever happens to be called `c.txt` over there. Each pair is
then settled exactly as §11 settles one: by the size, the time and the tolerance, or by the
bytes where that was asked for, with the fragment limit and the line-endings question.

The answer cannot be a selection: a difference three directories down cannot be shown as a
marked row in a panel that is not showing that directory. So it is a **panel of what differs**,
one row per path, marked with which way it goes:

| Mark | What it means |
| --- | --- |
| `≠` | on both sides, and not the same |
| `→` | only the left tree has it |
| `←` | only the right tree has it |
| `≈` | on both sides, and the only difference is the line endings |

The rows are the real files — the left-hand copy for a pair or a left-only file, the right-hand
one for a right-only file — so `F3` reads one, `Enter` goes to where it lives, `F5` copies it
and `F8` deletes it, all with the ordinary confirmations. The sentence at the end counts each
mark, the pairs looked at, the pairs taken as the same, what could not be read, and whether
`Esc` stopped it early.

## 12. Duplicate files (`tool.duplicates`, shipped)

The third thing the compare plugin does: what under this directory is a copy of what. It is
the question behind "where has the disk gone", and no listing can answer it.

Three passes, each cheaper than the one that would do the job alone. **Walk** the tree with the
search engine's own walker — so a link that points back up the tree ends its branch here as it
does there — collecting every file's path and size. **Group by size**, and throw away every
group of one: two files of different lengths cannot be copies, and this settles almost
everything without opening a file. **Hash** what is left, group by digest, and keep the groups
of two or more.

A hash rather than a side-by-side read, which is the opposite of what §11 does, and for a
reason: here each file is compared against every other file of its size, so reading each once
beats reading it once per pair. The digest is SHA-256. Empty files are left out — they are all
copies of each other, there are hundreds on any machine, and a list that opens with four
hundred of them is a list nobody reads.

The result is a **panel** of the copies, one row per file, each row numbered with its group
and zero-padded so that the panel's own sort by name keeps a group together; the size column
holds the file's size, and the groups come in order of what they waste — the first group is the
one to look at. The path is shown as it reads from where the hunt started, because two rows of
a group are told apart by nothing else.

Nothing is deleted and nothing is even selected. The rows are the real files, so `F3` reads
one, `Enter` goes to where it lives, and `F8` on what one picks out is the ordinary delete,
with the ordinary confirmation. The sentence at the end says how many groups, how many files,
how much is wasted, how many files could not be read, and whether `Esc` stopped it early.

Still to come: the plugin's other two definitions of a duplicate — same name whatever the
contents, and same size whatever the name — and its picture and music comparisons, which need
an image decoder and a tag reader.

## 13. Writing files again (`file.recode`, shipped)

Text arrives from somewhere else: a zip from a Windows machine, a dump out of a database, a
file somebody mailed. What it is in is not written down anywhere — a `.txt` of nothing but
ASCII is every code page at once — so this asks three questions and the first is the one that
matters: *what are they now*. The honest default is to work it out per file, from the byte-order
mark where there is one and from what the bytes look like where there is not; the row exists so
a file that was guessed wrong can be told.

Then: what to write them as, and what to do about the line endings — leave them, `LF`, or
`CRLF`. The selection, or the file under the cursor; a directory is never one, because a code
page is a property of bytes.

Three refusals, each of them a decision rather than an accident. A file that does not look like
text is refused, because nothing good comes of "converting" a disk image. A file larger than
64 MB is refused, because decoding reads the whole of it and a file that does not fit in memory
is a file this is the wrong tool for. A target page that cannot hold every character in the
file is refused by name, rather than writing a file full of question marks.

A file that would come out exactly as it went in is **not written at all**: rewriting it would
change its modification time and tell every backup in the house that it had changed. The write
itself goes to a file beside the original and is renamed over it, so an interruption leaves the
original whole, and the original's mode is carried across, because a script that was executable
has to stay executable.

## 14. Undoing an operation (`file.undo`, 0.39.0)

`Ctrl+Alt+Z` (*Files → Undo last operation*) takes back the newest of up to twenty remembered
operations, after a dialog naming what it will do and the first five names. The engine does
not report which names it made, so the undo is **worked out when the job is queued**, with the
planner's own rule (a target that is a folder, or more than one source, means *into*; else
*as*), and **kept only if the job ends uncancelled and without an error**. Jobs queued by an
undo are not remembered themselves.

| Operation | Undo |
| --- | --- |
| Copy | a `Trash` job of the destinations that did not exist when the copy was queued; one that did is left out, because the copy may have written over it and the wastebasket would then hold the only copy of neither |
| Move / rename | `Move` jobs back: one into each original folder for the names that kept their name, one per name for those that changed it; only for destinations that did not exist when queued |
| Trash | restored on a worker through the system's wastebasket (`trash::os_limited`: freedesktop and Windows), the newest item for each original path, and only where nothing is in its place; on macOS an error says to use the Finder |
| Mkdir | the folder removed on a worker if it is still empty, refused if not |

Local files only (not a server, a bucket or an archive), no extraction (`contents`) and no
relative copy (`relative_to`). A real delete and a wipe have nothing to come back from and are
not offered. The multi-rename keeps its own undo (§8), which survives a restart; this one is
held in memory.
