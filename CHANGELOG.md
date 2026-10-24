# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[semantic versioning](https://semver.org/), which before 1.0 means anything may still move.

## [Unreleased]

### Added

- **The key bar follows the modifier.** Hold Shift and the bar becomes the archive keys —
  `Add`, `Extrct`, `ArcCmd`, `Edit…`, `Copy`, `Rename`, `Delete`, `Save`, `Group`, `SelUp`;
  hold Ctrl and it becomes the sort keys; hold Alt and it becomes the drives, the link, the
  search and the two histories. The words are FAR's own, in all seven languages, and the bar
  is built from the keymap rather than written out, so it cannot promise a key that is bound
  to nothing — a keymap of your own is described correctly for free. A terminal has to speak
  the keyboard protocol for this (kitty, foot, WezTerm, Ghostty and others do); where it does
  not, the bar simply stays as it was.

- **`F1` is about where you are.** It opens the keys of the screen you pressed it on — the
  panels, the viewer, the editor, the hex editor, a dialog — each with a sentence saying what
  it does, grouped by subject, generated from the keymap as loaded and from the commands'
  own descriptions. A binding of your own appears there with the right words next to it, and
  a key nobody bound appears nowhere. `Right` goes on to the chapter that explains that
  screen at length, `Esc` to the table of contents, and leaving the help puts you back where
  you were rather than on the panels.

- **The search dialog** (`F7`), in the viewer and the editor, is FAR's: what to look for,
  whether it is text, hex or a regular expression, whether case matters, whether only whole
  words count, and then *Find next* or *Find previous* — the same question asked in two
  directions. It opens filled in with what was asked last. **Whole words** is new in the
  engine too, for both the viewer and the editor and for plain text and regular expressions
  alike: it is the difference between finding `set` and finding `offset`.

  `Enter` now answers a dialog wherever the cursor is, as it does everywhere else and as it
  does in FAR; `Space` is what turns a box on and off.

- **Syntax highlighting**, in the editor and the viewer, for Rust, C, C++, Java, C#,
  JavaScript, TypeScript, PHP, Python, Ruby, Go, shell, SQL, HTML, XML, CSS, Markdown, JSON,
  TOML, YAML, INI, Make, Dockerfile, Lua and Perl. It is a scanner rather than a parser, and
  deliberately: it knows how a language writes a comment, a string and a number plus the words
  anyone would call keywords, which colours real files correctly without a grammar per
  language.

  The editor keeps the state each line starts in and extends it as you scroll, throwing it
  away from the first line an edit touched — so closing a comment uncolours the lines below
  it in the same frame, and a file of a million lines costs only what is on screen. The
  viewer starts at the top of the window, because it never reads more of a file than it
  shows; a comment opened above the window is not carried, which is what buys opening a
  forty-gigabyte log instantly. The editor's status line names the language.

  Each class is a `syntax.*` theme key and every built-in theme defines all of them in its
  own palette, foregrounds only, so a coloured run never changes the page behind it.
  `ui.syntax = false` turns the whole thing off.

- **The change-drive dialog is a table** (`Alt+F1`, `Alt+F2`), as it is in FAR: every row
  says what the thing is — disk, removable, optical, network, memory — what it is called,
  what it is formatted as, how big it is and how much is left, read from the real mount
  table rather than guessed. The things that are not filesystems come underneath, numbered
  `0` to `9` the way FAR numbers its plugin panels, and the number takes the row. Mounts that
  are part of how the system runs (`/proc`, `/sys`, `/dev`, `/run`) are left out: a `tmpfs`
  under `/run` is plumbing, while a `tmpfs` someone mounted on `/mnt/fast` is a choice, so
  where a thing is mounted decides and not what it is.

- **The panels can be resized.** `Ctrl+←` and `Ctrl+→` move the line between them,
  `Ctrl+↑` and `Ctrl+↓` take rows off both and give them to the user screen — the output of
  the last command, right there under the panels, which is what those keys are for in FAR.
  The split is a percentage so it survives a resize of the terminal, neither panel can be
  squeezed below ten columns, and both numbers are settings (`panel.split`,
  `panel.height_cut`) that `Shift+F9` keeps.

- **The archive commands menu (`Shift+F3`)** — MultiArc's `ArcCmd`. On an archive, or inside
  one: test it, extract it here, extract it elsewhere, delete it, or add files to a new one.

- **Testing an archive.** Every entry is read and decompressed and the bytes are thrown away;
  what the format checks, it checks — a zip's CRC, a decompressor's own framing. It reports as
  a job, so there is a progress bar, Esc stops it, and the entries that failed are listed with
  the reason each one gave. A compressed tar is walked once rather than reopened per entry.

- **`Shift+F1` can delete the originals** once the archive is written. It is a box in the
  dialog, off by default, and the deletion happens only if the pack really finished: a pack
  that failed or was stopped leaves every file where it was.

- **`Shift+F11` and `Shift+F12`** toggle *directories first* and *selected files first*, which
  are FAR's `Group` and `SelUp`.

