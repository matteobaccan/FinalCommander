# Backlog

What is left to do, and nothing else.

A row leaves this file the day it is finished. What shipped and why it was built that way is in
`CHANGELOG.md`, the design is in `docs/spec/`, and the reasoning behind each line of it is in
the commit that wrote it — so a finished row kept here as well was a third copy that nobody
read. It had grown to a hundred and twenty-eight rows of which nineteen were the work, and a
row half of which was done read as done.

Still in the order the menus mention things, with the sections after them taken from
`docs/04-far-inventory.md`: FAR 3.x read from its own repository and compared with what we
have. **Open question** marks the rows where a decision comes before the code, and the row
says what the decision stands on.

## Panels

| Item | Where | What is left |
| --- | --- | --- |
| Columns from a plugin | `[[column_set]]` | Total Commander's content plugins: a column whose value a plugin computes (an EXIF date, a track's tags). The column grammar is there; a column API for plugins is SPEC 08's next tier. |
| Panel filter | `Ctrl+Shift+I` | Locking a filter so a changed directory keeps it (feature 1.15). |
| Mounting from the drive list | `Alt+F1` | Mounting, unmounting and ejecting a drive from the dialog, MountMan's job (feature 2.4). |

## Selection and sorting

| Item | Where | What is left |
| --- | --- | --- |
| Sort criteria chains | sort modes list, `F4` | FAR's per-sort-mode chains with their own editor (`Ins`/`Del`/`F4`/`Ctrl+↑↓`), where the chain changes with the key rather than being one list. Ours is one chain of up to three keys for every mode. |
| Selecting from a list | `Gray +` | The names of a list on the clipboard (feature 2.6). Wants the clipboard to be *readable*, which `OSC 52` is not in practice: terminals refuse the query. |

## Files

| Item | Where | What is left |
| --- | --- | --- |
| File attributes | `Ctrl+A` | FAR's *System properties* button, which hands the file to the shell's own property sheet; the NTFS compressed and encrypted bits; and a picker for the owner row, which takes a name or a number and completes neither. |
| The files themselves on the clipboard | `Ctrl+Shift+C`/`X` | No terminal can be asked to carry a file — `OSC 52` carries text. This is window-system work, and it is what `fcmd-platform` would be for. |
| Descriptions | `Ctrl+Z` | Writing one *inside* an archive, which is a rebuild of the archive rather than a write to a file. |
| Formats through a program on the machine | `Enter` on a `.rar`, `.iso`, `.cab`… | A selection extracted through the job engine, with the bar counting bytes as they land; `Test` on `Alt+F6`; a table for RARLAB's `rar`, verified on a machine that has it (`docs/05-multiarc.md` §10). Writing is not on this list: it is decided against (§9). |
| `cab` read here | `Enter` | Read through `7z` today, so not on a machine without it. `cpio` and `rpm` went the same way in 0.30.0; `cab` is the one of the three that wants a crate (`docs/05-multiarc.md` §9). |
| zstd's levels | `Shift+F1` | Every level writes zstd's fastest, because the encoder this uses implements no other: the normal and the best level are a choice for the other codecs only. |
| `F7` inside an archive | — | No format stores a directory worth keeping, so this wants a decision about what an empty directory in an archive even is. |
| An archive on a server | — | A progress bar while the copy comes down. Today a large one is simply a pause. |
| Multi-rename | `Ctrl+Shift+F6` | More than one step of undo, exporting a plan and reading one back, more than one search rule, transliteration, a dedup rule, and the `[X]` and `[=meta]` fields. |
| 7z | `Shift+F1` | Writing an encrypted archive, split volumes, and the codecs only 7-Zip ZS writes (zstd, lz4, brotli). A zip encrypted with AES wants a feature of the `zip` crate we do not build, so it says what it is rather than asking. |

## Viewer and editor

