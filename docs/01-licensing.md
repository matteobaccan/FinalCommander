# Licensing: can we rewrite FAR Manager in Rust?

Short answer: **yes**. Two independent reasons:

1. FAR Manager's own source code is released under the **BSD 3-Clause** licence, which permits
   reuse, modification, porting and relicensing-into-a-larger-work as long as the copyright
   notice and disclaimer travel with it.
2. Even without any licence, re-implementing the *behaviour* of a file manager (two panels,
   `F5` = copy, a command line under the panels) is not an act of copyright infringement.
   Copyright protects the *expression* — source code, help text, icons, artwork — not the
   function, the key layout or the file formats.

This document records what we verified, what the rules are, and where the real risks sit.

> Disclaimer: this is engineering due diligence, not legal advice. For a commercial release,
> have a lawyer read the final `THIRD-PARTY.md`.

## 1. FAR Manager — BSD 3-Clause

Verified from the upstream repository (`FarGroup/FarManager`, file `LICENSE`, fetched
2026-10-23):

```
BSD 3-Clause License

Copyright (c) 1996, Eugene Roshal
Copyright (c) 2000, Far Group
All rights reserved.
```

The three conditions are the standard ones:

| Condition | What it means for us |
| --- | --- |
| Source redistribution keeps the copyright notice, conditions and disclaimer | Only relevant if we *copy* FAR source into our tree. Then that file keeps its header and is listed in `THIRD-PARTY.md`. |
| Binary redistribution reproduces the notice in the documentation / accompanying materials | If any FAR-derived code ships in our binary, the notice goes into our `THIRD-PARTY.md` / `--about` screen. |
| No endorsement: the copyright holder's and contributors' names may not be used to promote derived products without written permission | We must not market SupremeCommander as "by Far Group", "Eugene Roshal's new commander", or use those names in our branding/taglines. Factual statements ("key layout compatible with FAR Manager") are description, not endorsement — keep them factual and neutral. |

Practical consequence: **a Rust rewrite inspired by FAR is unquestionably allowed**, and we may
even port individual BSD-licensed pieces (algorithms, tables, format parsers) if we keep the
notices. A permissive outbound licence (MIT/Apache-2.0) is compatible with that.

History note: FAR was shareware until the Unicode branch (1.80 → 2.0) was published in 2007 and
the 1.x branch followed in 2010; current development (3.x) is BSD-3-Clause on GitHub/GitLab.

## 2. The other commanders — what we may and may not touch

Verified licences:

| Project | Licence | May we read its code for ideas? | May we copy code into SupremeCommander? |
| --- | --- | --- | --- |
| FAR Manager (`FarGroup/FarManager`) | BSD-3-Clause | yes | yes, with notice |
| far2l (`elfmz/far2l`, Linux fork of FAR 2) | **GPL-2.0** (README: "License: GNU/GPLv2") | yes, but see the bleed-over risk below | **no** — would force the whole project to GPL-2.0 |
| far2m (`shmuz/far2m`, far2l + FAR3 Lua macros) | GPL-2.0 (inherits far2l) | same as far2l | no |
| Midnight Commander | GPL-3.0-or-later | yes | no |
| Krusader | GPL-2.0-or-later | yes | no |
| Double Commander | GPL-2.0-or-later | yes | no |
| GNOME Commander | GPL-2.0-or-later | yes | no |
| MTN2 (`Laex/MTN2`) | MPL-2.0 | yes | file-level copyleft: possible but it would make those files MPL — avoid for a Rust rewrite (it is Delphi anyway) |
| Total Commander | proprietary shareware | **no reverse engineering, no decompiling** | no |
| Directory Opus, Multi Commander, FreeCommander, WinNc, Altap Salamander | proprietary / freeware | no | no |
| nnn (BSD-2), yazi / xplr / broot / lf (MIT), joshuto (LGPL-3), vifm (GPL-2), ranger (GPL-3) | mixed | yes | only the permissive ones (MIT/BSD/Apache), with notice |

**The far2l bleed-over risk is the one to manage.** far2l is the closest thing to "FAR on
Linux" and it is tempting to read it while solving the same problems (Unix console, no
`CONIN$`, no ADS, case-sensitive paths). Reading it is legal; *transcribing* it is not, because
a line-by-line Rust translation of a GPL-2.0 C++ file is a derivative work. Rule for this
project: **far2l is reference documentation for behaviour, never a source to translate.** When
a non-obvious algorithm is needed, take it from FAR itself (BSD) or write it from the spec.

## 3. Total Commander, and why its key layout is not a problem

We want a `tc` keymap profile. That is legitimate:

- **EU**: Directive 2009/24/EC art. 1(2) protects the expression of a program, not the ideas and
  principles underlying it. In *SAS Institute v World Programming* (C-406/10) the CJEU held that
  the functionality of a program, its programming language and its data file formats are not
  protected by copyright, and that reproducing functionality without access to the source code
  is not infringement.
- **US**: *Lotus v Borland* (1st Cir. 1995) held a menu command hierarchy to be an
  unprotectable method of operation; *Google v Oracle* (2021) treated re-implementing an API's
  declarations as fair use.

