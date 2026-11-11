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

## Parity after 0.41

"Do we do everything the other commanders do?" — **no: about three quarters of it.** For
0.41.0 the sources of the others were read rather than their brochures: FAR Manager, far2l,
Midnight Commander, Double Commander, Krusader, muCommander, nnn, ranger, lf, yazi, broot,
vifm, superfile and xplr from their repositories; Total Commander from the 394 internal
commands Double Commander records for compatibility; Directory Opus and TheFile.Ninja from
their home pages; Commander One from its Wikipedia article. The sites of Total Commander,
Commander One, ForkLift, Marta and Krusader could not be reached from where this was read, so
ForkLift and Marta appear below only where a feature is one they are known for. What was read
from each, and the count per program, is in `docs/02-features.md` §12: 1038 groups of features,
706 there, 92 partly, 240 not — 72% counting a part as half, 74% against the seven orthodox
commanders alone, from 55% (xplr, which is a scripting host first) to 83%.

The quarter that is missing is mostly the same dozen things in program after program. Every
gap is below, a gap already written further down this file included, with a pointer instead
of a second description. **Who** is who has it. **Work** is `S` (days, inside one crate), `M`
(a week or two, or a new dialog and a spec section) or `L` (a subsystem, or a dependency to
argue for). **Rules** is whether `AGENTS.md` and *Settled* below allow it: no GUI toolkit, no
GPL dependency, a dependency only with a justification, no Lua and no macro conditions, and
anything that is another program's job done by running that program (SPEC 02 §14: found on
the `PATH`, listed in the external programs panel, named when it is missing).

Three findings that are not gaps against anybody:

* **`app.palette` is declared and not built.** The command is in the vocabulary, the `far`
  keymap puts it on `Ctrl+Shift+P`, SPEC 07 and SPEC 08 name it, and the application answers
  that it is not implemented. It is the only command of the vocabulary with no code behind it.
  `S`: the list-that-narrows of *All settings* over `Command::all()` and its sentences.
* **The feature table was behind the code** in both directions, and is corrected: `4.5b` said
  `~` for a row whose SMB, NFS, MTP/ADB and rclone halves do not exist at all; `2.10` said `✓`
  with drag and drop with the desktop in it, which is decided against; `5.8`, `8.8` and `9.6`
  said "not there" for things that partly are; and six things the program does had no row.
* **The two open issues from users are both answered in released versions** and still open on
  GitHub: #38 (`Shift+↑`/`Shift+↓` select, bound in `far.toml`) and #29 (`Ctrl+C` on the
  command line copies it); `CHANGELOG.md` names both. They want closing, not work.

### Panels and navigation

| Item | Who | Work | Rules | Note |
| --- | --- | --- | --- | --- |
| Cut, copy and paste of files inside the program | Total Commander, Double Commander, Krusader, muCommander, ranger, lf, yazi, nnn, vifm, superfile | S | yes | *Cut and paste of files* below. A list of sources remembered with a verb; pasting is the copy or move dialog with them. The most common thing we lack |
| Marks that survive a change of folder | nnn, ranger, lf, yazi, broot, xplr, Double Commander (stash) | M | yes | Feature 2.7. The temporary panel is half of it; the other half is the marks staying when the panel moves |
| Icons | yazi, lf, nnn, superfile, broot | S | yes | Feature 1.22. Glyphs of a Nerd Font by kind and extension, off by default; a table, no dependency |
| Stacked layout | Total Commander, Double Commander, Midnight Commander, Krusader, muCommander | S–M | yes | Feature 1.2: one panel above the other |
| Thumbnail view | Total Commander, Double Commander, Krusader, Directory Opus, Commander One | M | yes | *Thumbnail view* below; the real pixels it waited for are in |
| Miller columns | ranger, lf, yazi, vifm, TheFile.Ninja | M | yes, as a third layout | **Open question**: it is the other school of file manager, and the quick view panel is our answer to its preview column |
| `vi` keys with counts, a visual mode and registers | vifm, ranger, lf, yazi, nnn, superfile | M–L | yes | *More keymap profiles* below. The profile is a file; counts and the visual mode are new machinery in the key reader |
| `tc` keymap profile | Total Commander, Double Commander, Krusader's own profile for it | S–M | yes | *More keymap profiles* below |
| Synchronised browsing | Total Commander, Double Commander, Krusader, ForkLift | S | yes | Entering `x` on one side enters `x` on the other while it is on; the natural companion of comparing two trees |
| Tabs: reopen a closed one, close the duplicates or the others, sets kept under a name | Total Commander, Double Commander, Krusader, muCommander | S | yes | The state file already holds the tabs |
| Sessions under a name | vifm, nnn, Krusader | S–M | yes | One session is restored today |
| The files themselves on the desktop's clipboard | Total Commander, Double Commander, Krusader, muCommander, FAR | L | only as programs (`wl-copy`, `xclip`, `pbcopy`, PowerShell) | *The files themselves on the clipboard* below |
| The selection to a file and back; the listing written out or printed | Total Commander, Double Commander, Directory Opus | S | yes | *Export or print a listing* below |
| Showing only the selected rows | Total Commander, ranger | S | yes | A filter whose condition is "selected" |
| Entries per folder as a column; sort by permissions, by group, at random; the cursor to the next selected row | lf, broot, ranger, muCommander, yazi, Total Commander | S each | yes | Small, and each is one program's habit |
| A tree with the files in it, folding in place | broot, vifm | M | yes | The tree panel holds folders; the branch view is flat |
| A standing list of names never shown | Total Commander, Double Commander, lf | S | yes | A named filter that is always on |
| Tags, labels, ratings | Directory Opus, TheFile.Ninja, ranger, lf, ForkLift | M | parked by the author | *Tags, labels, ratings* below |
| A sidebar; more than two panels | superfile, Directory Opus, ForkLift, broot | M | yes | *Sidebar* and *More than two panels* below |

