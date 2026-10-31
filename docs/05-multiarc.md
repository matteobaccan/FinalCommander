# MultiArc: how FAR drives external archivers

FAR writes no archive itself. It shells out, and it has done so since the nineties through a
plugin called **MultiArc**, whose whole design is a *table of command lines*. This file is that
table and that design, read from FAR's own repository (`FarGroup/FarManager`, BSD-3-Clause,
HEAD `b63106e`, fetched 2026-10-31), so that the decision about what to copy is made from what
is there rather than from memory of it.

Why it matters here: `docs/02-features.md` row 4.1 lists seven formats we do not read (rar, cab,
iso, rpm, cpio, lha, arj) and row 4.2 one option we do not offer (split volumes). Each of those
is thousands of lines of Rust if we write the format, and four lines of configuration if we ask
a program that already has it. MultiArc is thirty years of experience in doing the second thing.

Sources, in order of authority:

| Source | What it gives |
| --- | --- |
| `plugins/multiarc/custom.ini` (554 lines, 243 of them the format's own documentation) | the placeholder language, the listing grammar, and 15 worked format definitions |
| `plugins/multiarc/global.cpp:10` | `CmdNames[]` — the canonical list of operations |
| `plugins/multiarc/Archivers/*.reg` | 12 shipped archiver tables, one file each |
| `plugins/multiarc/Docs/eng/Arc.Support.txt` | the user-facing description of the split |
| `plugins/multiarc/*.vcxproj` | the 10 compiled format modules |

## 1. Two layers, and only one of them is external

`Arc.Support.txt` §2 states the split plainly:

> Viewing archive contents is done by the plugin itself, through the respective plugin
> extension module (`<ArchiveType>.fmt`) or by calling (automatically and invisibly for the
> user) external utility programs.
>
> To create or modify archives, external archiver programs are used.

So:

- **Listing** has two implementations. Ten compiled modules — `ace arc arj cab custom ha lzh rar
  targz zip` — of which nine know a format and read its headers in process. The tenth, `custom`,
  knows no format at all and gets everything from `custom.ini`: it runs an external program and
  parses what it prints.
- **Every operation** — extract, add, delete, test and nine more — is a command line. There is no
  in-process writing anywhere in MultiArc.

That asymmetry is the right one and it is worth keeping: reading a listing is what a panel does
constantly and must be quick, so the formats worth having are read in process; writing happens
when a person asks for it and can afford a process.

## 2. The sixteen keys

From `global.cpp:10`, in order:

```
Extract  ExtractWithoutPath  Test  Delete
Comment  CommentFiles  SFX  Lock  Protect  Recover
Add  Move  AddRecurse  MoveRecurse  AllFilesMask  DefExt
```

Fourteen are commands; `AllFilesMask` and `DefExt` are settings stored in the same table. An
archiver that cannot do one of them **leaves that string empty**, which is how MultiArc knows
not to offer the operation — the table doubles as the capability list, with no separate
declaration to keep in step with it.

Note what is on that list beyond the obvious four: a comment on the archive and a comment per
file, making a self-extracting archive, locking an archive against change, adding recovery
records and recovering from them. Those are RAR features, and the table is how FAR offers them
without knowing anything about RAR.

## 3. The placeholder language

This is the part worth taking. A command string is not a shell line with the names pasted in;
it is a template with typed holes.

| Hole | What it becomes |
| --- | --- |
| `%%A` | the archive's name (`%%a`: its short name) |
| `%%W` | a temporary folder, with the trailing slash |
| `%%P` | the password |
| `%%R` | the current folder *inside* the archive |
| `%%L` | **the name of a list file** — a file holding the names to work on (`%%l`: short names) |
| `%%F` | the names on the command line, re-running the archiver until they are all used up |
| `%%f` | one name per run, for a program that cannot take more |
| `%%E<n>` | the highest exit code still counted as success; without it, only 0 is |
| `%%S` | where the extra switches typed in the dialog go; at the end if absent |

`%%F` carries its own length limit: `%%F512` means "fill the line up to 512 characters, then run
again with what is left", and the default is 127 — a DOS command line. This is the answer to
"what if the selection is four thousand files", and it is the answer we would otherwise have had
to invent.

`%%L` is the better answer to the same question, and the one to prefer: the names go in a file,
so there is no length limit and no name that the shell or the archiver could mistake for a
switch.

Nine modifiers follow a hole, with no space, and several may be combined (`%%LMN`, `%%F*QSN128`):

| | |
| --- | --- |
| `Q` | quote the names that contain spaces |
| `q` | quote all of them |
| `S` | `/` instead of `\` in paths |
| `M` | pass a folder as `folder\*.*` |
| `N` | pass a folder as `folder` (assumed when neither `M` nor `N` is given; both may be given, and then both forms are passed) |
| `*` | use `*` as the all-files mask, overriding `AllFilesMask` for this one command |
| `W` | the name only, without its path |
| `P` | the path only, without the name |
| `A` | encode in ANSI rather than OEM |

`P` and `A` may also be used on `%%A` and `%%a`.

## 4. `{ }`, the trick that keeps it to one line

A part of the string wrapped in braces is included **only if it contains at least one
placeholder that came out non-empty**. So

```
"Add"="rar a -y {-p%%P} {-ap%%R} {-w%%W} {%%S} %%A @%%LN"
```

is one line that covers a plain add, an add with a password, an add into a subfolder of the
archive, and any combination: `-p` disappears when there is no password rather than being passed
empty. Without this there would have to be a variant per combination, or a conditional language.
It is the single cleverest thing in the file.

## 5. Reading what the program printed

For the `custom` module, `custom.ini` describes both halves of recognition and parsing.

**Which format is it.** `Extension` (no dot), plus `ID` — the magic bytes as two-digit hex
separated by spaces — with `IDPos` for where to look (empty means "near the start") and
`IDOnly=1` to ignore the extension entirely and trust the bytes.

**Getting the listing.** `List` is the command, appended with the archive name unless the string
already names it with `%%A`. `Errorlevel` sets the highest accepted exit code; `IgnoreErrors`
ignores it altogether, "use with caution".

**Finding the rows.** `Start` and `End` are the text after which parsing begins and at which it
stops; a leading `^` anchors to the start of the line, otherwise the text is looked for anywhere
in it. `IgnoreString0..N` drops rows outright.

**Reading a row.** `Format0..FormatN` are tried in order, and there are *two* grammars:

- **Positional**, a mask where each letter claims a column: `n` name, `z` unpacked size, `p`
  packed size, `c` description, `d`/`t`/`y` day, month, year, `TTT` a three-letter month name,
  `h`/`m`/`s` time, `H` the am/pm letter, `a` attributes, `r` CRC in hex, `*` skip to the next
  space, `.` trim the name and give it a dot, `(…)` digits that may be missing, and three more
  sets of letters for access time (`b v e x l k`) and creation time (`j g f o i u`).
- **Regular expressions**, which FAR added later: a pattern in slashes, PCRE, with flags `i x m
  s` after the closing slash, and **named captures** carrying the fields — `name`, `size`,
  `packedSize`, `attr`, `description`, `CRC`, and `mYear`/`mMonth`/`mDay`/`mHour`/`mMin`/`mSec`
  with the same for access (`a…`) and creation (`c…`) time, plus `mAMPM` and `mMonthA`. The
  file's own examples parse `dir` output and a third-party tool's pipe-separated table.

The regex mode supersedes the positional one and is the one to copy. The positional mode was
right when archivers printed fixed-width tables under DOS; it fails silently now, because a name
that overflows its column simply takes the next field's characters and nothing says so.

## 6. What the tables ship with

**Twelve archivers** in `Archivers/`, one `.reg` each: `Arj`, `FarUC2`, `InfoZIP`, `LGHA`,
`LhArk`, `Lha32`, `Lha32a`, `Lha32b`, `PkZip25`, `PkZip25Dos`, `Rar20`, `WinZipCL`. RAR's, whole:

```
"Extract"           ="rar x {-p%%P} {-ap%%R} -y -c- {%%S} %%A @%%LNM"
"ExtractWithoutPath"="rar e -av- {-p%%P} -y -c- {%%S} %%A @%%LNM"
"Test"              ="rar t -y {-p%%P} -c- {%%S} %%A @%%LNM"
"Delete"            ="rar d -y {-w%%W} {%%S} %%A @%%LNM"
"Comment"           ="rar c -y {-w%%W} {%%S} %%A"
"CommentFiles"      ="rar cf -y {-w%%W} {%%S} %%A {@%%LNM}"
"SFX"               ="rar s -y {%%S} %%A"
"Lock"              ="rar k -y {%%S} %%A"
"Protect"           ="rar rr -y {%%S} %%A"
"Recover"           ="rar r -y {%%S} %%A"
"Add"               ="rar a -y {-p%%P} {-ap%%R} {-w%%W} {%%S} %%A @%%LN"
"Move"              ="rar m -y {-p%%P} {-ap%%R} {-w%%W} {%%S} %%A @%%LN"
"AddRecurse"        ="rar a -r0 -y {-p%%P} {-ap%%R} {-w%%W} {%%S} %%A @%%LN"
"MoveRecurse"       ="rar m -r0 -y {-p%%P} {-ap%%R} {-w%%W} {%%S} %%A @%%LN"
"AllFilesMask"      ="*.*"
```

**Fifteen formats** in `custom.ini`: `JAR`, `IMP`, `AIN`, `UC2`, `InstallShield`, `Limit`, `RK`,
`7z`, `rpm`, `cpio`, `deb`, and `InnoSetup2` through `InnoSetup5`.

Three of those are the point. **`rpm`, `cpio` and `deb` all go through the user's `7z`**, and each
is four working lines:

```
[rpm]
ID=ED AB EE DB
IDOnly=1
Extension=rpm
List=7z l -- %%AQ
Extract=7z x {-p%%P} -r0 -y -scsDOS -i@%%LQMN -- %%A
Test=7z t {-p%%P} -r0 -scsDOS -i@%%LQMN -- %%A
```

`cpio` and `deb` are the same with a different extension and no magic. `rpm`, `cpio` and `lha`
are three of the seven formats row 4.1 says we are missing, and this is what they cost FAR.
(`deb` we read in process since the `ar` reader landed, so there we are ahead.)

`custom.ini` also says, in its first line, that a `custom_user.ini` beside it **extends and
overrides** it. The shipped table is a default, not a fixed list.

## 7. What to take, and what MultiArc does not solve

Take: the two layers, the sixteen keys with the empty-string-means-cannot convention, the
placeholder language including `%%L`, `%%E<n>` and `%%S`, the `{ }` rule, the magic-plus-extension
recognition, and the regex grammar with named fields. Take the shape of a user table that
extends the shipped one.

Leave: the positional column masks (§5), and the `%%F`/`%%f` re-running — a list file covers it,
and anything that cannot read a list file we can feed through a temporary one anyway.

And four things MultiArc has no answer for, because it is a Windows plugin from the nineties and
we are not:

- **Progress.** Our engine counts bytes and draws a bar. A child process gives percentages on
  its standard output, locale-dependent and different per tool. The honest answer is an
  indeterminate bar with the tool's own output in the job's log, not a parser per archiver.
- **Cancellation.** `Esc` cancels one of our jobs between files. Against a child process it has
  to mean killing it, and killing a packer halfway leaves whatever it had written.
- **Atomicity.** SPEC 04 §4 has it that a change to an archive is written beside the original and
  renamed over it, so an interruption leaves the original whole. `rar a` and `7z a` modify in
  place. Keeping our rule means copying the archive to a temporary, letting the tool work on the
  copy, and renaming — one full copy of the archive per change.
- **No shell.** Everything we spawn today goes through a shell line (`App::run_filter`), which is
  right for a command a person typed and wrong here: a packer gets an argv array, with the names
  in a list file and never on the line. This is the first time we would run an external program
  to *write* the user's files, and it is the one place where a quoting bug stops being cosmetic.

A fifth, which is ours rather than MultiArc's: `Format::writable`, which came with the `ar`
reader, answers "can
this program write it". With an external packer the question splits in two — can *we* write it,
and is there a tool here that can — so the pack dialog's list becomes a property of the machine,
and a format offered but unavailable has to say which program is missing rather than failing.