- **Object storage in the panels.** `s3://logs/2024/08/` opens a bucket as a tree of prefixes,
  on AWS or on anything else that speaks the protocol — MinIO, Ceph, Garage, R2, Wasabi, a
  LocalStack in a container — with the endpoint, the region and the addressing style named in
  an `[[s3]]` block of `scmd.toml`. `s3://work@bucket/` picks a service by name and
  `s3://work@/` lists the buckets it can see.

  A bucket has no directories: `2024/08/13/access.log` is one key with slashes in it. What the
  panel shows as a folder is a *common prefix*, which the service computes, so opening a prefix
  costs one request whether the bucket holds a thousand keys or ten million. Asked for
  everything under a prefix instead, the listing really is the whole bucket and it stops at
  `listing_cap` rows (100 000 by default) — and when it stops the footer says `(partial)` for
  as long as that listing is on screen, with a line saying why. SPEC 04 §8 has the measured
  numbers for a bucket of 1.7 million objects, since that is the question this started from.

  No access key is stored in `scmd.toml`, ever: a block says *where* its keys come from, and
  they are read from the `AWS_*` variables or from `~/.aws/credentials`, the file every AWS
  tool shares. `credentials = "anonymous"` signs nothing, for a bucket with a public read
  policy. TLS is `rustls` — one implementation on every system, and no OpenSSL to find — but
  the trust anchors are the machine's own, because an S3-compatible endpoint inside a company
  is usually a MinIO or a Ceph behind that company's certificate authority, and a root list
  bundled into the binary cannot be told about it. The signature is AWS Signature Version 4, HMAC-SHA256 over a canonical request, and
  it is tested against `botocore` — the library the AWS command line signs with — on a frozen
  clock, one of those vectors being the worked example AWS publishes the signature of.

  `F3` on an object brings it down to a temporary copy and shows that, the same path a file on
  a server takes, with `net.fetch_limit_mb` as the ceiling — so the viewer's seeks, its hex
  mode and the disassembler all work on it. Objects are read-only in this release: the provider
  claims `READ | SEEK | CASE_SENSITIVE` and nothing else, so nothing in the UI offers to rename
  an object (object storage has no rename), set permissions on one or write to one. `F5` from a
  bucket to a local panel is not wired to the job engine yet, and that — with upload, delete
  and server-side copy — is the next increment.

- **Six more example plugins**, taking the folder to twenty-one and thirty-three commands:
  `dir-sizes` (every folder here measured and ranked, where FAR's `Ctrl+Space` does one at a
  time), `line-count` (lines by extension under here, skipping `node_modules`, `vendor`,
  `target` and the rest of what nobody wrote by hand), `syntax-check` (does this parse? `php
  -l`, `python -m py_compile`, `node --check`, JSON, `ruby -c`, `sh -n`, `perl -c`, chosen by
  what the file is), `env-diff` (which keys `.env` is missing from `.env.example` — names
  only, never values, because a secret in a dialog is a secret in a screenshot), `ssh-hosts`
  (what `~/.ssh/config` already knows, `Include` followed, `Host *` left out) and `strip-bom`
  (the three bytes that put a blank line before `<?php`, found and taken off, with the file's
  mode kept and the result checked by looking again rather than by trusting the loop).
- **Tunnels over the netbox connection** (`F9 → Commands → Open a tunnel…`, or `net.tunnel`
  from the palette): `8080:db.internal:5432` makes this machine's 8080 reach `db.internal:5432`
  as the server sees it — SSH's `direct-tcpip`, the thing `ssh -L` is built on, over the
  session the panel is already using. No second authentication, no second host-key check, no
  second connection: one more channel on the one that is open. `db.internal:5432` uses the same
  port on both sides and a bare `5432` means the server's own. The local end binds **127.0.0.1**
  and not every interface, because a tunnel is the server's network handed to this machine and
  binding it everywhere hands it to the office; `Tunnels…` lists what is open with the
  connections carried, open now and refused by the far end, and `Enter` closes one. Tested
  against a real SSH server in the test process, with a real service at the far end: the
  channel, the copying both ways, the counters, the port freed on close, and a far end that
  refuses without taking the session down.
- **Compare by content** (`F9 → Commands`), beside FAR's own compare folders. That one matches
  names and compares size and time, which is instant and right nine times out of ten; this one
  reads the files. Every pair with the same name is opened on both sides and the bytes are
  compared — a pair whose sizes already differ is settled from the listing without opening
  anything, the rest are read until the first byte that differs — and what differs is selected
  on both sides, along with anything that has no counterpart at all. It is the question to ask
  about two trees built from the same source, or copied by something that did not keep the
  timestamps, where the date proves nothing. No hashing: a hash reads every byte of both files
  and then compares thirty-two, while this stops at the first difference, and a hash can
  collide where a comparison of the bytes cannot be wrong. The reading happens on a worker
  thread with the progress in the status line, and `Esc` stops it.
- **Screenshots**, in `docs/shots`, drawn by the renderer itself rather than photographed: the
  real panels are rendered into a cell buffer and the buffer is written as SVG, so regenerating
  them is one command and they cannot show a version of the program that no longer exists.

### Fixed

- **Three example plugins did not run on macOS**, and the build said so: in bash 3.2, which is
  the bash macOS has shipped since 2007, a `case` written inside a `$( … )` does not parse —
  the `)` of the first pattern closes the substitution and the error lands on the `;;`, on a
  line that looks perfectly fine. A newer shell parses it, so `sh -n` here could not see it.
  The `case` has moved into a function in two of them and become a plain test in the third,
  and a new test reads the scripts for the shape rather than parsing them, so this cannot come
  back on a machine the test suite does not run on.

### Changed

- **A provider may now say that a listing is not all of it**, through the new `Dir` the panel
  reads directories with. Nothing local changes — a local directory is always read whole, and
  `list` still answers with the rows alone for the job engine, the finder and the dialogs.
  `Listing::truncated` existed since the first release and was read by nobody; it now means
  something and is shown.
- **Ordering a panel by name is about twice as fast**, and three times with selected-first on,
  which matters on a directory or a prefix with a hundred thousand rows in it. The comparison
  now works on the bytes of a name instead of validating every name as UTF-8 twice per
  comparison, skips the head two names share with a word-at-a-time comparison, and answers
  "is this row selected" once per row rather than hashing it on every comparison.
- Two names that differ only in how a number is padded — `img2` and `img02`, `0` and `00` —
  no longer compare **equal**. They are different files, and an order that called them equal
  left the two rows wherever the sort happened to drop them. The padding decides, but last.

## [0.1.1] — 2026-10-24

Three things found by reading 0.1.0's own code an hour after cutting it. Two of them could
lose work, which is why this is a release of its own rather than a line in the next one.

### Added

- Assigning a recorded macro now **asks how it should play**: once, or once for every selected
  file. The second mode existed in 0.1.0 and could only be chosen by editing the file by hand,
  which is a mode nobody finds. `once` is the first row, so the ordinary answer costs one
  keystroke and `Esc` means the same thing rather than throwing the recording away.