### File operations

| Item | Who | Work | Rules | Note |
| --- | --- | --- | --- | --- |
| Elevated operations | Krusader, Commander One, Double Commander, FAR, far2l, nnn | L | **no** — *Settled*: a privileged helper is a security design | Feature 3.19. The one large thing six of them have and we have decided not to |
| Renaming many files by editing their names in the editor | ranger, vifm, nnn, yazi, Total Commander | S | yes | The names into our own editor, one per line; on save the lines become the plan the multi-rename tool already previews and undoes |
| More digests: SHA-512, SHA-3, BLAKE | Krusader, Double Commander, Total Commander, muCommander | S | SHA-512 yes (`sha2` is in the tree); the others are a dependency each | |
| Opening the desktop's file manager or terminal on the item | muCommander, Total Commander, ForkLift, Marta | S | yes, as programs | Feature 8.8 |
| A log of every operation in a file | Total Commander, Double Commander | S | yes | With the per-job log of *A queue for jobs* below |
| Changing where a symbolic link points | Midnight Commander, ranger | S | yes | A row in the link dialog |
| Extended flags (`chattr`) and access-control lists | Midnight Commander, Total Commander | M | flags yes (an `ioctl` in `fcmd-platform`); ACLs are *Settled* as another model | *ACL editor* below |
| Merging in the comparison | Midnight Commander | M | yes | *Two files side by side* below is the place |
| Several files made at once from a list | yazi, nnn | S | yes | `F7` already takes a nested path; a list is the same dialog |
| Printing | Total Commander, Krusader, FAR, Directory Opus | S | yes, through `lp` | Feature 7.9 |
| Encoding UUE, XXE, MIME; sending by mail; `gpg` on a file | Total Commander, muCommander, Midnight Commander, nnn | S each | `gpg` and mail as programs | One program each: user-menu material rather than core |
| Pictures converted and resized, metadata written | Directory Opus, nnn | M | as a program | *Image conversion* and *Editing metadata* below |

### Archives, servers and other filesystems