| Item | Where | What is left |
| --- | --- | --- |
| Pictures | `F3` on an image | What is left after the real pixels (feature 5.22): decoding a picture off the drawing thread, as its scaling and encoding already are; `auto` recognising Windows Terminal, `xterm -ti 340` and VS Code, which today have to be named in `viewer.pixels` because their environment does not say enough; real pixels through tmux's passthrough; kitty's and iTerm2's protocols looked at in those terminals — they are tested by their bytes, and only sixel against a real terminal (xterm). The thumbnail view and the photo-marking slideshow are their own rows below. |
| Video | `F3`, `Ctrl+Q` on a video | What is left after the frames and the playback through `ffmpeg` (feature 5.24): containers the structure pane does not read — MPEG-TS, MPEG-PS, FLV, ASF/WMV — are not recognised as films, since the running time and the codec come from that reader; stepping in the quick view (it shows the frame at a tenth and no other); a step smaller than a tenth, and a frame-by-frame one; sound, which stays out. Playing is eight frames a second and a preview: a faster one wants frames compared and only the difference sent, which none of the three protocols makes easy. |
| Following a branch | `Enter` on a `call` | The jumped-from stack shown on the screen, and a virtual address in `Alt+F8` (feature 5.16). |
| More than one file in the editor | `F4` | Several files open at once, each a screen of its own today through `F12` (feature 6.1). |
| Search and replace | `F7`, `Ctrl+F7` | Replacing inside a marked block only; the separator half of FAR's *preserve style* (`foo_bar` ↔ `fooBar`); a key that repeats *Replace next* without the dialog (feature 6.4). |
| Encodings | `Shift+F8` | Reading an open file again in another code page, as the viewer does: today the editor's list only converts what will be written (feature 6.6). |

## Command line and history

| Item | Where | What is left |
| --- | --- | --- |
| A terminal in a panel | Commands menu | One at a time. Several, as tabs of terminals, wants the pane to own a list rather than one child, which is the same shape the panel tabs already have. |
| Command prefixes | command line | `clip:` and `link:` of FAR's `farcmds` (feature 9b.9). |
| Internal commands | command line | `title`, `chcp` and the nestable `if` (feature 9b.10). |

## Histories and screens

| Item | Where | What is left |
| --- | --- | --- |
| Screen grabber: append to the clipboard | `Alt+Ins` | Wants the clipboard to be *readable*, and it is not: `OSC 52` asks, terminals refuse. |
| Screen to screen without the panels | `Ctrl+Tab` | SPEC 02 §10. The ring steps out to the panels between screens, because the screens shadow the application's mode rather than owning it. A ring that walked screen to screen wants `screens: Vec<Mode>` with the front swapped in and out, which means every `self.mode = Mode::Normal` that closes a viewer has to say which screen it is closing. Worth doing when something else needs that list anyway — the task list (`Ctrl+W`) probably will. |

## Binary viewer and hex editor

| Item | Where | What is left |
| --- | --- | --- |
| Crypt / decrypt a block | `F12` on a block | As a block operation (XOR, and a named cipher with a key), never as a promise of security — the dialog has to say what it is and what it is not. A named cipher means a crypto dependency to justify, so that part is an **open question**. |

## Tools

| Item | Where | What is left |
| --- | --- | --- |
| Find file | `Alt+F7` | From FAR's dialog: a sixth scope, one code page or a chosen set, alternate streams, the advanced page (a first-N-bytes limit, result columns in the column grammar), and a new search from the results. |
| Checksums, split and combine | Files menu | Checksums, splits and joins of something that is not on this machine, which wait on `JobFs::read_chunks` beyond the local filesystem, and the self-joining script. |
| A queue for jobs | `Alt+Shift+F5` | The queue, the lanes per device, background, pause, reordering and retries are in (0.31.0, SPEC 03 §5). A server's speed limit came in 0.34.0 (`max_kbps`). The journal that lets a copy or a move go on after a crash came in 0.41 (SPEC 03 §5a, `Alt+Shift+F6`). Left: cloning a job; journalling the jobs still waiting in the queue and the chain behind a running one, which a crash loses today; a per-job log kept on disk (feature 3.7). |
| Disk usage | `Alt+Shift+F3` | A whole tree at once rather than one folder's level, and a treemap (SPEC 06 §12). |
| Synchronise folders | `Ctrl+Shift+Y` | Saving a pair under a name (`[[sync]]`, `Ctrl+Alt+Y`) and the written report came in 0.41 (SPEC 03 §10a). Left: running a saved pair unattended — from the command line or on a schedule, without the review table — a report that is appended to rather than rewritten, and a side that is not on this machine. |
| User menu | `F2` | Reading FAR's `FarMenu.ini` as it stands, and a dialog to edit the entries. |
| File associations | Commands menu | Defaults read off the OS (xdg-mime, `LSCopyDefaultRoleHandler`, the registry), a dialog to edit the blocks, and `execute`/`extract`/`preview` entries. |
| Two files side by side | `Ctrl+Shift+D` | A unified view, a file against the copy on disk, and the first differing offset of two binaries. |