So: key bindings, menu structure, command names, panel layout, the `cm_*` command vocabulary →
free to re-implement. What is *not* free:

- help files, manuals, translated string tables, icons, themes, sounds, screenshots;
- any binary or resource extracted from a proprietary product (`*.chm`, `WCMICONS.DLL`, …);
- decompilation of Total Commander or any other proprietary commander — forbidden by its EULA
  and unnecessary for us;
- the *names*: "Total Commander", "Norton Commander", "Far Manager", "Midnight Commander",
  "Double Commander" are trade marks of their owners. We may say "TC-compatible keymap"
  (nominative, descriptive use); we may not name a product, a logo, a domain or a package after
  them, and we should not imply affiliation.

For this repo the keymap tables in `docs/03-keymap.md` were written from primary sources where
they are open (FAR's own `FarEng.hlf.m4`, mc's man page, Krusader's DocBook handbook) and from
secondary third-party cheat sheets for Total Commander, **in our own wording, as factual
key→action pairs**. No documentation text is copied.

## 4. Third-party components we might want (watch these)

| Component | Licence | Note |
| --- | --- | --- |
| 7-Zip / `7z.dll`, `sevenz-rust2` | LGPL-2.1 (7-Zip), MIT/Apache (Rust crates) | LGPL dynamic use is fine; prefer pure-Rust crates |
| UnRAR source | **non-free**: free to use, but forbids using the source to re-create the RAR *compression* algorithm | read-only RAR support via the official unrar lib or `unrar` crate; keep it an optional feature with its own notice |
| `libarchive` | BSD-2 | fine |
| `russh` / `ssh2` (SFTP) | Apache-2.0 / LGPL-2.1+ | prefer `russh` (Apache-2.0) |
| `wasmtime` (plugins) | Apache-2.0 WITH LLVM-exception | fine |
| `mlua` (Lua scripting) | MIT; Lua itself MIT | fine |
| `ratatui` / `crossterm` | MIT | fine |
| Colour themes ported from other apps | often undeclared | re-derive palettes from the published hex values or ship our own; do not copy theme files wholesale |

## 5. Outbound licence for SupremeCommander

**Decided: MIT** (`LICENSE` in the repository root, copyright "Matteo Baccan and the
SupremeCommander contributors").

Consequences of that choice, recorded so nobody has to re-derive them:

- Inbound compatibility: we may use MIT, BSD-2/3, Apache-2.0, ISC, Zlib and MPL-2.0 (file-level)
  dependencies. FAR Manager fragments (BSD-3) may be reused, keeping their notice in
  `THIRD-PARTY.md`.
- **No GPL/LGPL/AGPL code in the shipped binary's default feature set.** An LGPL component is
  acceptable only as a *dynamically loaded, optional* library (e.g. a system `7z.dll`/
  `liblzma`), never statically linked into the default build. `cargo deny` enforces the
  allow-list.
- Non-free pieces stay out of the default build: the UnRAR library (its licence forbids using
  the source to re-create the RAR compression algorithm) is an opt-in cargo feature,
  disabled in distro packages, with its notice shipped alongside.
- Contributions are under MIT by the usual inbound=outbound rule (stated in `CONTRIBUTING.md`);
  no CLA.
- A GPL-licensed *plugin* may exist outside this repository: plugins are separate artifacts
  (WASM/Lua) loaded at runtime, so their licence is their author's business.

Apache-2.0 was considered for its explicit patent grant (the Rust ecosystem's usual
`MIT OR Apache-2.0`); MIT alone was chosen for simplicity. If a patent grant is ever needed,
re-licensing to `MIT OR Apache-2.0` only requires contributor agreement, which inbound=outbound
keeps feasible.

## 6. Operating rules for contributors (clean-room-ish)

1. Implement from the **specs in `docs/spec/`**, not by porting someone else's file.
2. FAR Manager source may be consulted and partially reused; add the BSD notice and an entry in
   `THIRD-PARTY.md` when reused.
3. GPL projects (far2l, mc, Krusader, Double Commander) may be consulted to understand
   *behaviour*; never copy, never translate line by line. If a PR's diff mirrors a GPL file's
   structure, it gets rejected.
4. Proprietary products: use them as a user, document observed behaviour, never decompile.
5. Every third-party dependency goes into `THIRD-PARTY.md` with licence and purpose, generated
   by `cargo about` / `cargo deny` in CI.
6. No trade mark of another commander in our name, icon, domain or release artifacts.

## Sources

- `https://github.com/FarGroup/FarManager` — `LICENSE` (BSD-3-Clause), `far/FarEng.hlf.m4`
- `https://github.com/elfmz/far2l` — `README.md` ("License: GNU/GPLv2")
- `https://en.wikipedia.org/wiki/Far_Manager`, `https://en.wikipedia.org/wiki/Double_Commander`
- `https://github.com/MidnightCommander/mc`, `https://github.com/KDE/krusader`
- `https://github.com/Laex/MTN2` (MPL-2.0)
- CJEU C-406/10 *SAS Institute v World Programming*; Directive 2009/24/EC art. 1(2);
  *Lotus v Borland*, 49 F.3d 807 (1st Cir. 1995); *Google LLC v Oracle America*, 593 U.S. (2021)