| Item | Who | Work | Rules | Note |
| --- | --- | --- | --- | --- |
| SMB | far2l, muCommander, Krusader, Double Commander, Commander One, ForkLift | L in process; M through `smbclient` | a pure-Rust client is a dependency to argue for; the program is allowed | *SMB, NFS, AFP* below. The widest gap on the network side, and the table used to hide it |
| Phones: MTP and ADB | far2l, muCommander, Commander One, nnn | M through `adb`; MTP is L | `adb` as a program | Feature 4.5b |
| Google Drive, Dropbox, OneDrive | muCommander, Commander One, ForkLift, Double Commander | L each in process; M once through `rclone` | `rclone` as a program is the route the rules allow: one bridge, every cloud it knows | Feature 4.5b names the bridge |
| A filesystem from a plugin | FAR, Total Commander, Double Commander, yazi | M–L | yes: a program speaking list/read/write on its standard streams, the way `[[archiver]]` tables already do for archives | Feature 4.11 |
| Passwords in the system's keyring | Krusader, Commander One, ForkLift, muCommander | S–M | as programs (`secret-tool`, `security`); a crate is a dependency | Feature 4.6. The vault holds them today |
| Mounting, unmounting, ejecting | Krusader, vifm, yazi, nnn, Total Commander, Double Commander | M | as programs (`udisksctl`, `diskutil`) | *Mounting from the drive list* below; *Settled* leaves USB removal and CD eject to the platform's tool |
| NFS without a mount | far2l, muCommander | L | a dependency | Low: the system mounts it |
| The Windows registry | FAR, muCommander | M | yes, Windows only | Feature 4.9 |
| Torrent files, patches, installed packages as folders | Midnight Commander, Double Commander | S, M, S | torrent in process (a small parser); packages through `dpkg`/`rpm` | One program's corner each |
| Servers found on the local network | muCommander, FAR, Total Commander | M | mDNS is a dependency | |
| Containers as panels; virtual-machine disks | far2l (a plugin), muCommander | M | `docker` as a program | Plugin material |
| FTP resume and active mode | Total Commander | S | yes | *FTP* below |

### Viewer and editor

| Item | Who | Work | Rules | Note |
| --- | --- | --- | --- | --- |
| Previews from rules: a program per mask, with a timeout | yazi, ranger, lf, nnn, vifm, broot; Total Commander and Double Commander as lister plugins | M | yes — it is SPEC 05 §1.7, written and not built | Feature 5.8. `[[decompile]]` is the same shape; this is the quick view asking it |
| Fonts and SVG drawn; HTML and RTF laid out | yazi, Total Commander, Double Commander | S as programs (`rsvg-convert`, `magick`); M in process | as programs | Would fall out of the row above |
| Sound | Total Commander, Double Commander, nnn | S | **no** — decided with the video | |
| A right margin in the editor; a ruler in the viewer | Midnight Commander | S | yes | The editor's line numbers are in (`Alt+N`) |
| A paragraph reflowed, wrap while typing, a block aligned; the case of a block; boxes drawn | Midnight Commander, FAR, far2l | S each | yes | FAR ships them as plugins (`align`, `autowrap`, `editcase`, `drawline`) |
| The syntax chosen by hand | Midnight Commander, far2l | S | yes | For the file whose name says nothing |
| Search while typing; the next occurrence of the word under the cursor | far2l | S | yes | |
| Git's changes in the editor's margin | far2l | M | yes | The git panel and blame are in |
| Spell check | Midnight Commander | M | as a program (`aspell`, `hunspell`) | |
| Name, path and date inserted | Midnight Commander, FAR | S | yes | Feature 6.8 |
| A scratchpad kept between runs | far2l | S | yes | |

### Tools, the shell and extending it

| Item | Who | Work | Rules | Note |
| --- | --- | --- | --- | --- |
| Chooser mode: print the chosen files and end | nnn, ranger, lf, yazi, vifm, broot, xplr | S | yes | `--choose FILE` beside `-P`. It is what lets an editor or a desktop portal use the program as its file dialog; seven of the eight terminal managers read have it |
| A list of paths on standard input as a panel | nnn, vifm | S | yes | The other half of the row above: `fd … \| fcmd -` |
| Hooks: on entering a folder, on a selection, when a job ends, on leaving | lf, yazi, xplr, vifm, ranger, TheFile.Ninja | M | yes — a `[[hook]]` naming a plugin command, which is a program; no language | Feature 10.6 |
| A running instance told what to do from outside | vifm, lf, yazi, xplr, nnn | M | yes, a local socket; nothing over the network | With the row above it is what the others' "scripting" is mostly used for |
| A language inside the program | FAR, far2m, yazi, xplr, Marta, TheFile.Ninja (Lua); ranger, far2l (Python); vifm; Directory Opus; Double Commander | L | **no** — *Settled*: no Lua. A plugin is a program | Features 7.8 and 10.2. Eleven of them have one; the two rows above and WASM (10.3) are the answer we chose |
| The command palette | Total Commander (command browser), Double Commander, Marta, ForkLift | S | yes | Declared and not built: see the top of this section |
| A dialog that binds keys | Total Commander, Double Commander, Krusader, muCommander, Commander One | M | yes | The keymap is a file; `F4` on a row of the keys chapter would be the place |
| Plugins installed from a list | yazi, nnn, Krusader, far2l | M | yes, on an explicit action only (feature 9.9) | Feature 10.5 |
| The command line without the screen: a copy, a saved synchronisation | TheFile.Ninja, yazi (`ya`) | M | yes | Feature 10.7, *Synchronise folders* below |
| "Open with" from the desktop's own table | ranger, yazi, muCommander, far2l, nnn | M | as programs (`xdg-mime`, `gio`) | *File associations* below |
| A notice when a long job ends | nnn | S | yes: the bell, or `OSC 9` | |
| Suspending with `Ctrl+Z`; the terminal locked when idle | Midnight Commander, yazi, ranger, vifm, lf; nnn | S | yes | `Ctrl+O` and the terminal pane answer the first |
| A mode for screen readers | nnn | M | yes | Feature 9.6 |
| A scheduler, versioned backup, transfer by a code | TheFile.Ninja | L | the first is `cron`'s job | *Scheduler* and its neighbours below |
| Toolbars and button bars, mouse gestures, the desktop's context menu, drag and drop with the desktop, replacing the system's file manager | Total Commander, Double Commander, Krusader, muCommander, Directory Opus | — | **no** — a window system's, and *Settled* | Left out of the count |