## Interface and configuration

| Item | Where | What is left |
| --- | --- | --- |
| A theme editor | `opt.colors` | A grid to pick from instead of typing, and FAR's colour *groups* — one change for every "selected" colour at once. |
| Lock the filter | `Ctrl+Alt+L` | The one of the three list behaviours that needs something new: the filter would have to outlive the list it belongs to, and no list yet asks for that — every one of them is built where it is opened. |
| Macro conditions | `Ctrl+.` | SPEC 08 §5. A condition is a named question the application already knows the answer to — is anything selected, is the cursor on a directory, is this panel a real directory or a list, is the command line empty, is this a server — so a `when` field holding a list of those needs no engine, no dependency and no language. What it does need is the *fall-through*, which is half of why FAR has it: a macro on `F5` conditioned on something being selected plays when something is, and when nothing is, `F5` copies as it always did. |
| The Lua engine | SPEC 08 §2 | **Decided against** (see *Settled* below), and so are the conditions above. far2m's macros are Lua: a dependency to justify, an API surface to draw (what a script may ask and what it may do), and the question of what a script is allowed to do when a recorded macro is already not trusted. FAR's fourteen macro areas are also finer than our five scopes; worth splitting when something needs it. |
| XLat | — | **Open question** on the key, which nothing has claimed. Transliterate a typed line between keyboard layouts, with tables, rules and flags. Niche outside Cyrillic keyboards, cheap once the tables exist. |
| More keymap profiles | `--keymap` | `tc`, `nc` and `vi` beside `far` and `mc` (feature 9.2): each wants its conflicts with this program's own keys worked out before it is written. |
| Text layout | everywhere | Combining marks and truncation that is safe for right-to-left text (feature 9.5). |
| Session restore | state file | The selection, which is not kept across a restart (feature 9.7). |
| Moving a dialog | mouse | Dragging a dialog by its frame (feature 9b.17). |

## Settings not yet in the configuration

| Item | Where | What is left |
| --- | --- | --- |
| Volumes with no mount point | `Ctrl+H` | Wants `FindFirstVolumeW`. The drive list shows what the platform can tell us otherwise. |
| The wastebasket as a panel | — | Browsing it, and putting something back from inside the program. |

## Netbox

| Item | Where | What is left |
| --- | --- | --- |
| A chain of jump hosts | SPEC 04 §6.3 | One hop is there — a `jump` field on the session, the bastion's session and a `direct-tcpip` channel to the destination inside it. `ssh -J a,b,c` is a list, and the shape generalises (the third step could itself be a jump), but a list is a different dialog from a field; worth building the day somebody has two bastions in a row. A bastion that wants a password of its own is the other half: a second secret through the pool, the prompts and the retry. |
| FTP | SPEC 04 §6 | Resuming an interrupted transfer with `REST`, active mode for a server behind an awkward firewall, and a tested path through a proxy. |
| WebDAV | SPEC 04 §6 | Digest authentication, locking, quota, and a `PROPPATCH` for the servers that do let a time be set. |
| Multipart upload | object storage | **Resuming** one that was interrupted, which needs the id and the parts to outlive the session, and sending several parts at once, which is throughput rather than correctness. |

## Dependencies

