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
| Panel filter | `Ctrl+Shift+I` | A dialog for the size, date and attribute conditions the filter grammar already has, and named filter sets to pick from (feature 1.15). |
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
| Table mode | `F4` in the viewer | Sorting the rows by a column and filtering them (feature 5.18). |
| Pictures | `F3` on an image | The kitty, iTerm2 and sixel protocols for real pixels where the terminal has them; half blocks stay the fallback (feature 5.22). |
| Following a branch | `Enter` on a `call` | The jumped-from stack shown on the screen, and a virtual address in `Alt+F8` (feature 5.16). |
| More than one file in the editor | `F4` | Several files open at once, each a screen of its own today through `F12` (feature 6.1). |
| Undo | `Ctrl+Z` | Grouping a burst of typing into one step, and a depth that can be set (feature 6.3). |
| Search and replace | `F7`, `Ctrl+F7` | Whole word, capture references in the replacement, FAR's *preserve style*, and a list of every match (feature 6.4). |
| Encodings | `Shift+F8` | Explicit commands to convert the end of line, the BOM and the code page (feature 6.6). |
| A file changed underneath | `F2` | A diff of what changed on disk, before choosing to overwrite or reload (feature 6.12). |

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
| Find file | `Alt+F7` | From FAR's dialog: a sixth scope, one code page or a chosen set, fuzzy matching, alternate streams, the advanced page (a first-N-bytes limit, result columns in the column grammar), saving and loading a search, and a new search from the results. |
| Checksums, split and combine | Files menu | Checksums, splits and joins of something that is not on this machine, which wait on `JobFs::read_chunks` beyond the local filesystem; splitting by a count of pieces, and the self-joining script. |
| A queue for jobs | `Alt+Shift+F5` | The queue, the lanes per device, background, pause, reordering and retries are in (0.31.0, SPEC 03 §5). Left: cloning a job, a bandwidth cap for a server's lane, and the on-disk journal that would let a copy resume after a crash (feature 3.7, 11.6). |
| Disk usage | `Alt+Shift+F3` | A whole tree at once rather than one folder's level, and a treemap (SPEC 06 §12). |
| Synchronise folders | `Ctrl+Shift+Y` | A saved pair of directories as a named job, and a written report. |
| User menu | `F2` | Reading FAR's `FarMenu.ini` as it stands, and a dialog to edit the entries. |
| File associations | Commands menu | Defaults read off the OS (xdg-mime, `LSCopyDefaultRoleHandler`, the registry), a dialog to edit the blocks, and `execute`/`extract`/`preview` entries. |
| Two files side by side | `Ctrl+Shift+D` | A unified view, a file against the copy on disk, and the first differing offset of two binaries. |

## Interface and configuration

| Item | Where | What is left |
| --- | --- | --- |
| A theme editor | `opt.colors` | A grid to pick from instead of typing, and FAR's colour *groups* — one change for every "selected" colour at once. |
| Lock the filter | `Ctrl+Alt+L` | The one of the three list behaviours that needs something new: the filter would have to outlive the list it belongs to, and no list yet asks for that — every one of them is built where it is opened. |
| Sorting collation | System settings | **Open question.** FAR splits what we fold into one `natural_sort`: collation (ordinal / invariant / linguistic), digits-as-numbers, case sensitivity. The ground it stands on: digits-as-numbers is `panel.natural_sort` and works; case sensitivity *exists in the comparison already* but is not settable — `Sort::case_sensitive` starts from the platform (sensitive on Linux, not on Windows and macOS); collation does not exist at all, because names are compared on their **UTF-8 bytes** on purpose (no name is validated to be sorted, and two names that are not valid UTF-8 do not collapse onto the same replacement character). What that costs, visibly, is one thing: `Éxtra.txt` sorts *after* `zorro.txt`, because `C3 A9` comes after `7A`. Two roads: (a) two settings and no dependency — `panel.sort_case = auto\|yes\|no`, and folding the Latin accents in the one branch that already decodes, documented as "the Latin accents, not a locale"; or (b) a real collator (`icu_collator` with CLDR data), which wants valid text, megabytes in the binary, and an entry in `THIRD-PARTY.md`. |
| Macro conditions | `Ctrl+.` | SPEC 08 §5. A condition is a named question the application already knows the answer to — is anything selected, is the cursor on a directory, is this panel a real directory or a list, is the command line empty, is this a server — so a `when` field holding a list of those needs no engine, no dependency and no language. What it does need is the *fall-through*, which is half of why FAR has it: a macro on `F5` conditioned on something being selected plays when something is, and when nothing is, `F5` copies as it always did. |
| The Lua engine | SPEC 08 §2 | **Open question**, and not the same row as the conditions above. far2m's macros are Lua: a dependency to justify, an API surface to draw (what a script may ask and what it may do), and the question of what a script is allowed to do when a recorded macro is already not trusted. FAR's fourteen macro areas are also finer than our five scopes; worth splitting when something needs it. |
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
| Plugins configuration | Options menu | SPEC 08: there is nothing to configure until plugins load from disk. |

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
| `reqwest` 0.13 | `fcmd-net`, `fcmd-s3` | A version bump that is not one. 0.13 renamed the TLS features — `rustls-tls-native-roots` no longer exists — and its `rustls` feature brings `aws-lc-rs`, the backend this workspace already refuses for its OpenSSL licence and the C toolchain it wants (`THIRD-PARTY.md` says why, under `russh`). The honest port is `rustls-no-provider` in both crates plus installing the `ring` provider as the process default before the first request, which changes how the program sets up TLS and wants a real server to test against rather than only the in-process fake. 0.12 is maintained and carries no advisory, so there is no hurry; `renovate.json` holds the version back and says the same thing. |

## Settled, so that nobody proposes them again

Decisions, not gaps. Each of these was asked and answered; the answer is no, or it is "another
way", and the reason is here so it does not have to be found again.

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
* **Parquet and SQLite in table mode** — Parquet means a columnar stack larger than this
  program; SQLite is several tables and a query language, which is a plugin with a panel of
  its own.
* **Netware's NLM/DSK/LAN in the structure pane** — no living platform behind them.
* **An image bot on the repository** — every picture is generated or the source of something
  generated, and `logo.svg` is animated and meant to be read as text in a pull request
  (`assets/README.md`, `.imgbotconfig`).
* **Short (8.3) names, `Ctrl+N`** — no equivalent on Unix, and dying on Windows.
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
* **`fish://`** — a filesystem over a plain ssh shell, mc's answer for a host without sftp.
  `sftp://` covers the hosts people meet, and the protocol is shell snippets echoed into a
  remote shell and `ls` output parsed back in an unknown locale (feature 4.5d).
* **Printing WebAssembly in process** — `wasmprinter` was 7.3% of the binary for one
  feature. `Shift+F6` on a `.wasm` runs `wasm2wat` like any other `[[decompile]]` rule.
* **Writing an archive through a program on the machine** — a table may list and extract,
  never create, add or delete (`docs/05-multiarc.md` §9).
* **`Ctrl+M`** — FAR's *restore selection* is `Ctrl+Shift+M` here, because a terminal
  delivers `Ctrl+M` as `Enter`. This one will not change.