### What we have and none of them has

Checked against the same sources: none of the programs above does these out of the box.
(Where one comes close it is said.)

* **A disassembler in the viewer** — x86, x86-64 and AArch64, with symbols, following a call
  and coming back — and **an assembler in the hex editor**, one instruction into the patch list.
* **Decompilation**: JVM bytecode and .NET IL listed in process, the rest by rule through the
  tools on the machine.
* **The structure pane**: what a file is, field by field — PE, ELF, Mach-O, NE, LE, LX, TE,
  signatures, version resources, and the metadata of media, PDF and office files. far2l's
  `inside` does a part of the ELF half.
* **A hex editor with a patch list and byte blocks** that go through the job engine; Midnight
  Commander and far2l edit bytes, without either.
* **Table mode**: CSV, TSV, JSONL and DBF with its memo files as a table that sorts, filters
  and marks rows.
* **SQLite databases, JSON documents and mailboxes as folders**, and **IMAP** read as a panel.
  Midnight Commander reads an mbox; nobody reads the other three without a plugin.
* **A copy that goes on after a crash**, from a journal.
* **Raw devices** opened in the viewer and the hex editor.
* **Base64, PEM and `data:` decoded in the viewer.**
* **`.editorconfig`** honoured by the built-in editor.
* **A git panel**: stage, unstage, diff, commit, log, branches and blame. broot, yazi and
  TheFile.Ninja show a state; far2l marks the editor's margin.
* **Tunnels and a jump host** from the connection list.
* **AI with its guard rails**: questions with the secrets masked first, a switch that keeps
  everything on this machine, words to a rename, to a search and to a rewritten block shown as
  a diff, a failed command explained, a commit message drafted. TheFile.Ninja has an assistant
  that writes scripts; none masks what is sent.
* **Video played in the terminal** through `ffmpeg`; yazi shows one thumbnail.
* **FAR's own files understood**: `FarMenu.ini` read as it stands, and archiver tables written
  in MultiArc's own keys.
* **Picture metadata taken out.**
* **The help generated from the keymap in force**, and a manual page a test compares with the
  options.
* **All of it in one executable that is the same on Linux, macOS and Windows.** FAR is
  Windows, far2l and Midnight Commander are Unix, Total Commander is Windows; the ones that
  run everywhere are Double Commander and muCommander, with a GUI toolkit or a JVM under them.

## Panels