| Item | Where | What is left |
| --- | --- | --- |
| **Fewer crates, 0.40** — what is ours to write | the workspace | Decided with the author after the audit of November 2026: of the 348 crates in the Linux binary, about 45 go if six libraries become our own code, in this order. **`dirs`** (5 crates) — the config, data and cache folders by XDG, `~/Library` and `%APPDATA%`, forty lines in `fcmd-config`. **`globset`** (2) — a glob turned into the `regex` we already carry, in `fcmd-core`. **`lzma-rust2` 0.23 → 0.25** in `fcmd-arc` — not a rewrite: `sevenz-rust2` and `zip` already bring 0.25, so the binary holds the xz code twice; the API is the same. **`portable-pty`** (11, and with it the second `nix`, `bitflags` 1 and `thiserror` 1) — `openpty` and the child on `nix` 0.31 for Unix, ConPTY through `windows-sys` for Windows, in `fcmd-platform` where `unsafe` is allowed. **`fluent-bundle` + `unic-langid`** (10) — a reader for our own `.ftl` files: messages, `{ $var }`, and the plural selector with the CLDR rules of our seven languages (Russian one/few/many, Chinese none). **`clap` + `clap_mangen`** (17) — the argument parser and the man page, last because the user sees its help and errors. **`tar`** (2) is optional, the way `cpio`, `ar` and `rpm` already are. Not worth it: the hashes, `chrono`, `anyhow` and `async-trait` (other crates bring them anyway), and crypto, TLS, HTTP, SSH, image decoders, encodings and `notify`. Each step ships with the full gate and leaves `THIRD-PARTY.md` and `deny.toml` saying what left. **Done:** all of it — `dirs`, `globset`, `portable-pty` and the duplicate `lzma-rust2` before 0.40, then the `.ftl` reader (`fcmd-i18n/src/ftl.rs`, 9 crates) and the command line (`fcmd/src/cli.rs`, 17 crates) in 0.40.0, and `tar` after it (`fcmd-arc/src/tar.rs`). |
| **Fewer crates, round 2** — decided with the author on 11 November 2026 | the workspace | The audit after 0.40: the Linux binary holds 319 crates, and most of them sit under five libraries that stay — `russh` (133 with its own), `reqwest` (99), the archive codecs, `tokio`. A rewrite only counts where **nothing else in the tree uses the crate**: `sha1`/`sha2`, `thiserror` and `async-trait` come in under `russh` anyway, and `chrono` under `russh-sftp`, `suppaftp` and `trash`, so writing our own would remove nothing. What does go, in order of what it saves: **`crossterm`** (about 28) — the terminal backend in `fcmd-tui`, raw mode and key decoding on `nix` for Unix and `windows-sys` for Windows, both already here; **`suppaftp`** (15–20 of its 41 are its own) — an FTP and FTPS client on the `tokio` and `rustls` we already carry; **`notify`** (11) — the folder watcher in `fcmd-platform`: inotify, kqueue or FSEvents, ReadDirectoryChangesW; **`trash`** (about 5, `chrono` stays) — the freedesktop trash is files and folders, the other two are system calls; **`vte`** (3) — the escape-sequence parser of the terminal pane; **`md5`**, **`shell-words`**, **`anyhow`** (1 each). About 65–75 crates in all, a fifth of the binary. **Stays outside, by decision**: cryptography and TLS (`ring`, `rustls`), SSH, HTTP, `tokio`, SQLite, the compression codecs, the image decoders, the disassemblers (`iced-x86`, `yaxpeax`), `object`, `regex`, the Unicode tables, `encoding_rs`, `serde`/`toml`/`serde_json`, and the system bindings (`nix`, `libc`, `windows-sys`) — where a bug is a security hole or a damaged file, and a library many people check is worth more than one we own. |
| `reqwest` 0.13 | `fcmd-net`, `fcmd-s3` | A version bump that is not one. 0.13 renamed the TLS features — `rustls-tls-native-roots` no longer exists — and its `rustls` feature brings `aws-lc-rs`, the backend this workspace already refuses for its OpenSSL licence and the C toolchain it wants (`THIRD-PARTY.md` says why, under `russh`). The honest port is `rustls-no-provider` in both crates plus installing the `ring` provider as the process default before the first request, which changes how the program sets up TLS and wants a real server to test against rather than only the in-process fake. 0.12 is maintained and carries no advisory, so there is no hurry; `renovate.json` holds the version back and says the same thing. |

## What the others do and we do not (survey of November 2026)

