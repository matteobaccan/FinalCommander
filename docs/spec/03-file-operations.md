# SPEC 03 — File operations engine

Everything that touches the filesystem goes through `scmd-jobs`. The UI never calls
`std::fs::copy` directly.

**Implemented today** (M1): the plan/execute/settle pipeline, copy, move (with the
same-filesystem rename fast path), recursive delete, nested mkdir, the conflict matrix
(`Ask` / `Overwrite` / `OverwriteIfNewer` / `Skip` / `Rename`) with per-conflict and
`…All` answers, error decision points (retry / skip / skip all / cancel), progress with a
per-file and a total bar, cancellation, metadata preservation (mtime, Unix mode), symlinks
copied as links, write-to-`.scmd-part`-then-rename, the guards in §2, and a dry-run mode.
**Still to come**: the job queue and journal (§5), verification and
resume (§3), attribute dialogs (§7), multi-rename (§8), split and combine (§9), compare and
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
| Interruption | partial target removed (default) or kept for resume; never left under the final name — write to `name.scmd-part` then atomic rename |
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
  units and completion marks, so after a crash `scmd` can report what had been done and offer to
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

- `tool.compare.dirs`: side-by-side compare by name/size/mtime(±tolerance)/content hash, with
  "ignore case / ignore date / subdirectories" options; result colours rows in both panels and
  can select the differing/newer/orphan sets.
- `tool.sync.dirs`: a review table of planned actions (`→`, `←`, `=`, `≠`, `delete`), editable
  per row, one-way or two-way with conflict rules, dry-run by default, then handed to the job
  queue as a normal job.