- **An example plugin**, `examples/plugins/git-here`: a manifest naming one command and a
  shell script that answers it, saying which branch the panel's directory is on. "A plugin
  command is a process" is a sentence; a script one can read in twenty seconds is the
  explanation. A test loads it the way the binary does, because an example that does not work
  is worse than no example.

### Fixed

- **`F12` lost the screen it was drawn over.** The screens list replaced whatever was in front
  with itself, so pressing it while reading a file dropped the viewer — and with an editor in
  front, an afternoon's unsaved work with it. It parks the front screen now, the way the block
  menu parks the hex editor: `Esc` brings it back, choosing the panels sends it to the
  background, choosing another screen swaps them. And a question asked over a viewer belongs to
  that viewer, so `F12` from a search prompt lists the viewer as the screen in front instead of
  showing the panels and hiding the viewer from its own list.
- **A macro recorded in the editor was stored as a panel macro.** The documentation said the
  scope is where the recording started and the code wrote `panel` regardless, so a run of
  editor keys ended up on a panel key where its steps mean something else entirely. The scope
  is taken when recording starts now, which is also the right moment for a second reason:
  stopping a recording is itself a keystroke, and by then one may be looking at something else.
- `--list-plugins` said "(0 items)" for a plugin that provides a command: it counted the
  declared TOML blocks, which such a plugin has none of. It counts what a plugin actually
  brings now, data and commands separately.
- Sixteen message ids were translated into all seven languages and shown by nothing. There was
  already a test that every id the code asks for exists in every language; the other direction
  is now a test too, because that is the one that catches a catalogue growing a tail of text
  the program no longer matches.

### Notes

- **0.0.8 has no release**, and the changelog says so where its section is: the tag never
  happened, and everything in it reached users in 0.1.0. The workflow that cuts a release now
  says why it cannot when it cannot, instead of timing out.


## [0.1.0] — 2026-10-24

The first release meant to be used by somebody other than its author: macros, the screens, a
real help system, a manual page, and the first thing a plugin can do beyond carrying data.

### Added

- **Macros (`Ctrl+.` records, `Ctrl+Shift+.` lists)**. A macro is a list of *commands*, not of
  keystrokes: every action in the program is already a named command, so a recorded macro is a
  readable list of names in `scmd-macros.toml` that one can read, edit, diff and carry to
  another machine. What a key cannot say as a command is kept as a key — the letters typed into
  a prompt, `Enter` on a dialog — and recording tells the two apart by where the key went.
  The corner of the screen says `REC` and counts the steps while one is being recorded; the key
  pressed afterwards is the key the macro answers to, which is how one finds out it is free.
  Two play modes: once, or once for every selected file with the cursor put on each in turn —
  the mode that makes a macro worth recording. Anything that copies, moves, deletes, packs,
  overwrites or runs a shell command asks first and shows the steps, unless the file says
  `trusted = true`.
- **The screens (`F12`, `Ctrl+Tab`)**. A viewer or an editor is a screen one steps away from
  and comes back to, keeping its position, its search and its unsaved changes while the panels
  or another file are worked on. `F12` lists them with a `*` in front of one holding unwritten
  changes; the count of what is behind the panels sits in the corner. The panels are screen
  zero of the ring, so `Ctrl+Tab` from a screen steps out to them and from them steps in to the
  next: the panels are never more than one key away. `F10` from the panels now asks about every
  background screen with unsaved changes instead of ending the program on the spot.
- **A help system worth the key (`F1`)**. A table of contents: seven written chapters —
  what this is, the panels, files, the viewer, the editor, servers, configuration — compiled
  into the binary from `docs/help/`, and three **generated** ones: every binding by scope,
  every command with the keys that reach it, and the settings as this session has them. The
  generated half is built from the keymap actually in force, so it cannot drift; a written list
  of keys is wrong the day after it is written.
- **A manual page**, `docs/scmd.1`, generated from the command line's own options with the
  sections `clap` cannot know about written underneath, and shipped in the release archives. A
  test fails when the committed page and the binary disagree, so a flag cannot exist without a
  line in the page.
- **Plugin commands (`F11`)** — the first plugin tier that does something rather than carrying
  data. A plugin declares a `[[command]]` and the host runs it: deliberately a *process*, so
  one artifact works on every platform, there is no sandbox to get wrong, and a plugin that
  crashes crashes alone. It runs on the worker pool with the context as JSON on standard input
  and the same facts in the environment, and answers with one action per line. An action that
  runs something names a command of *our* vocabulary, so everything a plugin can do a key can
  do too. A command id outside the plugin space, a program that leaves the plugin's directory,
  or a context nobody can give is refused at load rather than at the first press.

### Fixed

- The manual-page test failed on Windows and only there, because the checkout converts line
  endings. `.gitattributes` pins the data the program reads with `include_str!` to LF, and the
  comparison ignores the endings anyway.
- The tag workflow waited half an hour for a CI run it was not allowed to read: reading whether
  CI passed is an Actions read, and a job with an explicit permissions block gets only what it
  lists. It says so immediately now instead of timing out, and a dispatch can name the commit a
  missed release was cut at.

### Notes for the first testers

Everything is a file you can read: `scmd --paths` says where they are, `scmd --dump-config`
prints the settings as the program has them, and `scmd --check-keymap` says whether a keymap is
sound. `F1` is the manual, `F9` names every dialog with its key, and the key bar always says
what the function keys do right now.

What is deliberately not here yet is written down rather than left to be discovered:
[`docs/BACKLOG.md`](docs/BACKLOG.md) is the list, each line with the reason it is a line and
not a feature.


## [0.0.8] — 2026-10-24

> **Not published as a release.** The tag never happened: the workflow that cuts one waited
> half an hour for a CI result it was not allowed to read, and by the time that was fixed the
> branch had moved on. Everything below reached users in 0.1.0 instead, which is why there is
> no `v0.0.8` on the releases page. The section stays because it says what was built and when.

