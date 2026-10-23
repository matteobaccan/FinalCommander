# The commander landscape (survey, Oct 2026)

Why this file: before specifying SupremeCommander we catalogued what already exists, what each
project is written in, how it is licensed, and which of its ideas are worth taking. "Commander"
here means an **orthodox file manager** (OFM): two equal panels, a command line under them, a
function-key bar at the bottom, keyboard first.

Legend for *Take*: ✅ idea we want, 🟡 maybe, ❌ deliberately not.

## 1. The ancestors (DOS / text mode)

| Project | Years | Language | Licence | Notes | Take |
| --- | --- | --- | --- | --- | --- |
| **Norton Commander** | 1986–1998 | asm / C | proprietary, discontinued | Defined the genre: 2 panels, `F1..F10` bar, command line, built-in viewer (`F3`), tree, archive browsing via plugins | ✅ the whole grammar |
| **Volkov Commander** | 1991– | asm | freeware (closed) | 64 KB NC clone, legendary speed | ✅ "stay small/fast" |
| **DOS Navigator** | 1991–1999 | Turbo Pascal | source released 1999 under BSD-3, but "compromised" (bundled Turbo Vision / Virtual Pascal terms) | Tabs/windows inside the manager, spreadsheet, terminal, multiple panels | ✅ multi-panel windows, 🟡 do **not** vendor its code |
| **NDN** (Necromancer's DOS Navigator) | 2000s | Pascal | DN-derived, unclear | DN fork, still used on Windows console | 🟡 reference only |
| **Demos Commander** (`deco`) | 1990s–2001 | C | GPL | First serious Unix OFM, pre-mc | reference |

## 2. Windows

| Project | Language | Licence | Notes | Take |
| --- | --- | --- | --- | --- |
| **FAR Manager** (`FarGroup/FarManager`) | C++ | **BSD-3-Clause** | The reference implementation for us: panels, plugin API, internal viewer/editor, macros (Lua), dialogs API, huge plugin ecosystem | ✅ primary model |
| **Total Commander** | Delphi | proprietary shareware | Tabs, multi-rename, synchronise dirs, FTP, packer/WFX/WLX/WCX plugin ABIs, branch view | ✅ features, keymap profile; ❌ no code/resources |
| **MTN2** (`Laex/MTN2`) | Delphi (FireMonkey/Skia) | **MPL-2.0** | Modern DN/FAR tribute: TUI grid drawn on a GPU canvas, ConPTY console, tabs, SFTP, archives as folders, **native + WebAssembly plugins**, JSON keymap/themes, Linux TUI + SDL2 builds | ✅ wasm plugins, keymap/theme files, real PTY; ❌ the GPU/SDL window — we stay strictly terminal |
| Directory Opus | C++ | proprietary | The heavyweight; scripting, metadata, FAYT | 🟡 ideas |
| Multi Commander / FreeCommander / Unreal Commander / WinNc / SpeedCommander / Altap Salamander | mixed | freeware or proprietary | TC-alikes | 🟡 |
| Nomad.NET, xplorer², One Commander, XYplorer, Q-Dir | mixed | proprietary/freeware | | ❌ |
| Explorer++, Files | C++ / C# | GPL-3 / MIT | not orthodox (Explorer-like) | ❌ |

## 3. Unix / cross-platform desktop

| Project | Language | Licence | Notes | Take |
| --- | --- | --- | --- | --- |
| **Krusader** | C++/Qt (KDE) | GPL-2.0-or-later | The most complete OFM feature set on Linux: synchroniser, queue manager, MountMan, checksums, split/combine, UserActions, key-binding profiles ("swap easily to other commanders") | ✅ feature list, ✅ keymap-profile idea |
| **Double Commander** | Free Pascal / Lazarus | GPL-2.0-or-later | TC-compatible defaults on Win/Linux/macOS, `cm_*` command vocabulary, WCX/WFX/WLX plugin reuse | ✅ `cm_*` command model |
| **GNOME Commander** | C++/GTK | GPL-2.0-or-later | GTK OFM, python plugins | 🟡 |
| **muCommander** | Java | GPL-3.0-or-later | Runs anywhere incl. Solaris/OpenVMS; virtual filesystems | 🟡 |
| **Nimble Commander** | Obj-C++/macOS | GPL-3.0 (free since 1.6) | Best-in-class macOS OFM; native feel, VFS, batch rename | ✅ macOS UX notes |
| **File Commander** (`VioletGiraffe/file-commander`) | C++/Qt | Apache-2.0 | TC-like, Win/mac/Linux, plugin viewers | 🟡 |
| Tux Commander, Worker, emelFM2, SpaceFM, Xfe, 4Pane, Sunflower | C/C++/Python | GPL family | older GTK OFMs | reference |
| Commander One, ForkLift, CRAX Commander, Marta, Path Finder | Swift/Obj-C | proprietary | macOS | ❌ |
| Ghost Commander, Total Commander for Android, Material Files | Java/Kotlin | GPL-3 / freeware | mobile OFMs | 🟡 future |

## 4. Terminal (TUI)

| Project | Language | Licence | Model | Take |
| --- | --- | --- | --- | --- |
| **Midnight Commander** (`mc`) | C | GPL-3.0-or-later | The Unix standard: 2 panels, subshell, VFS (`extfs`, `fish`, `sftp`, `tar`), `mcedit`/`mcview`/`mcdiff`, learn-keys, panelize | ✅ subshell integration, VFS model, keymap profile |
| **far2l** | C++ | **GPL-2.0** | FAR 2 ported to Unix: full plugin set (NetRocks SFTP/FTP/SMB/NFS/WebDAV/S3, multiarc, colorer), TTY + GUI backends, X11 clipboard, `far2l` extensions over ssh | ✅ behavioural reference (read, never copy) |
| **far2m** | C++ | GPL-2.0 | far2l + FAR3 Lua macro engine | ✅ macro design reference |
| **vifm** | C | GPL-2.0 | Two panels with vi modality, own VFS-ish `fuse` mounts | ✅ vi keymap profile |
| **ranger** / **joshuto** / **lf** / **nnn** / **clifm** / **felix** | Python / Rust / Go / C | GPL-3 / LGPL-3 / MIT / BSD-2 / GPL-2 | single-pane column browsers, not OFM | 🟡 previews, async design |
| **yazi** | Rust | MIT | The current state of the art in Rust TUI FM: async I/O, Lua plugins (`ya pkg`), image/video/PDF previews, multi-tab, SFTP VFS | ✅ async engine + preview pipeline, ✅ plugin packaging |
| **broot** | Rust | MIT | tree + fuzzy navigation | ✅ fuzzy "goto" |
| **xplr** | Rust | MIT | programmable explorer, Lua config | ✅ config model |
| **superfile** | Go | MIT | polished UI | 🟡 visual polish |
| **termscp** | Rust | MIT | remote transfer TUI (SFTP/SCP/FTP/S3/SMB) | ✅ remote backends |
| **Cloud Commander** | JS | MIT (unverified) | dual-pane in the browser | 🟡 future web UI |

## 5. Rust prior art (dual-pane, as of Oct 2026)

Small and mostly young — this is the gap SupremeCommander targets. None of these is a
full FAR/TC-class commander today.

| Project | Toolkit | Note |
| --- | --- | --- |
| `Harunx9/sfm` | TUI | vim + TC inspired, on crates.io |
| `jaroslavpachola/rcmd` | ratatui (+egui window mode) | mc keybindings, archive and FTP/SFTP panels |
| `mg4466/ferric-commander` | TUI | no async runtime, slow ops on threads |
| `anjunar/rust-commander` (RCommander) | GTK4 | Win/mac/Linux, SFTP |
| `gr3p/rust-commander` | GTK4 | F-key workflow, early |
| `DoubyCz/douby-commander` | GTK4 | Double Commander tribute |
| `ice-commander/app` | GTK4 | local + FTP/SFTP/WebDAV |
| `sonatina-dream/ferrous-commander` | GPUI (macOS) | Commander One behaviour |

Licences of these were **not** verified; treat them as prior art to learn from, not to copy.

## 6. What the survey tells us

1. **The feature ceiling is FAR + Total Commander + Krusader.** Everything else is a subset.
   Our feature matrix (`02-features.md`) is built as the union of those three, plus the modern
   additions from yazi/MTN2 (async previews, wasm/Lua plugins, JSON/TOML config).
2. **Nobody owns "orthodox file manager in Rust" yet.** The Rust entries are either single-pane
   browsers (yazi, joshuto, xplr) or very young GTK4 apps. A complete, portable,
   complete, portable, terminal-only commander is an open slot.
3. **Strictly TUI is a positioning choice, not a limitation.** The GUI-toolkit commanders
   (Krusader, Double Commander, the young GTK4 Rust ones) already crowd that space; the open
   slot is a terminal commander with a modern engine that works identically over SSH.
4. **Keymap profiles are the migration path.** Krusader, Double Commander, MTN2 and mc all ship
   alternative key-binding sets. We make that a first-class feature (`far`, `mc`, `tc`, `nc`,
   `vi` profiles) — see `03-keymap.md`.
5. **Plugins decide longevity.** FAR's and TC's ecosystems are why they survived 30 years.
   We need a stable ABI from day one: WASM (sandboxed, portable) + Lua (scripting) + an
   optional native C ABI. See `spec/09-plugins.md`.
6. **Licence-wise the clean path is FAR (BSD) as the model**, with GPL projects (far2l, mc,
   Krusader, DC) used as *behavioural* references only. See `01-licensing.md`.
