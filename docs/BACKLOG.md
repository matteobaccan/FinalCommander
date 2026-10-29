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
| Info panel | `Ctrl+L` | Mount options and a git summary. |
| Tabs per panel | `Ctrl+Alt+T`/`W`/`N`/`P`/`M`/`K` | Reordering a tab within its row, and a list of them to pick from. |
| Custom columns | the view keys `Ctrl+1..0` | From SPEC 02 §3b: the per-mode flags (fullscreen, align extensions, upper and lower case names) and the `+1`/`+2` precision suffixes, which our format strings say another way. |

## Selection and sorting

| Item | Where | What is left |
| --- | --- | --- |
| Sort criteria chains | sort modes list, `F4` | FAR's per-sort-mode chains with their own editor (`Ins`/`Del`/`F4`/`Ctrl+↑↓`), where the chain changes with the key rather than being one list. Ours is one chain of up to three keys for every mode. |

## Files

| Item | Where | What is left |
| --- | --- | --- |
| File attributes | `Ctrl+A` | FAR's *System properties* button, which hands the file to the shell's own property sheet, and `chown`, which wants a user picker and the privilege to use it. |
| The files themselves on the clipboard | `Ctrl+Shift+C`/`X` | No terminal can be asked to carry a file — `OSC 52` carries text. This is window-system work, and it is what `fcmd-platform` would be for. |
| Descriptions | `Ctrl+Z` | Writing one *inside* an archive, and FAR's settings for which names to look for and whether to write UTF-8. |
| rar | `Shift+F1`/`F2` | Needs `unrar`, whose licence is not free: an optional feature, off in distribution builds. |
| `F7` inside an archive | — | No format stores a directory worth keeping, so this wants a decision about what an empty directory in an archive even is. |
| An archive on a server | — | A progress bar while the copy comes down. Today a large one is simply a pause. |
| 7z | `Shift+F1` | Writing an encrypted archive, split volumes, and the codecs only 7-Zip ZS writes (zstd, lz4, brotli). A zip encrypted with AES wants a feature of the `zip` crate we do not build, so it says what it is rather than asking. |

## Command line and history

| Item | Where | What is left |
| --- | --- | --- |
| Autocomplete | the line, `Ctrl+Space` | From FAR's version: appending the first match as one types, and a modal list. |

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
| Checksums | Files menu | CRC32 and `.sfv`, which is a different file format; splitting and combining; and checksums of something that is not on this machine, which waits on `JobFs::read_chunks` beyond the local filesystem. |
| Synchronise folders | `Ctrl+Shift+Y` | A saved pair of directories as a named job, and a written report. |
| User menu | `F2` | Reading FAR's `FarMenu.ini` as it stands, separators between groups, a dialog to edit the entries, and `Shift+F10` for the row taken last. |
| File associations | Commands menu | Defaults read off the OS (xdg-mime, `LSCopyDefaultRoleHandler`, the registry), a dialog to edit the blocks, and `execute`/`extract`/`preview` entries. |
| Two files side by side | `Ctrl+Shift+D` | A unified view, copying a hunk to the other side (an edit, so the editor's), a file against the copy on disk, and the first differing offset of two binaries. |

## Interface and configuration

| Item | Where | What is left |
| --- | --- | --- |
| A theme editor | `opt.colors` | A grid to pick from instead of typing, and FAR's colour *groups* — one change for every "selected" colour at once. |
| Lock the filter | `Ctrl+Alt+L` | The one of the three list behaviours that needs something new: the filter would have to outlive the list it belongs to, and no list yet asks for that — every one of them is built where it is opened. |
| Sorting collation | System settings | **Open question.** FAR splits what we fold into one `natural_sort`: collation (ordinal / invariant / linguistic), digits-as-numbers, case sensitivity. The ground it stands on: digits-as-numbers is `panel.natural_sort` and works; case sensitivity *exists in the comparison already* but is not settable — `Sort::case_sensitive` starts from the platform (sensitive on Linux, not on Windows and macOS); collation does not exist at all, because names are compared on their **UTF-8 bytes** on purpose (no name is validated to be sorted, and two names that are not valid UTF-8 do not collapse onto the same replacement character). What that costs, visibly, is one thing: `Éxtra.txt` sorts *after* `zorro.txt`, because `C3 A9` comes after `7A`. Two roads: (a) two settings and no dependency — `panel.sort_case = auto\|yes\|no`, and folding the Latin accents in the one branch that already decodes, documented as "the Latin accents, not a locale"; or (b) a real collator (`icu_collator` with CLDR data), which wants valid text, megabytes in the binary, and an entry in `THIRD-PARTY.md`. |
| Macro conditions | `Ctrl+.` | SPEC 08 §5. A condition is a named question the application already knows the answer to — is anything selected, is the cursor on a directory, is this panel a real directory or a list, is the command line empty, is this a server — so a `when` field holding a list of those needs no engine, no dependency and no language. What it does need is the *fall-through*, which is half of why FAR has it: a macro on `F5` conditioned on something being selected plays when something is, and when nothing is, `F5` copies as it always did. |
| The Lua engine | SPEC 08 §2 | **Open question**, and not the same row as the conditions above. far2m's macros are Lua: a dependency to justify, an API surface to draw (what a script may ask and what it may do), and the question of what a script is allowed to do when a recorded macro is already not trusted. FAR's fourteen macro areas are also finer than our five scopes; worth splitting when something needs it. |
| XLat | — | **Open question** on the key, which nothing has claimed. Transliterate a typed line between keyboard layouts, with tables, rules and flags. Niche outside Cyrillic keyboards, cheap once the tables exist. |

## Settings not yet in the configuration

| Item | Where | What is left |
| --- | --- | --- |
| Volumes with no mount point | `Ctrl+H` | Wants `FindFirstVolumeW`. The drive list shows what the platform can tell us otherwise. |
| The wastebasket as a panel | — | Browsing it, and putting something back from inside the program. |
| Plugins configuration | Options menu | SPEC 08: there is nothing to configure until plugins load from disk. |

## Netbox

| Item | Where | What is left |
| --- | --- | --- |
| `ProxyJump` in the session editor | `Alt+N` → `F4` | The session stores host, port, user, authentication, remote folder, group, keep-alive and compression; FAR's NetBox also goes through a jump host. One field in the dialog, one hop in the transport: the jump host's session first, then a `direct-tcpip` channel to the destination through it, which `russh` does. |
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
* **`Ctrl+M`** — FAR's *restore selection* is `Ctrl+Shift+M` here, because a terminal
  delivers `Ctrl+M` as `Enter`. This one will not change.