The Hiew track: a hex editor with a patch list, block operations, an assembler, a calculator,
search by instruction pattern, and raw devices. The one thing deliberately left out is a
compiler — see below for what the assembler does and does not do.

### Added

- **Hex mode of the editor (`F4` on a binary, `Alt+H` either way)**. A piece table over the
  file, so an edit costs nothing at the front of a terabyte and the length can change without
  rewriting what follows. The two panes are the bytes and the text; `Tab` moves between them
  and both type. Changed bytes are drawn as changed until they are written, with a patch list
  beside them — offset, old byte, new byte — and the editor's own `Ctrl+Z`/`Ctrl+Y` over the
  whole session. `F2` saves atomically: a part file beside the original, synced, then renamed
  over the top, and the buffer reopens clean.
  It is in the editor and not the viewer on purpose: the viewer's read-only contract is
  exactly what lets it seek through a 40 GB file.
- **Block operations (`F3` marks, `F12` operates)**: write the block out to a file, read a file
  in, fill with a byte or a repeating pattern, exclusive-or with a key, copy or move bytes from
  elsewhere in the same file, insert zeros, delete. Each one says what the file's length will
  be before it happens, because the failure of a hex editor is a file that is quietly one byte
  wrong. The exclusive-or is named for what it is and not for security.
- **An assembler (`F9`)** for the instructions a patcher types: no operands, register to
  register, register and a number, one register, and a branch written as the address to arrive
  at rather than as a displacement — the shortest form that reaches it, unless `short` or
  `near` says otherwise. The architecture comes from the file's own header, so a 32-bit PE is
  assembled as 32-bit code on a 64-bit machine. Everything encoded is decoded again by the
  disassembler before the dialog shows it, so what appears is the instruction the processor
  will see.
  **Memory operands are refused by name**: `mov [rbx+rcx*4+8], eax` needs a full ModRM and SIB
  parser and brings exactly the addressing-mode mistakes a patcher cannot afford. The hex pane
  is still there for bytes nobody can spell, and the backlog carries the parser.
- **A calculator (`Alt+=`)**: one line of arithmetic answered in decimal, hex, octal, binary
  and as characters at once, with the bitwise and shift operators and literals in every base
  the tools around it use (`0x1f`, `1Fh`, `0b1010`, `0o37`, `'PE'`). The same parser reads the
  numbers typed into the assembler and the block operations, so a number means one thing
  everywhere in the program.
- **Search by instruction pattern (`F7` over a disassembly)**. The query matches what the
  disassembler prints, not the bytes: nobody looking for a `call` wants the four letters of it
  to appear in the file. `*` stands for whatever lies between the pieces of a pattern, so
  `mov *, eax` finds every move into that register, and the operands' column padding is
  collapsed first so the pattern does not have to know about it. `Shift+F7` and `Alt+F7` walk
  on as they always did.
- **Raw devices on Linux and macOS**, read-only. A block device is a file, but `stat` reports
  it as zero bytes long and the viewer believed that; the length is measured by seeking to the
  end instead, which the kernel answers without reading anything and which needs no `ioctl`
  and no C dependency. A device opens read-only, the status line names it and the size it
  measured, and the bytes are written *out* to a file rather than back over the disk — an
  atomic save renames a new file over the old one, and there is no such move onto a disk.
  Windows needs its own path and is on the backlog.

### Fixed

- `Esc` out of the block menu, or out of a question one of its operations asked, dropped the
  hex editor along with every unsaved patch in it. It goes back to the editor now.

## [0.0.7] — 2026-10-24

A fix release: 0.0.6's tree panel could hang, and it went out before the build on the other
two platforms had finished saying so. The Hiew track moves to 0.0.8.

### Fixed

- **`Ctrl+T` could spin for ever** on a path the tree could not find a row for — which on
  macOS was most paths, because `/tmp`, `/var` and `/etc` are symbolic links and the tree
  only showed real directories. The tree now shows a link that points at a directory, and the
  loop that opens its way towards the current directory is a loop that cannot spin: every turn
  either opens a row that was closed or gives up looking. It gives up when the deepest row it
  can see is already open and the path is still not under it — a name spelled differently by
  the filesystem, or a directory that has gone since.
- The archive dependencies brought in a licence our own policy did not list: `libbz2-rs-sys`
  carries Julian Seward's own bzip2 licence rather than one of the usual names. It is BSD-style
  permissive and both Debian and Fedora treat it as free, so it is in `deny.toml`'s allow-list
  with a line saying exactly that, and in `THIRD-PARTY.md` with the same.
- `actions/checkout` to v7, `upload-artifact` to v7 and `download-artifact` to v8, which have
  to move as a pair.

## [0.0.6] — 2026-10-24

The panels FAR has and we did not, and the view modes to go with them.