| Item | Where | What is left |
| --- | --- | --- |
| Columns from a plugin | `[[column_set]]` | A plugin's `[[column]]` is drawn (SPEC 08 §1d). Left: sorting by one — the sort keys are a closed list the commands, the keymaps and the sort menu each enumerate — searching by one in `Alt+F7`, a column for the rows of a list panel, an archive or a server, and answers that outlive the session. |
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
| Formats through a program on the machine | `Enter` on a `.rar`, `.iso`, `.cab`… | A bar that moves while a program tests or lists (it prints nothing useful until it ends); a table for RARLAB's `rar`, verified on a machine that has it (`docs/05-multiarc.md` §10) — a `.rar` is read, extracted and tested through `7z` today, which is checked against a real RAR 5 archive; the `unrar` most Linux machines have is unrar-free, whose listing is another layout and whose test tests nothing, so it is not driven. Writing is not on this list: it is decided against (§9). |
| `cab`, the rest | `Enter` | Stored and MSZIP cabinets are read here (`fcmd-arc::cab`). LZX and Quantum are not: each is a compressor of its own to write and to test against real cabinets, and until then such a cabinet goes to `7z`, so not on a machine without it. Nor is a file that crosses from one cabinet of a set into the next put together. |
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
| Find file | `Alt+F7` | An absolute date range and the other three timestamps (an age in days of the last write is there, SPEC 06 §1f), and the attributes. From FAR's dialog: a sixth scope, one code page or a chosen set, alternate streams, the advanced page (a first-N-bytes limit, result columns in the column grammar), and a new search from the results. |
| Checksums, split and combine | Files menu | Checksums, splits and joins of something that is not on this machine, which wait on `JobFs::read_chunks` beyond the local filesystem, and the self-joining script. |
| A queue for jobs | `Alt+Shift+F5` | The queue, the lanes per device, background, pause, reordering and retries are in (0.31.0, SPEC 03 §5). A server's speed limit came in 0.34.0 (`max_kbps`). The journal that lets a copy or a move go on after a crash came in 0.41 (SPEC 03 §5a, `Alt+Shift+F6`). Left: cloning a job; journalling the jobs still waiting in the queue and the chain behind a running one, which a crash loses today; a per-job log kept on disk (feature 3.7). |
| Disk usage | `Alt+Shift+F3` | The whole tree as a map came in 0.45 (`Tab` in the list, SPEC 06 §12). Left: deleting from the map, a walk that goes on after the screen is closed, and a rectangle sized by what a file takes on the disk rather than by its length. |
| Synchronise folders | `Ctrl+Shift+Y` | Saving a pair under a name (`[[sync]]`, `Ctrl+Alt+Y`) and the written report came in 0.41 (SPEC 03 §10a). Left: running a saved pair unattended — from the command line or on a schedule, without the review table — a report that is appended to rather than rewritten, and a side that is not on this machine. |
| User menu | `F2` | A dialog to edit the entries. From FAR's `FarMenu.ini` (SPEC 06 §4a): the symbols with no counterpart — the drive, the other panel's selection, a file name of the left or right panel — the list-file modifiers, and a function key as a hot key. |
| File associations | Commands menu | Defaults read off the OS (xdg-mime, `LSCopyDefaultRoleHandler`, the registry), a dialog to edit the blocks, and `execute`/`extract`/`preview` entries. |
| Two files side by side | `Ctrl+Shift+D` | Two binaries byte against byte came in 0.45 (SPEC 05 §3a). The unified layout (`U`) and the comparison saved as a patch (`Shift+F2`) came with it (§3b). Left: a patch of a whole pair of folders, and how many lines of context it carries as a setting; for binaries, a comparison that lines the two up around an inserted byte, and one of files that are not on this machine. |

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
| What the AI could do next | — | What is left of the study: ask the table (B3), explain a difference (B4), describe a picture (B5), words to a search (C2) or a filter (C6), and the larger ideas (D) | The study is `docs/06-ai-study.md`; follow-ups, streaming, cost, words to a command, the commit message, explain this, redaction (A4), the local-only switch (A5), words to a rename (C3), the rewrite of a block (C5) and explaining a failed command (B2) are in |
| Verifying a signature | — | Saying whether an Authenticode or a Mach-O signature is valid, not only who it names | Needs the machine's trust store and the hash of the image; 0.38.0 reads and names, and says "not verified" |
| HEIC/HEIF and TIFF/RAW EXIF | Opus | The EXIF of a phone's HEIC and a camera's raw file | The TIFF reader is there; HEIF keeps EXIF in an `iloc` item |
| Image conversion | Opus | Converting and resizing images from the panel | Through a program on the machine, as `7z` is |
| Photo marking, slideshow | Opus, Wikipedia | Step through pictures full screen and mark the keepers | The real pixels it waited for are in (feature 5.22); this is the next step |
| Thumbnail view | Wikipedia, Opus | A panel view of pictures instead of names | The real pixels are in; wants many small placements at once, which the backend's list already allows |
| Export or print a listing | Opus, Wikipedia | A folder's listing written out as text, CSV or HTML, or printed | Panelize goes the other way |
| Tab labels from the path | TabLabelizer | A tab named after its shortened full path, with a maximum length and aliases | Today the folder's name, or one given by hand |
| Cut and paste of files | superfile, Explorer | `Ctrl+X`/`Ctrl+C` on files, `Ctrl+V` in another folder, inside the program | Not the system clipboard (*The files themselves on the clipboard* above) |
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