Read from Directory Opus 13 (its home page's feature list), two Directory Opus plugins by
PolarGoose (TabLabelizer, CSV viewer), TheFile.Ninja (public beta), superfile (its README and the
docs in its repository), and the feature columns of Wikipedia's *Comparison of file managers*.
Only what we do **not** do is here; a row that repeats a gap already listed above says so and
points to it rather than being a second copy. *Out of scope* marks what a program in a terminal
cannot be, kept so the question is not asked again.

| Item | Seen in | What it is | Note |
| --- | --- | --- | --- |
| Tags, labels, ratings | Opus, File Ninja, Tabbles | Free tags, coloured labels, star ratings and status icons on files, as columns, filters and sort keys | **Parked by the author (11 November 2026): not now.** The study, for when it comes back: extended attributes in Dolphin's (`user.xdg.tags`, `user.baloo.rating`) and the Finder's formats on Linux and macOS, a `descript.ion`-like sidecar on Windows and wherever xattrs are missing, optionally XMP `Rating` inside pictures; no database of our own, which breaks the moment a file moves outside the program. About a release of work. |
| Boolean search | Wikipedia | Nested and/or/not over conditions | The conditions of `Alt+F7` are all *and*-ed today |
| Metadata panel | superfile, Opus | A side panel showing a file's metadata (EXIF and the like, `exiftool` where present) as the cursor moves | `F11` shows it in the viewer today |
| Editing metadata | Opus | Writing EXIF/ID3/document fields | Read-only today |
| What the AI could do next | — | Follow-ups, streaming, cost, words to a command, a commit message, explain this, rename and rewrite proposals, and the larger ideas | The study is `docs/06-ai-study.md`; the author chooses from it |
| Verifying a signature | — | Saying whether an Authenticode or a Mach-O signature is valid, not only who it names | Needs the machine's trust store and the hash of the image; 0.38.0 reads and names, and says "not verified" |
| HEIC/HEIF and TIFF/RAW EXIF | Opus | The EXIF of a phone's HEIC and a camera's raw file | The TIFF reader is there; HEIF keeps EXIF in an `iloc` item |
| Image conversion | Opus | Converting and resizing images from the panel | Through a program on the machine, as `7z` is |
| Photo marking, slideshow | Opus, Wikipedia | Step through pictures full screen and mark the keepers | The real pixels it waited for are in (feature 5.22); this is the next step |
| Thumbnail view | Wikipedia, Opus | A panel view of pictures instead of names | The real pixels are in; wants many small placements at once, which the backend's list already allows |
| Export or print a listing | Opus, Wikipedia | A folder's listing written out as text, CSV or HTML, or printed | Panelize goes the other way |
| Tab labels from the path | TabLabelizer | A tab named after its shortened full path, with a maximum length and aliases | Today the folder's name, or one given by hand |
| Cut and paste of files | superfile, Explorer | `Ctrl+X`/`Ctrl+C` on files, `Ctrl+V` in another folder, inside the program | Not the system clipboard (*The files themselves on the clipboard* above) |
| Frecency jump | superfile (zoxide), broot | A list ordered by how often and how lately a folder was used, typed into | Feature 2.3 |
| More than two panels | superfile, DOS Navigator | Splitting into a third and fourth panel | M4 |
| Sidebar | superfile, Opus | Pinned folders and drives in a column beside the panels | The hotlist and `Alt+F1` hold the same things |
| `vi` keys | superfile, vifm | A keymap profile | *More keymap profiles* above |
| Scripting with dialogs | Opus, File Ninja | A script that draws its own dialogs, over a large API | *The Lua engine* above |
| AI writing the automation | File Ninja | Describing a task and getting a script back to read, dry-run and keep | Our AI answers questions; this writes a plugin |
| Scheduler | File Ninja | Jobs on a cron schedule, catching up on missed runs | |
| Versioned backup | File Ninja | Content-addressed snapshots of folders, browsed and restored like folders | |
| Watch and act | File Ninja | Running something when a folder changes | The watcher refreshes panels; it triggers nothing |
| Peer-to-peer transfer | File Ninja | Magic Wormhole: send a file to another machine with a code | |
| SMB, NFS, AFP | Wikipedia, far2l | Native clients instead of the system's mounts | |
| Encrypting a file | Wikipedia | One file encrypted under a password, not an archive | The vault's cipher is in the tree |
| Renaming a busy file | Wikipedia | Windows: a rename or delete done at the next boot | |
| Pasting the clipboard as a file | Wikipedia | Text or a picture on the clipboard written to a new file | Wants a readable clipboard |
| ACL editor | Wikipedia | Access-control lists, not only the mode bits | Feature list M3 |
| Explorer replacement, floating toolbars, global hotkeys | Opus | Taking over the desktop's file windows; keys that work outside the program | Out of scope: a terminal program |

## Settled, so that nobody proposes them again

Decisions, not gaps. Each of these was asked and answered; the answer is no, or it is "another
way", and the reason is here so it does not have to be found again.

* **The Lua engine and conditions on macros** — no (decided 11 November 2026). Macros stay
  recordings played back; a plugin is the place for logic, and `[command.when]` is where a
  command says where it makes sense.
* **A cipher for the hex editor's blocks** — no (decided 11 November 2026). A named cipher
  would be a crypto dependency for something nobody should mistake for protection; XOR on a
  block is the most this program will offer, and only if somebody asks.

* **Dropping files between the panels does not move or copy by itself** — it opens the copy
  dialog with that destination. The Explorer's rule (move within a drive, copy across one) is
  the same gesture doing two different things depending on where the files are, one of which
  deletes the original.
* **Editing a cell in table mode** — a viewer that writes is an editor; `F4` and the hex pane
  are where writing lives.
* **Selecting text in the viewer with the keyboard** — answered another way, by the screen
  grabber (`Alt+Ins`, SPEC 06 §11b), which copies a rectangle of *any* screen.
* **A click outside an open dialog** does nothing, on purpose: it must not move a cursor
  nobody can see.
* **Parquet in table mode** — Parquet means a columnar stack larger than this program.
  (SQLite, the other half of this line, is a container of its own since 0.33.0: SPEC 04 §4b.)
* **Netware's NLM/DSK/LAN in the structure pane** — no living platform behind them.
* **An image bot on the repository** — every picture is generated or the source of something
  generated, and `logo.svg` is animated and meant to be read as text in a pull request
  (`assets/README.md`, `.imgbotconfig`).
* **Short (8.3) names, `Ctrl+N`** — no equivalent on Unix, and dying on Windows.
* **A locale's collation for the sort** — the names are sorted with the Latin accents folded
  and the case as `panel.sort_case` says (SPEC 02 §4), by a table of our own. A real collator
  (`icu_collator` with CLDR's data) wants valid text, megabytes in the binary and a
  dependency, and what it would add is each language's own alphabet — `ö` after `z` in
  Swedish — which nobody has asked for.
* **NTFS alternate data streams** — out of the core; a provider may expose them as entries if
  it wants to.
* **SUBST drives, VHD attach and detach, USB safe removal, CD eject, volume labels** — the
  drive list shows what the platform can tell us; the rest is a platform tool's job.
* **An access-rights policy on copy (default / copy / inherit)** — Unix permissions and ACLs
  are a different model; `operations.preserve` lists what we carry and the rest is the
  destination's.
* **Elevation ("request administrator rights")** — a privileged helper is a security design,
  not a setting. Out of scope until someone needs it badly enough to design it.
* **The OS context menu (`emenu`) and drag and drop with the shell** — a plugin's job on each
  platform, not the core's.
* **OEM (FAR 1.x) plugin support, the console icon, ClearType redraw** — not applicable.
* **12-hour time and `%USERDOMAIN%`-style title variables** — the date format follows the
  locale; the title shows the path and the version.
* **`Ctrl+F12`** opens the sort list, which is FAR's own behaviour; reversing moved to
  `Ctrl+Shift+F12`.
* **`fish://` as a scheme of its own** — mc's filesystem over a plain ssh shell. The need
  behind it is met another way since 0.35.0: an `sftp://` session whose server refuses the
  SFTP subsystem falls back to the shell by itself (SPEC 04 §6.5b): POSIX commands, and `ls`
  read under `LC_ALL=C` so its output has one locale. A second scheme would be a second name
  for the same sessions (feature 4.5d).
* **Printing WebAssembly in process** — `wasmprinter` was 7.3% of the binary for one
  feature. `Shift+F6` on a `.wasm` runs `wasm2wat` like any other `[[decompile]]` rule.
* **Writing an archive through a program on the machine** — a table may list and extract,
  never create, add or delete (`docs/05-multiarc.md` §9).
* **`Ctrl+M`** — FAR's *restore selection* is `Ctrl+Shift+M` here, because a terminal
  delivers `Ctrl+M` as `Enter`. This one will not change.