- **Quick view (`Ctrl+Q`)**: the viewer's own rows inside a panel's frame, following the other
  panel's cursor — so text, a hex dump of something binary and the search machinery are the
  same code the reader uses. The file is opened once and kept while the cursor stays on it. A
  directory says what it is (measuring one is `F3`'s job) and a file whose bytes are not local
  says that `F3` brings them out, because a preview that copies a file down on every cursor
  movement is a trap.
- **Info panel (`Ctrl+L`)**: where the other panel is, what kind of place that is — this
  machine, a server, inside an archive, a list of files — the free space, what the directory
  holds, what is selected, and the row under the cursor. Everything shown is already known:
  nothing is read from the disk to draw it.
- **Tree panel (`Ctrl+T`)**, with the directory cache that makes it bearable: rooted where the
  other panel's path starts and opening its way down a level per read until the cursor is on
  the directory that panel is showing. It is the one special kind that takes the focus, because
  moving its cursor is how it changes the other panel's directory. `→`/`Enter` open a branch,
  `←` closes it or climbs, `*` opens everything under the cursor up to two thousand rows, and
  `Ctrl+R` forgets a branch and reads it again. The cache outlives the panel, so leaving the
  tree and coming back costs no reads.
- All three sit in the *passive* panel and the key that turns one on turns it off, as in FAR.
- **`Ctrl+L` is settled**: it is FAR's info panel in both keymap profiles, and the
  change-directory dialog moves to `Ctrl+Shift+L` — a key it can afford now that `cd` works on
  the command line.
- **Compare folders** (Commands menu): names, sizes and times, no subdirectories and no
  contents, which is what makes it instant. Each panel keeps selected what the other does not
  have or has in an older version, so what stays selected is what a copy would have to carry.
- **FAR's five view modes on FAR's keys**: `Ctrl+1` brief (three columns of names), `Ctrl+2`
  medium (two), `Ctrl+3` full, `Ctrl+4` wide (name and size, two columns), `Ctrl+5` detailed.
  `Ctrl+3` and `Ctrl+4` therefore mean something different than they did in 0.0.5, which is the
  point: a FAR user's fingers are the specification. A mode now says how many columns it draws
  and whether it shows a size, so a column too narrow for what it holds falls back to fewer
  columns.
- **`Left` and `Right` move by a column** in the multi-column views, as they do in FAR, and do
  nothing in the one-column views — also as in FAR.
- The four view modes FAR has beyond ours (descriptions, long descriptions, owners, links), its
  alternative full view, the deeper folder comparison, and the key tabs will need now that
  `Ctrl+T` is the tree's are all lines in [`docs/BACKLOG.md`](docs/BACKLOG.md).

## [0.0.5] — 2026-10-24

Archives: a panel can be inside one, and a key puts files into one.

- **A path can point inside a container**, in the syntax SPEC 04 §4 specifies —
  `/home/mb/src.zip!/doc/readme.md`, and nested. Every path operation works on the innermost
  level, and `..` at the root of an archive leaves it, because an archive root has a parent and
  so is not a root. Two archives are two providers, and so are an archive and the directory it
  sits in: that is what stops a copy out of one being planned as a rename.
- **`scmd-arc` reads zip and the tar family** — plain, gzip, bzip2, xz and zstd — and builds
  all six. Every decoder and encoder is pure Rust, so the binary stays one file that
  cross-compiles without a C toolchain. The format comes from the bytes first and the name
  second: a zip called `.epub` is still a zip, while only the name can say that a gzip stream
  has a tar inside it.
- **`Enter` goes into an archive** and everything that works on a panel works in there: the
  sort, the selection matrix, `F3` (which brings the bytes out to a temporary file, the same
  shape as fetching a remote file) and `F5`, which copies out through the ordinary job engine
  with its conflicts, progress and cancellation.
- **`Shift+F1` builds an archive** out of what is selected, offered in the other panel and
  named after the one item or the directory, in any of the six formats with three compression
  levels. A name the user typed wins over the format row. It writes to a sibling `.part` and
  renames it into place, so a cancelled pack leaves no half-built archive where someone will
  later trust one.
- **`Shift+F2` extracts** into a directory of its own, named after the archive without its
  extension — a hundred files straight into a panel is nobody's intention. It is an ordinary
  copy job with one new option, which copies what is *inside* each source rather than the
  source itself.
- **The zip-bomb guards of SPEC 04 §4** are `[arc]` in the config file: entry count, unpacked
  total, the ratio to the archive's own size, and the nesting depth. A refusal names the limit
  that fired and what the archive asked for, because a user who meets one has to decide whether
  to raise it. An entry whose name climbs out of the archive is pinned at its root rather than
  trusted.
- **What is inside an archive cannot be changed yet** and says so in all seven languages:
  that is the rebuild of SPEC 04 §4, with an archive inside an archive and an archive on a
  server beside it in [`docs/BACKLOG.md`](docs/BACKLOG.md).
- The release archives are now `scmd-<platform>-<version>` and hold the files at their root,
  with no directory of their own.

## [0.0.4] — 2026-10-24

Finding things: a search worth the name, and a panel that can hold what it found.

- **A panel can be a list of files rather than a directory.** One method decides whether a
  row's path is the panel's own path with the name on the end or something the panel was told,
  and every operation, the viewer, the editor and the metasymbols go through it. A list panel
  re-reads by asking about its own paths again, so a file deleted since the list was built
  drops out; `Enter` on a row goes to the directory the file lives in; navigating away turns
  the panel back into a directory.
- **Find file (`Alt+F7`)**: masks in the filter grammar, text or a hex pattern, case
  sensitivity, whole words, files that do *not* contain it, five scopes, following symbolic
  links, folders in the results. It opens on the last search. Results stream into a list that
  is usable while the walk runs — `Enter` goes to the file, `F3` reads it, `F5` turns the
  results into the panel, `Esc` stops the search and keeps what it found.
- **The engine is its own crate** (`scmd-find`), so the dialog does not know how a walk works
  and the walk does not know what a panel is: the dialog makes a plain specification, and one
  place turns it into a query. Progress is throttled rather than sent per file, and the
  directories a walk cannot read are collected and capped instead of announced one by one — a
  search over a whole drive meets them by the dozen.
- **Replace across files**: its own job over the selected files, with the needle as text, a
  pattern or hex bytes. The dry run is not optional — the confirmation says how many matches in
  how many files by running the same engine that then writes, so what it promises and what it
  does cannot disagree. A rewritten file is left looking modified.
- The documentation says what is *not* in this first version rather than half-building it:
  searching inside archives, code pages, the size and date conditions, and saving a search are
  lines in [`docs/BACKLOG.md`](docs/BACKLOG.md).

## [0.0.3] — 2026-10-24

A command line, and the keys a FAR user reaches for without thinking.

### A command line, and the screen it writes on

- A **command line under the panels** (`ui.command_line`): everything printable goes to it once
  it has anything in it, and to quick search while it is empty — FAR's rule, so neither habit
  gets in the other's way. Word jumps break on the separators a path uses, `Ctrl+K` cuts to the
  end, and the history walks with `Ctrl+↑`/`Ctrl+↓` keeping the half-typed line safe.
- **Commands we run ourselves**, because a commander that shells out for `cd` cannot keep the
  panel in step: `cd`/`chdir` (with `~`, `-` and a remote panel's own notion of a path), `cls`,
  `set`, `pushd`/`popd`/`clrd`, `title`, `exit`, and `if [not] exist` / `if [not] defined`
  conditions, nestable.
- **Prefixes**: `view:`, `edit:`, `goto:`, `whereis:`, and `run:` with its output to a file, the
  screen, or nowhere.
- **Metasymbols** (SPEC 06 §5): FAR's `!.!`, `!`, `` !` ``, `!\`, `!@!`, `!&`, `!?title?init!`
  and the side prefixes, plus our own `{named}` forms, quoted per platform. An unrecognised
  `{…}` passes through untouched, so `awk '{print $1}'` still works, which is the whole reason
  for the rule.
- **The user screen** (`Ctrl+O`): what the commands printed, scrollable, capped, with each
  command as its heading.
- **Command history** (`Alt+F8`) as a list, and a small on-disk store that keeps it, the folder
  history and both panels' directories between sessions.
- **`Ctrl+G` applies a command to every selected file**, each file taking its turn as "the
  current file". They run in sequence rather than all at once, counted through in the status
  line, and the run stops at the first failure instead of pushing ninety-nine more over a
  broken one.

### The keys a FAR user reaches for

- **The selection matrix**: the modifier picks the criterion, not just the action. `Ctrl++` and
  `Ctrl+−` take everything with the extension of the file under the cursor, `Alt++` and `Alt+−`
  everything with the same name whatever its extension, `Alt+*` inverts folders too. One
  restore (`Ctrl+Shift+M`) undoes a whole sweep rather than one row.
- **`Shift+F5`, `Shift+F6`, `Shift+F8`** act on the item under the cursor whatever is selected —
  the case they exist for is a selection you forgot about. The list travels with the dialog, so
  selecting something else while it is open cannot widen the copy.
- **`Ctrl+F1` / `Ctrl+F2`** give the screen to one panel, and the focus goes with it.
- **`Ctrl+F8` / `Ctrl+F9`** sort by creation and access time. An entry whose filesystem does not
  keep the field sorts with the others that do not, at the old end, so the order stays total.
- **`Alt+Del` wipes**: the bytes are overwritten once, then the file is unlinked. The dialog
  says plainly what that cannot promise on an SSD, on a copy-on-write filesystem or with
  snapshots — including that the name survives in the directory and the journal.
- **`Alt+F6` creates a link** in the other panel, as a copy would: a relative symbolic link, the
  one that survives the tree being moved elsewhere. The engine does hard and absolute links
  too; the chooser is still to come.
- **Long names scroll sideways** with `Ctrl+Shift+←/→` (FAR's `Alt+←/→` is history here).

### The panel's own frame

- **Free space** on the device at the right-hand end of the bottom frame — the one thing that
  needs a platform call, and honest about saying nothing on a server or on Windows.
- **A scrollbar** in the right frame, drawn only when there is more than fits.
- **The panel's own status line** (`panel.status_line`), a rule and a row inside the frame with
  the item under the cursor. Off by default, because the screen-wide status bar says the same
  thing and two rows saying it wastes a row.

### One filter object

- The quick filter bar now takes the whole grammar: a comma or semicolon list, `*`, `?`,
  `[c-f]` ranges, `/regex/i`, an exclusion after `|`, and named groups (`<arc>`, `<temp>`,
  `<exec>`, nestable). A bare word still means "contains", so nothing anybody was already doing
  changed.
- It is **one value type** rather than three, the way FAR has it, and it already carries the
  conditions the find tool and the copy dialog will need: size ranges with binary and decimal
  suffixes, date ranges that can be relative in days, attributes as three states, and a
  hard-link condition. A mask that does not parse leaves the previous filter alone instead of
  flickering the panel empty.

### The viewer says what a binary is

- **Structure pane** (`F11`): beside the hex or disassembly rows, a pane that decodes the file —
  identity (format, architecture, bitness, kind, entry point), the header field by field with
  its raw bytes next to its meaning, and then the tables: ELF sections and segments, PE
  sections, Mach-O load commands and dylibs, the slices of a universal binary, the members of
  an `ar` archive, plus an ELF's interpreter and build id. `Tab` moves the keys between the
  bytes and the pane, `Enter` on a row takes the bytes to that offset.
- **Imports and exports**: an ELF's `DT_NEEDED` libraries and its `.dynsym` symbols with their
  binding and type, the PE import and export directories including forwarders and exports with
  no name at all, Mach-O's load-dylib commands and symbol table. The **.NET CLI header** too,
  with the runtime version, the flags in words, the entry-point token and the metadata root —
  and the identity line then says a PE is managed.
- **Symbols feed the disassembly**, and `Enter` on a `call` or `jmp` follows it, `Backspace`
  comes back, with the jumped-from stack on screen.
- It reads only the offsets that matter: an ELF keeps its section table at the end of the file,
  and our own 162 MB debug binary is decoded in four reads rather than loaded.
- Every offset is range-checked before it becomes a jump target, every table is capped, and a
  file that stops early fills the rows it can and says `truncated` — a corrupt binary is the
  normal case for this pane, not an error.
- **A hex row now fits the window**: never more bytes than the setting, never fewer than four,
  always a power of two so an offset stays aligned, and the structure pane takes the room left
  over instead of cutting the printable gutter in half.
- **`F6` crosses to the editor** on the line the bytes were showing, and `F6` in the editor
  crosses back to the byte the cursor was on. **`+` and `−`** walk to the next and previous file
  of the panel without going back to it, skipping directories and keeping the mode.
  **`Ctrl+F10`** brings the panel to whatever is being viewed. Decompiling moved to `Shift+F6`,
  since FAR's `F6` in the viewer means the editor.

### Jobs

- **Speed, elapsed and what is left** in the progress box. All three come from the average
  rather than the last tick, and the estimate is absent until there is enough to compute it: a
  number that jumps between four seconds and four hours is worse than no number.
- **Append** and **only newer** as copy policies, beside ask / overwrite / skip / rename.
- **Fixed, and it was a data-loss bug**: a move plans a copy and then a removal of every source,
  and the removals ran whatever happened to the copy. Answering "skip" to a conflict in a move
  left the destination alone *and deleted the source*. The runner now declines the removal of
  any source whose copy did not happen, and of any directory above it.

### Plugins

- `scmd-plug`: manifests discovered from disk, with themes, keymaps, associations, decompile
  rules and mask groups loaded from them, a version requirement checked against ours, and every
  path inside a plugin verified to stay inside it — relative, no `..`, and the canonical target
  inside the canonical directory, so a symlink cannot walk out either.
- `--list-plugins` says what was found, what is waiting for a host API, and what was refused and
  why.

### The screen looks like FAR's

- A **`far` theme**: FAR Manager's own default palette, element for element, read from
  `far/palette.cpp` and `far/colormix.cpp` of `FarGroup/FarManager` (BSD-3-Clause). Light cyan
  on blue panels, yellow for marked files and column titles, a cyan bar on the active panel's
  path, cyan menus with yellow shortcuts, light grey dialogs with cyan input fields, red warning
  dialogs.
- **Column titles** (`Name │ Size │ Date Time`) under the panel frame, with the sort-mode letter
  in the top-left corner — lower case for ascending order, upper case for descending — and a
  `*` beside it while a filter is on, exactly where FAR puts them. `panel.column_titles` turns
  the row off; with it off the sort mode goes back to the footer.
- The **active panel's path** is drawn inverted, as FAR draws it.
- **Menus have their own colours** instead of borrowing the dialog's: a dropdown is cyan with
  yellow shortcuts and a black selection bar, and the bar across the top is black on cyan.
- **Shadows.** Dialogs, menus, lists and the progress window darken two columns to their right
  and one row below, which is FAR's `DropShadow` geometry. The shadow halves what is already
  there rather than painting a colour, so it works in every theme and in 16 colours.
- **Four things that were not FAR's, checked against its source and corrected**: the key bar's
  background between the labels is grey on black rather than the label's cyan; the totals turn
  yellow on cyan while something is selected; file colours are FAR's six default highlighting
  groups in FAR's order, so a hidden directory is cyan rather than white and `*.bak` is
  litter-coloured; and the size column says `Up` and `Folder`, localised and centred the way
  FAR fits them, instead of `UP--` and `FOLDER`. Nothing is bold any more either: FAR draws with
  console attributes, which have no bold.
- Every theme gained the keys those changes need (menu, menu bar, dialog fields and buttons,
  warning dialogs, column titles, the active panel's title, the scrollbar, the key bar's
  background, the selected totals, the `<temp>` group) — all seven built-in themes define every
  key, and a test keeps it that way.
- The key bar now uses FAR's own wording where FAR ships the language: `UserMn`, `MkFold`,
  `ConfMn`, and their Italian, German, Spanish and Russian forms.

### Also

- **Quoting on Windows was wrong**: the backslash was in the set of characters that need
  quoting, which is right on Unix (it escapes the next one) and wrong on Windows (it *is* the
  path separator) — so every path a metasymbol expanded came back wrapped in quotes, which
  `cmd` then reads differently. Each platform has its own set now.
- A panel can be **a list of files rather than a directory**: one method decides whether a
  row's path is the panel's own path plus a name or something it was told, and every
  operation goes through it. Nothing reaches it from the keyboard yet — it is what find
  results and `panelize` will be shown in — but it is in this build.
- Four tests that only held on Unix now hold on both platforms, and one that needed a
  Windows privilege no longer insists on it.

### Documentation

- `docs/04-far-inventory.md`: FAR 3.x read from its own repository — 283 help topics, 311
  settings keys — with every feature marked against what we do, the three grammars worth
  matching exactly (panel columns, file masks, metasymbols), and a section on exactly how close
  the screen now is and what is still different.
- `docs/02-features.md` §5b: Hiew release VIII's published feature list, item for item, against
  what we have — what is shipped, what is in the backlog, and the two we are not doing.
- `docs/spec/02-panels.md` §9b (what FAR draws in a panel's frame and what each element costs),
  `docs/spec/06-tools.md` §7.3 (the four decisions apply-command had to take), and
  `docs/spec/10-menus-and-dialogs.md` for the menus and the dialogs.
- Three facts corrected in `docs/03-keymap.md` and one in the backlog, now that the source
  rather than the documentation is the reference.

## [0.0.2] — 2026-10-24

The menus. Everything the keys do can now be found by reading, which is the half of FAR that
makes it learnable.

### Menu bar (`F9`)

- FAR's five menus — Left, Files, Commands, Options, Right — with `←`/`→` between them, `↑`/`↓`
  over the entries, a letter to pick one, and a `√` on the view mode and the hidden-files
  setting the panel is actually in.
- `Left` and `Right` act on their own panel whichever one has the focus, as FAR's do.
- Every entry is a command from the same vocabulary the keys use, so there is no behaviour
  reachable only from a menu, and the shortcut shown beside an entry is read from the keymap in
  force — rebind it in `scmd.toml` and the menu says the new key.
- An entry whose feature is not here yet stays in the menu, drawn dim, and says
  `not implemented: <command>` when chosen: the menu is a map of the program, and a map with
  the gaps marked beats one that lies. A test stops those entries from advertising a key that
  already does something else.

### Settings dialogs

- System, Panel, Interface and Confirmations. Every row is bound to a configuration key and
  goes in through `Config::set` — the same code path as `--set key=value` — so a dialog cannot
  write something the file format does not accept.
- `OK` (or `F10`) applies to the running panels at once: no restart, nothing to save first.
  `Esc` keeps nothing.
- `Save setup` (`Shift+F9`) writes `scmd.toml` as the configuration now stands, themes that came
  from the file included. What it writes parses back into the same configuration.

### Lists to choose from

- Languages — switches the whole interface on the spot — and themes.
- Drives, mount points and open connections (`Alt+F1` left, `Alt+F2` right): the platform's
  roots plus every live `sftp://` connection, and a configured-but-closed site connects first.
- Folder history per panel (`Alt+F12`).
- Sort modes (`Ctrl+F12`), where choosing the order already in force turns it round, the way FAR
  does it, unless `panel.allow_reverse_sort` says otherwise. `Ctrl+Shift+F12` reverses directly.

### Netbox session editor

- `Alt+N` then `Ins`/`F7` stores a connection, `F4` edits the one under the cursor: name, host,
  port, user, authentication (SSH agent, key file or ask), key file, remote folder, group,
  keep-alive, compression. Connections no longer need `scmd.toml` to be edited by hand.
- It is validated before it is stored — a host is required, port and keep-alive must be numbers,
  the key authentication needs a file, and the result has to be a site the netbox can use. A
  dialog that cannot be applied stays open with the reason in the status line.
- No password or passphrase is ever a field: a session says *how* to authenticate and the secret
  is asked for when the connection needs it.

### Also

- The dialog framework gained text rows with a real caret and the editing keys.
- New commands: `app.themes`, `app.languages`, `app.setup.save`, `opt.system`, `opt.panel`,
  `opt.interface`, `opt.confirmations`, `panel.sort.menu`, `panel.drive(left|right)`,
  `panel.history`.
- Two new settings: `panel.allow_reverse_sort` and `general.confirm_overwrite`.
- 105 new messages in each of the seven languages.
- `docs/spec/10-menus-and-dialogs.md` specifies all of it; `docs/BACKLOG.md` lists what is
  specified but not built, with the open questions marked — including the two keys where we
  differ from FAR today (`Ctrl+A` is select-all here, `Ctrl+L` is change-directory).

## [0.0.1] — 2026-10-24

The first release anyone can download. It is a preview: the parts that are here are tested and
behave, and what is missing says so instead of failing quietly.

### Panels and navigation

- Two panels, keyboard-first, with the FAR key layout (`mc` is the second shipped profile).
- Brief, full, detail and wide views; sorting by name, extension, size, date or unsorted, with
  natural ordering, directories first and a second press to reverse.
- Navigation with history (`Alt+←`/`Alt+→`), parent, root, `Ctrl+L` to type a path, quick
  search by typing, `Alt+I` for a live filter, `Ctrl+H` for hidden files, `Space` for a
  recursive directory size, `Ctrl+=` to send the other panel here.
- Selection by `Ins`, by mask (`+`, `-`, `*`), invert and restore, with totals.

### File operations (`F5`–`F8`)

- Copy, move, create directory and delete, planned before anything is touched: the planner
  counts files and bytes and refuses the impossible (a directory into itself, a file onto
  itself) before the first write.
- A copy is written beside its destination and renamed into place, so a crash or a cancel
  never leaves a half file; a move deletes the source only once the destination is there, and
  a move on the same filesystem is a rename, so it copies nothing.
- Conflicts and errors are questions, not guesses: overwrite, overwrite if newer, skip,
  rename, and the `…All` variants; retry, skip, skip all or cancel on an error.
- Progress with per-file and total bars, cancellable at every step.

### Viewer (`F3`)

- Text, dump and hex on files of any size — position is a byte offset, so a 40 GB log opens in
  one read.
- Encoding detected (UTF-8, UTF-16 with or without a BOM, CP1252, CP437) and switchable;
  no-wrap, character and word wrapping; search (plain, regex or hex pattern) with highlighted
  hits; go to offset or percentage; follow mode; bookmarks.
- `F5` shows the code inside a compiled file: ELF, PE, Mach-O and raw x86/x86-64 disassembled
  with sections and symbols, Java class files and .NET assemblies read in process, WebAssembly
  printed as its text format. `F6` hands anything else to an installed decompiler through a
  `[[decompile]]` rule.

### Editor (`F4`)

- The file's encoding, line ending, BOM and final newline survive a round trip; the save is
  atomic, and a file changed on disk is never overwritten without a confirmation.
- Stream blocks and **column blocks**, grouped undo and redo, search and replace, go to
  `line:column`, bookmarks, auto-indent, line operations.
- `Shift+F4` edits a new file.

### Netbox: SFTP (`Alt+N`)

- A panel can sit on a server. One connection per `user@host:port` is shared by both panels
  and every job, so `F5` and `F6` copy and move up, down, between two servers or inside one,
  with the same progress and conflict handling as a local copy.
- Host keys are checked against `known_hosts`: an unknown one is shown as a fingerprint for
  you to accept, and a changed one is refused.
- Authentication through the SSH agent, a key file, or a password asked for in a masked
  prompt. Nothing is stored: `[[site]]` blocks say only *how* to authenticate.
- `F3` views a remote file through a temporary copy; `F4` edits that copy and `F2` writes it
  back, uploaded beside the original and renamed into place.

### The rest

- Six themes (commander, black, white, matrix, tron, blade) and seven interface languages
  (English, Italian, German, French, Spanish, Russian, Simplified Chinese).
- One executable: themes, keymaps and languages are compiled in. The only files on disk are
  `scmd.toml` (if you want one) and an optional plugins folder.
- Non-interactive modes for scripts and CI: `--list-themes`, `--list-commands`,
  `--list-languages`, `--check-keymap`, `--export-theme`, `--dump-config`, `--paths`.

### Not here yet

Archives, the trash (`F8` deletes permanently), the command line and the user screen, the menu
(`F9`), find files (`Alt+F7`), FTP/WebDAV/S3, the plugin host, syntax highlighting, several
open editor buffers, tabs, the diff viewer and the background job queue. Anything
unimplemented says so rather than doing nothing.

[0.0.1]: https://github.com/matteobaccan/SupremeCommander/releases/tag/v0.0.1
