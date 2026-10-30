# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[semantic versioning](https://semver.org/), which before 1.0 means anything may still move.

## [Unreleased]

### Added

- **The best answer appears while one types, and the list can have FAR's manners** (SPEC 06
  §7.4). Two settings under `[cmdline.complete]`:

  - `append` — `hint` by default: the rest of the best answer is drawn after the caret in a
    dim colour and is **not** in the line, so `Enter` still runs what was typed, and `→` or
    `End` takes it. `line` is FAR's own, where it goes into the line marked out so the next
    character replaces it and `Enter` runs it with the offer attached; `off` is neither.
    Nothing is appended unless the caret is at the end of the line, the answer continues what
    is typed without changing it, and the answer's own caret is at its end — so `Cargo.toml`,
    which is offered for `car` because a completion that minded the case would be one nobody
    uses, is in the list but not after the caret: the line would read `cargo.toml` and one
    would get `Cargo.toml`. `Tab` takes that one.
  - `modal` — off by default. FAR's list owns `Enter` the moment it appears; ours waits until
    somebody has walked into it, because a list that stole `Enter` would make typing feel like
    a trap. `true` asks for FAR's manners.

  Every theme gained `cmdline.hint.fg`, which is the dim of its own palette.

- **The information panel says what the directory is sitting on, and what its repository is
  doing** (`Ctrl+L`, SPEC 02 §9). Two lines that were the other half of *where am I*:

  - **the filesystem and the mount**: `ext4 (rw,noatime)` and `/dev/vda1 on /`. Linux keeps
    the whole table in `/proc/self/mountinfo`, so it is read with no call at all and the
    deepest mount point that covers the path wins — which is what makes a bind mount inside
    another answer for itself. macOS answers `statfs`, and its flags are reported in Linux's
    spelling so that nobody has to learn two vocabularies. Windows has no mount options; what
    it has is the filesystem's name, which the drives dialog already asks for. `ro` and
    `noexec` are the two words that explain most of the refusals a panel can produce.
  - **the git repository**: `main · 3a7f2e1 · 3 of 42 differ from the index`, with a merge or
    a rebase halfway through named as such. Nothing is run — `git` need not even be installed
    — and nothing is hashed: the branch, the commit and the state are small files, and the
    count is one `stat` per tracked file against what the index recorded. Which is why the
    line says *differ from the index* and not *modified*: a file that was touched, or rewritten
    with the same bytes, is counted here, and telling those apart is what hashing is for. An
    index larger than twenty thousand files, or in a version this reader does not know, says
    *not counted* rather than guessing. Untracked files are not looked for, because that means
    walking the tree and reading every `.gitignore` on the way down.

  Neither is read unless a panel is showing it: the request for a listing now says whether
  anything on screen is asking, and a panel of files asks for none of this. The prompt's own
  `%gitbranch%` moved onto the same reader, so there is one walk upwards in the program rather
  than two.

- **FAR's precision suffix on a column width** (SPEC 02 §3b). `Column widths` may carry a
  `+n` after a number — `8+2` — which is not width but precision, and importing a FAR mode
  dropped it without saying so. FAR counts precision where we spell a format, so this is where
  the one becomes the other: on a time column `+1` arrives as `%I:%M %p`, `+2` as `%H:%M:%S`
  and `+3` as `%H:%M:%S%.3f`; a date-and-time column keeps its date and lengthens only the
  clock; and `+2` on a date on its own asks for the four digits of the year, which ours writes
  in any case, so there is nothing to change and nothing to report. A precision on a column
  that has none — a size, a name — is reported, like every other part of a FAR mode we cannot
  honour.


## [0.23.0] — 2026-10-30

### Added

- **`F11` reads a video, a sound and a PDF, not only a program** (SPEC 05 §1.8). The structure
  pane existed because a binary is a tree of records with a length on each; so are the files
  people actually keep in a folder, and they raise the same question — what is this, and where
  inside it is anything.

  - **ISO base media** — MP4, MOV, M4A, M4V, 3GP, HEIF: the brands, the duration, every track
    with its codec, its resolution or its sample rate and its language, the tags an audio
    player writes into an `.m4a`, and the boxes as rows `Enter` follows. A film is three
    gigabytes of `mdat` and this reads sixteen bytes per box plus the `moov`. A `.mov` from
    before `ftyp` existed is recognised by the box it opens with.
  - **Matroska** — MKV, WebM, MKA: the `DocType`, the `TimecodeScale` and `Duration`, the title
    and the writing application, every `TrackEntry`, and the segment's children as navigation,
    with the clusters listed once and then counted because a film has tens of thousands. An
    element of unknown length — a stream still being written — stops the walk at that level
    rather than at the file.
  - **RIFF** — AVI, WAVE, WebP: AVI's main header and a group per stream, with the codec taken
    from the *format* chunk because `strh`'s handler is zero in half the files ever written;
    WAVE's format and how long its data plays; WebP's canvas, which is the one picture size our
    own decoder cannot give.
  - **PDF**: the version, the page count, the first page's paper in points, millimetres and by
    name, the `/Info` fields, what `/Encrypt` allows, how many times the file has been saved
    since it was written — and the pages themselves, each row going to its object. The index of
    a PDF is at the end and, since 1.5, is itself a deflated object whose catalogue lives inside
    object streams, so this follows `startxref`, reads the classic table or the stream, and
    inflates what it must, undoing the PNG predictor. **An index that leads nowhere is not an
    error**: the file is read for `N G obj` and that is believed instead, which is what a
    truncated download or a hand-edited file needs. An object packed into a stream sends `Enter`
    to the stream that holds it, because that is where its bytes are, and an encrypted file's
    strings are shown as `(encrypted)` rather than as the ciphertext they hold.

  Neither reader decodes a frame or pulls out a page's text: the first wants a video codec,
  which means a C library and a licence this project has refused, and the second wants the
  content stream's operators and a font's `ToUnicode` map. Both already have an honest answer
  in a `[[decompile]]` rule with `ffprobe` or `pdftotext` on `Shift+F6`.

### Fixed

- **The keys that change a binary's length are tested now.** `Ins` has chosen between
  inserting and overwriting in the hex editor since it was written, `Backspace` takes out the
  byte before the cursor while inserting and steps back while overwriting, `Del` takes out the
  one under it, and the header says `ins` or `ovr` — all of it was there, and none of it was
  covered by a test: the editor's own tests exercise the model, which cannot tell whether a key
  reaches it. That is precisely how the wastebasket went missing on a server earlier in this
  cycle. Two tests now press the keys and read the header.

- **A table's columns stay where the heading says they are.** A cell holding a newline — a
  CSV field between quotes, a JSON string somebody wrote as a paragraph — slid every column
  to its right one place left, on that row and on no other. The two halves of the program
  disagreed about a control character: `UnicodeWidthStr::width` counts one as a column while
  the screen buffer skips it and draws none, so the cell was measured a column wider than it
  appeared. A cell is flattened to one line before it is measured or drawn now, with each
  control character replaced by a space rather than dropped — `Attività:` and a list beneath
  it read as `Attività: - Eseguire` instead of running together — and the two measures cannot
  disagree because there is nothing left to disagree about.

- **The quick view shows a picture.** `Ctrl+Q` beside a `.png` left an empty frame: a picture
  has no rows of text, and only the viewer's own window knew to paint one. Both windows paint
  it the same way now. The quick view decodes at most eight mebibytes, because it draws
  whatever the cursor is standing on and the decoding runs on the thread that draws — a
  directory of photographs would otherwise make every arrow key feel stuck. Anything larger
  waits for the `F3` one presses on purpose.


## [0.22.0] — 2026-10-30

### Added

- **A picture is shown as the picture** (SPEC 05 §1.14). `F3` on a `.png`, `.jpg`, `.gif` or
  `.bmp` draws it, and `F4`'s wheel goes on to the dump and the hex for the times one came
  for the bytes instead.

  It is drawn in half blocks: a terminal cell is about twice as tall as it is wide, so a cell
  holding `▀` with the top half in the foreground colour and the bottom in the background is
  two pixels that are each very nearly square, and a window is a canvas twice as tall in
  pixels as it is in rows. It is coarse and it works in **every** terminal there is, which is
  why it comes before kitty's, iTerm2's and sixel's protocols rather than after them — a
  picture one cannot see at all is worse than one made of blocks, and nothing below the
  drawing has to change when the protocols arrive.

  The file's first bytes decide, never its name. A photograph is fitted to the window keeping
  its shape; a sixteen-pixel icon is left at its own size rather than blown up into coloured
  squares. The scaling averages the pixels it covers, so the text in a screenshot stays
  legible. What is transparent shows the viewer's own background through it. Nothing scrolls,
  because the picture is already the size of the window, and a file that is damaged, too
  large or not a picture says so rather than leaving a blank screen.

- **Base64, undone** (SPEC 05 §1.13). `Alt+B` in the viewer decodes a file that is base64 and
  shows what came out, keeping the path in the title. A certificate, a key, an attachment
  pulled out of a mail, a `data:` URI pasted into a file: all of them are bytes written down
  in sixty-four printable characters, and all of them are unreadable until something undoes
  that.

  The bytes decide how they are drawn — text when they read as text, the hex pane when they
  do not, which is the usual case — and the status line says how many came out, because that
  figure is how one knows the decode was the right one. Three shapes are read: the bare
  payload (wrapped or not, padded or not), PEM armour with its `-----BEGIN …-----` lines
  skipped, and a `data:` URI with its label taken off. Both alphabets work, the standard one
  and the URL-safe one a token uses.

  A file that is only *mostly* base64 is refused, with the byte that stopped it and where it
  is: decoding as far as the first bad character would hand back bytes nobody wrote. It is
  the one view that replaces the file rather than drawing it another way, so the key does not
  toggle — `Esc` and opening the file again are the way back, as they are from a
  disassembly.

### Fixed

- **The quick search is drawn in its own panel's foot** (SPEC 02 §6). `Alt+`a letter opened a
  box in the middle of the screen, over the rows one had started the search to read. It
  stands on the panel's own bottom frame now, two columns in from its left edge, which is
  where FAR draws it: the rows it covers are the last of the list, the ones already walked
  past. A panel too small to hold it still gets the box in the middle, which is at least
  somewhere it fits.

- **Deleting the file under the cursor no longer throws the cursor to the top.** The panel
  looks for the name it was on and, not finding it, fell back to row zero — `..`, the whole
  list away from where one was working. It keeps the *row* instead, so the cursor lands on
  whatever took the deleted file's place, and on the new last row when the last one went.

### Added

- **CRC32, written as a `.sfv`** (SPEC 03 §9). The fourth entry in the checksum dialog, and
  the one that is not a hash: two files with the same CRC32 are easy to make on purpose, so
  it answers *did this arrive intact* and nothing else, which is the question a `.sfv` asks.

  A list of them is written in the `.sfv` shape and never in the other one — `name hex8`,
  the digest at the end of the line, `;` for a comment, eight hex characters zero-padded, and
  `file.sfv` beside a single file — because nothing reads a `.sfv` written as `<hex>  <name>`.
  Reading is the same either way: a digest of eight characters says CRC32 as one of 64 says
  SHA-256, so a `.sfv` somebody sends is checked like any other list, and a name with a space
  in it still works because the digest is taken off the end rather than the name off the
  front.

- **Where a `descript.ion` lives, and in what code page** (SPEC 02 §13). Two settings of
  FAR's: `panel.description_names` is the list of names looked for, in order, with the first
  the one a new file is written under — a tree full of `files.bbs` no longer grows a second
  file beside each one — and `panel.description_utf8 = false` writes the file back in the
  code page it was read in, for a side file shared with the DOS-era tools that cannot read
  anything else.

  The page is the file's own and never this machine's: guessing the local one would turn a
  Russian `descript.ion` into a Latin one on a Latin machine. A file that is not there yet,
  one that was already UTF-8, and one whose page cannot hold what was typed are all written
  in UTF-8 anyway — an encoder writes a `?` for what it has no room for, and a file a DOS
  tool cannot read is better than a line that lost its words.

- **The switches beside the columns, per view mode** (SPEC 02 §3b). A `[[column_set]]` block
  now carries what the rest of FAR's panel-mode dialog carries: `fullscreen`,
  `align_extensions`, `align_folder_extensions`, `folders_upper`, `files_lower` and
  `upper_to_lower`. They belong to the mode rather than to the program, which is the point of
  them — one view of one's own draws the names with the extensions lined up in a column,
  another draws directories in capitals to tell them apart without colour, and the digit
  chooses between them.

  Each left out is what the panel drew before they existed — the extensions of *files* lined
  up, and nothing else — so a block that says nothing looks as it always did. `fullscreen` is
  `Ctrl+F1` applied by the mode: choosing it hides the other panel, leaving it gives the
  screen back, and a panel hidden by the key itself is left alone, because a mode undoing
  somebody's own keypress would be the mode reaching outside itself. `upper_to_lower` is
  FAR's switch for the 8.3 names a FAT volume hands back; a name with no letters in it is not
  "all capitals", so `README` changes and `2026` does not.

## [0.21.0] — 2026-10-29

### Added

- **A server reached through another one** (SPEC 04 §6.3). A session has a **Jump host** field
  on its SFTP page — `bastion`, `gate@bastion`, `gate@bastion:2222`, an address in brackets —
  and that is `ssh -J`: a session on the bastion, a `direct-tcpip` channel from it to the
  destination, and the destination's own SSH session inside that channel. The bastion carries
  a stream it cannot read, so the key exchange, the authentication and the files are between
  this program and the server behind it, which is the whole reason to prefer this to
  forwarding an agent.

  Each machine's host key is checked under its own name, so a first connection can ask twice —
  saying yes to the bastion is not saying yes to what is behind it. The bastion is
  authenticated with the agent or with the session's own key file, **never with the
  destination's password**: a password typed for one machine is not sent to another, and the
  error says so when there is no agent to fall back on. SFTP only, because FTP and WebDAV have
  nothing to build a tunnel out of, and a `jump` written on one of those is refused with a
  sentence rather than ignored. One hop, not a chain.

- **A table sorts and filters, and still writes nothing** (SPEC 05 §1.11). `Ctrl+F3` sorts the
  rows by the column the view is on — the one `Ctrl+←`/`Ctrl+→` walks to — ascending, then the
  other way round, then back to the order the records are in the file, because a sort with no
  third press is a sort with no way out. `Ctrl+F7` asks what a row has to hold and keeps only
  those; an empty answer gives them all back. `Ctrl+F3` is the key that sorts in the panels and
  `Ctrl+F7` the other thing one does with what `F7` looks for.

  Both are an index and nothing else: one record number per visible row, built from the records
  and thrown away to go back. Nothing is written, and every row keeps its own offset, so a
  search, a bookmark and `Alt+F8` still mean the record rather than its place in the sorted
  list. A column whose every cell reads as a number sorts as numbers — a column of sizes does
  not put 1000 before 9 — and `1,20` is read as a price while `1,200` is a thousand and two
  hundred. The status line says what the view is doing (`↑name`, `↓size`, `/tor`), and says out
  loud when a sorted table is only as much of the file as the mode read. Above 200 000 records
  both refuse and say so: a table that large is one to query rather than to sort in a viewer.

  **Editing a cell stays out**, which was the question this row had been waiting on. A viewer
  that writes is an editor, and `F4` and the hex pane are where writing lives; a cell written
  back would be a line rewritten for CSV, fixed-length records and a memo file beside them for
  DBF, and nothing honest at all for a file read only as far as the limit.

- **A drop on the other panel opens the copy dialog** (SPEC 02 §12). The third of the three
  mouse drags, and the one that was held back as a question: what a drop *means*. It means the
  dialog `F5` opens, with the destination the pointer chose — the directory under it, or that
  panel's own when the row is a file, because a file is not somewhere to copy into — acting on
  what the sweep selected on the way over, or on the row the press landed on.

  It copies nothing by itself. The gesture is imprecise by nature and starts by accident, so
  what answers for it is the dialog that was already there: the conflict policy, the filter,
  what travels with the file, and the `Esc` every dialog has. The Explorer's rule — a drag
  within one drive moves, a drag across drives copies — is the other way this could have gone:
  the same gesture doing two different things depending on where the files are, one of which
  deletes the original. It has surprised people for thirty years; FAR asks instead, and so
  does this.

### Fixed

- **`F8` reaches the system's wastebasket again.** Every job in the program runs through the
  routed filesystem — local paths to the local provider, remote paths to their connection —
  and that layer implemented the methods a server needs and left the rest to the trait's own
  refusals. A method it did not route was therefore a method the program had not got, and
  `trash` was one: deleting a local file with the wastebasket on answered *there is no
  wastebasket here*, which is the sentence meant for a server, on a disk that has one. Six
  methods were in that state and all six are routed now — the wastebasket, appending to a
  file, hard links, reading a file a chunk at a time (which is what a checksum does), the
  wipe and the search-and-replace in a file — so a local path arrives where it always should
  have. A server still refuses each of them, and now says it is a server that does.

- **An error box says the path once.** Nearly every error the engine raises already begins
  with the path it is about, and the box had just drawn that path on a line of its own: the
  name appeared twice, and three times when the hint about `Shift+Del` added it again. The
  duplicate prefix is stripped and the hint is a sentence rather than a line about a file.

- **The dialog's key bar keeps its twelve boxes.** Under a modifier a dialog answers nothing at
  all, and an empty list of labels does not mean an empty row: it takes the row off the screen
  altogether. The blank bar is twelve numbered boxes with nothing written in them now, which is
  what FAR's own dialog bar looks like — and what keeps `F5` the fifth box along wherever one is
  standing (SPEC 02 §12).

## [0.20.0] — 2026-10-29

### Added

- **A dialog has its own bottom row, and `F1` in one works** (SPEC 10 §3). FAR changes the key
  bar inside a dialog, and for the same reason the panels' bar changes with the modifier: the
  panels' keys are not on the screen, so a row still offering `Copy` and `MkFold` over a copy
  dialog is describing something that is not there. The row says only what the dialog answers —
  `F1` the help, `F10` `OK`, under `Ctrl` the `F5` that moves it, under `Alt` the `F10` that
  opens the copy dialog's folder list — and a list says `F4` where its rows can be edited, `F9`
  in the drive dialog, and `F10` as the way *out* rather than as an answer. It is short, and
  that is the honest length of it: a dialog is answered with `Enter` and `Esc`, and inventing
  function keys to fill the row out would be worse than leaving it empty.

  Writing `Help` there turned up something quieter: `F1` in a dialog did nothing at all, and
  the page it should have opened — *This dialog* — had been in the help since the help was
  written, reachable from nowhere. It opens now, with the dialog parked behind it and
  everything typed into it still there, and what the dialog was itself opened over — a file
  being read, a file being edited — stays parked under it. The help's own pages have a section
  on dialogs to follow it into, in both languages.

- **NE, LE/LX and TE in the structure pane** (SPEC 05 §1.8). Three families the pane met and
  called `MZ`: the 16-bit New Executable of Windows 1.0 to 3.11 and OS/2 1.x, the Linear
  Executables of OS/2 2.0 and of the DOS extenders and Windows 9x VxDs, and the Terse
  Executable that UEFI firmware is built out of — which has no `MZ` in front of it at all,
  because it is a PE with the stub and most of the headers cut off, so `VZ` is what is left to
  recognise. Nothing current produces any of them; people *read* them, which is what this pane
  is for.

  Each gets its header decoded field by field and the one table that says where the file's
  contents are. An NE's segment table is the one place in these families where a file offset
  falls out of the header arithmetic — a sector number shifted by `ne_align` — so those rows
  are what `Enter` follows. An LX or LE object is reached through a page table that is laid out
  one way in `LX` and another in `LE`, so the objects carry their sizes and permissions and the
  *tables* are what `Enter` follows: a page decoded against the wrong one of the two layouts
  would send a reader to a plausible wrong place, which is worse than sending them to the table
  and letting them look. A TE's sections do get offsets, because `StrippedSize` is the whole of
  the arithmetic — the section headers still carry the offsets the PE had — and the mapping
  from a virtual address follows the same sum, so `va:` in the viewer works there too.

  What none of the three gets is the equivalent of the PE reader's import and export walks:
  those want a second format each, and nothing here can be tested against a real file of them.
  There is no separate TE64 either: the header always carries a 64-bit image base, and
  `Machine` is what says x86-64 from IA-32.

- **Two of the three mouse drags** (SPEC 02 §12). A drag from a row selects every row it
  sweeps, in the panel the press began in, and leaves the cursor where it stopped; a drag on
  the frame where the two panels meet moves the rule between them, within the same limits
  `Ctrl+←` and `Ctrl+→` keep, so the two ways of moving it cannot disagree about how narrow a
  panel may get. Which of the two a drag *is* was decided when the button went down, so one
  gesture cannot turn into the other halfway, and a press on the frame alone does nothing — a
  click on a frame never did, and a press that moved the split would be a trap.

  The sweep only ever selects. A drag that overshoots and comes back leaves what it passed
  over selected, which is the right way round for a gesture nobody aims precisely: the right
  button takes one row out again and `Gray *` turns the lot round, while unselecting on the
  way back would mean remembering the selection the drag started from — and a selection the
  mouse quietly threw away is worse than one row too many.

  The third drag, a selection dropped on the other panel, is left on purpose: what a drop
  means is a question rather than a gap. The Explorer moves within a drive and copies across
  one, a rule that has surprised people for thirty years; FAR asks instead. Until that is
  answered a drop would be a destructive operation started by a slip of the hand, and `F5`
  with two panels open is already the short way to do it.

- **`Alt+F10` picks a path for the row that asks for one** (SPEC 10 §3). The copy dialog's
  destination could be chosen from the folder tree; a row that names a *file* — the key file of
  a stored connection — was typed by hand, and the backlog had wanted a `...` button opening a
  panel inside the dialog for it. It is one key and two lists instead: the tree of folders for
  a row that wants a folder, which the extraction dialog's destination now gets as well, and
  one directory at a time for a row that wants a file.

  The file list is a list like every other choice in this program, so it filters as you type
  like the rest of them: `..` above, the directories next, the files after them, `Enter` on a
  directory walking *into* it rather than answering — so a list of one directory reaches the
  whole filesystem — and `Esc` leaving the row exactly as it was. Everything in the directory
  is listed, hidden names included, because `~/.ssh` is exactly where the key file this was
  written for lives. Local only, for the reason the folder list is local only: a listing per
  keystroke over the wire is not a dialog, and a path on a server is typed with `Tab` to
  complete it. The key bar under `Alt` says which of the two the row would open.

- **Round boxes, where one answer excludes the others** (SPEC 10 §3). A row that cycles its
  values in place is right for a long list — thirty code pages do not belong on the screen at
  once — and wrong for a short one: *overwrite / skip / rename* are three answers to one
  question, and a reader who cannot see the other two has to press a key to find out what they
  are. So there is a `(•)` row now, each row of a group carrying the same configuration key and
  the value it stands for, `Space` or `→` picking the one under the cursor and putting the rest
  of the group out. Round where a box that stands on its own is square, so the shape says which
  it is before the label is read.

  The copy dialog's *what if it is already there* is the first group: six boxes instead of one
  row that cycles through six values, because *rename* and *append* are the two nobody finds by
  pressing `→` hopefully.

- **`Ctrl+F5` moves a dialog** (SPEC 10 §3). FAR's own key, for FAR's own reason: a dialog
  covers the thing it is about — the file under the cursor, the panel one is copying from — and
  sometimes that is the one thing one needs to read. The arrows push it about while it is
  moving, `Enter` and `Esc` only *stop* moving it (neither answers nor closes it, because a key
  that threw the dialog away while one was pushing it about would be a trap), and where it was
  pushed to is forgotten when it closes, so the next one opens where it belongs. It is clamped
  to the screen rather than pushed off it.

- **`Tab` completes a path in a dialog** (SPEC 10 §3). The destination of a copy is typed far
  more often than it is chosen from a tree, and `/usr/lo` is most of `/usr/local/`: `Tab` on a
  row that names a path now finishes it the way a shell does. One match is written out whole —
  with its separator when it is a directory, so the next `Tab` goes inside it — several are
  taken as far as they agree and the status line says how many there were.

  Where there is nothing to add, `Tab` moves on through the dialog as it always did, so the
  key never stops working; and it reads one directory, the one the half-typed path names, because a
  dialog's `Tab` should not start a walk. The rows that complete are the ones that name a
  path: a copy's destination, an extraction's folder, an archive's name, a key file. A mask or
  a colour is not a path, and a `Tab` that tried to complete one against the filesystem would
  be a `Tab` that does nothing where it used to do something.

### Changed

- **`Tab` walks the whole dialog** (SPEC 10 §3). It went to the buttons and stayed among them:
  somebody who had tabbed as far as `Cancel` to read it had no way back to the field they
  wanted to correct except an arrow key they had no reason to try. Now the ring is the rows of
  the page that is open, then the buttons, then the first row again, with `Shift+Tab` the other
  way round — which is why one key out of the first row is the last button, the way out. On a
  row that names a path `Tab` still completes it first, and only moves on when there is nothing
  left to add.

## [0.19.0] — 2026-10-29

### Added

- **Every list filters as you type** (SPEC 10 §2b). Four of them did — the folders and the
  three histories — and the rest were read by scrolling, which with thirty-odd code pages or
  four hundred plugin commands is slower than typing part of what you want. All of them do
  now, and three details make it a feature rather than a change of key:

  `Esc` clears what was typed **before** it closes the list, and a second `Esc` closes it:
  throwing away the list and the cursor's place in it to get out of a filter is a key that
  does too much. Where the rows have keys of their own — the drive dialog's letters, a user
  menu's digits — the letters stay theirs and `Alt` and a letter is the filter, because those
  keys are what somebody who uses them twice a day presses. And typing comes after every key
  a list has of its own, so `Space` in the settings browser still flips the setting.

  With it, `Alt+←/→` slides the rows sideways and `Alt+Home` puts them back: a folder history
  of deep paths is wider than its box, and eliding the middle of every line is not the same as
  being able to read the end of one.

- **The wheel is as many rows as you say, per area, and the sideways wheel works** (SPEC 02
  §12). Three rows a notch was a constant; it is three settings now, as it is in FAR, because
  a notch that is right for a file listing is too much in a page of prose: `ui.wheel_lines` for
  the panels and the lists, `ui.wheel_lines_viewer`, `ui.wheel_lines_editor`. Three by default
  and never more than twenty, because a number nobody should be able to set is clamped rather
  than trusted.

  The sideways wheel — a trackpad has one, a mouse often does not — scrolls what has anything
  to scroll sideways, by as many columns as the wheel moves rows: a long line in the viewer, a
  long line in the editor. Over a panel it does nothing, and that is the right answer rather
  than a gap: a panel's columns are a layout, not a window onto something wider, so there is
  nothing there to move.

  It also fixes something quieter. The wheel and the click *inside* the viewer and the editor
  had nothing to hit: the areas they are drawn in were never recorded, so a notch over a file
  being read did nothing at all whatever the documentation said. They are recorded now — after
  the scrollbar and the structure pane have taken their columns, so a notch over the pane is
  not a notch over the text.

- **Raw devices on Windows** (SPEC 05 §1.10). `F3` and `F4` on `/dev/sdb` have worked since
  0.0.8 because on Linux and macOS a disk *is* a file; on Windows it is not, in three separate
  ways, and the backlog had carried all three. They are done:

  The **name**: there is no `/dev`, so a disk is `\\.\PhysicalDrive0` and a volume `\\.\C:`,
  names the filesystem does not list and which are therefore typed rather than arrived at from
  a panel. The **length**: `metadata` on such a name fails — it asks the filesystem about
  something that is not in it — and seeking to the end answers nothing, so the device is asked
  (`IOCTL_DISK_GET_LENGTH_INFO`, with the drive geometry as a second try for a device that only
  implements that). And the **reading**: such a handle refuses a read that does not start on a
  sector boundary or is not a whole number of sectors long, for a call that works on any file,
  so the sector size comes back with the length and every read is widened to the sectors around
  what was asked for.

  The viewer and the hex pane read through one function for this, so they cannot disagree about
  it, and its arithmetic is tested against an ordinary file: reading *through* the sector path
  gives exactly the bytes reading without it gives, at every offset and every length. A refused
  open says it needs Administrator rights instead of leaving the system's "Access is denied" to
  be puzzled over, and the `unsafe` is in `fcmd-platform` with everything else that needs it.

### Changed

- **`Ctrl+A` over several files no longer makes them agree** (SPEC 03 §7). The attributes
  dialog read the mode of the file *under the cursor* and wrote it onto everything selected:
  mark a hundred files with different permissions, press `OK` to change one bit, and the other
  eleven went with it. FAR has had the answer to this since forever and the spec has promised
  it since the beginning — a three-state box — and this is it.

  A box the selection does not agree about opens at `[?]`. `Space` walks it through *on*,
  *off* and back to *leave it alone*, so a box pressed by mistake can be put back to changing
  nothing, which a two-state box cannot offer. The boxes they do agree about are ordinary ticks
  and answer for all of them. The same holds for the read-only box both platforms have, and for
  Windows' own hidden, system, archive, temporary, offline and indexed bits.

  Underneath, what travels to the engine is two masks — the bits to set and the bits to clear —
  instead of one whole mode, and a bit in neither mask keeps the value each file has. Over one
  file every box is answered, so the two masks cover all twelve bits and come to exactly what a
  whole mode meant; over a selection they are the difference between a dialog that asks and a
  dialog that assumes. A server's files go the same way, with the mode read first.

## [0.18.0] — 2026-10-29

### Added

- **The kernel can make a copy** (`general.system_copy`, SPEC 03 §3b). The row was in *System
  settings* as a dim one. Reading a file into a buffer and writing it back is still the
  default and still earns it — progress every chunk, `Esc` answered within a few kilobytes,
  the same behaviour on every filesystem — but it is the one thing that cannot let the system
  copy the file itself, and on the filesystems people keep large files on that is the whole
  difference: `copy_file_range` on Linux, a chunk at a time, which is a reflink on btrfs, XFS
  and bcachefs and a server-side copy over NFS 4.2; `fcopyfile` on macOS, which clones on
  APFS; `CopyFileExW` on Windows, which knows a share's own server-side copy.

  It is a faster copy, not a second copy engine. It writes the same `name.fcmd-part` file, so
  the rename into place, the permissions and the times are the engine's own afterwards; a
  platform that will not take the call answers *no* rather than an error and the byte loop
  does the work with nobody told anything, because nothing went wrong; and only a failure an
  ordinary copy would hit as well is reported as one. Every call that needs `unsafe` or a
  `#[cfg]` is in `fcmd-platform`, as the workspace's rule has it.

  Off by default, because the cost is told in the same breath as the gain: where the kernel
  copies a whole file in one call — macOS and Windows — `Esc` is answered when that call
  returns rather than within the next few kilobytes, and that file's progress arrives in one
  piece. On Linux the copy is still made a chunk at a time, so neither applies there.

- **The cursor may go past the end of a line** (`editor.beyond_eol`, SPEC 10 §3.6). The row was
  in *Editor settings* as a dim one, because the setting did not exist: every movement and every
  insert has to agree about a column that is not in the text. They do now. Off — the default, as
  in FAR — the arrows stop at the last character and `Right` there goes to the next line. On,
  the columns to the right of the text are somewhere to be, which is what one wants to line up a
  column of numbers or to mark the same columns down lines of different lengths.

  The bargain is that moving about out there writes nothing. The spaces appear at the keystroke
  that puts a character in one of those columns, as part of that one edit, so a single `Ctrl+Z`
  takes the character and its padding away together — a file does not quietly grow trailing
  space because somebody moved the cursor through it. `Backspace` and `Del` out there move the
  cursor and leave the file alone; `Del` in particular does not pull the next line up from three
  columns past the end of this one.

- **AArch64 is disassembled in the program** (SPEC 05 §1.6). `F5` decoded x86 and x86-64 and
  sent everything else to `objdump`, which on an Apple Silicon laptop or an ARM server means
  the viewer could not read the binaries actually on the machine. It reads them now, through
  `yaxpeax-arm` — pure Rust, so the Windows and macOS builds need no C toolchain — and
  everything built on the sweep comes with it: `Enter` follows a branch and `Backspace` comes
  back, `F7` searches the instructions rather than the bytes, and the structure pane names the
  symbols beside them.

  Two differences from x86 are the whole of the decoder's character, and both are deliberate.
  An instruction is four bytes, always: so a word nothing decodes is **one** `(bad)` line
  rather than four — there is no sync to lose in a fixed-width instruction set — and stepping
  backwards is arithmetic instead of decoding forward from a guess. And a branch is shown as
  the address it arrives at, not as the displacement the encoding holds: `b $+0x14` is what
  the bytes say, `b 401014h` is what the reader wants, and it is what `Enter` follows.

  The assembler stays x86. Reading an architecture and writing it are now separate questions
  in the code (`Arch::disassemblable` and `Arch::assemblable`), so `F9` in the hex pane on an
  ARM binary says what it cannot do rather than writing four bytes of x86 into it and showing
  a plausible listing of them.

- **The assembler writes addresses** (SPEC 05 §2.8). `F9` in the hex pane assembled registers,
  numbers and branches, and refused anything in brackets by name — which left out half of what
  a patcher reads. `mov eax, [rbx+8]` is the line one has just looked at on the screen, and
  retyping it is the job. So the brackets are now assembled: `[base]`, `[base+disp]`,
  `[base+index*scale+disp]`, a bare `[0x401000]`, and `[rip+0x2e35]` with the displacement
  counted from the end of the instruction, the way the disassembler prints it. The pieces may
  be written in any order, because that is how one retypes a line, and the numbers inside go
  through the same calculator as everywhere else. `lea` arrived with them, as the one
  instruction whose operand is the address itself, and so did the memory forms of `mov`, the
  arithmetic group, `test`, `xchg`, `inc` and its neighbours, `push`, `pop` and an indirect
  branch.

  How wide the thing at an address is, is the part that has to be right: the register on the
  other side says so, or a size word in front of the brackets does — `byte`, `word`, `dword`,
  `qword`, with masm's `ptr` for whoever writes it that way. When neither says, the
  instruction is refused with that sentence instead of guessed at, because `mov [rbx], 1`
  could write one byte or eight and the difference is somebody's binary; a size word that
  contradicts the register is refused too. The encoding rules that are easy to get wrong —
  `rsp` and `r12` needing a SIB byte, `rbp` and `r13` needing a displacement that is there
  even when it is zero, an index with no base, `rsp` which cannot be an index at all, and the
  machine-word moves that must *not* carry a `REX.W` — are pinned down by their bytes, and
  everything still goes back through the disassembler before the dialog shows it, so a bug in
  the encoder is a refusal rather than a corrupted file.

## [0.17.0] — 2026-10-29

### Added

- **A host key that changed says where the old one is written** (SPEC 04 §6c). Accepting a key
  nobody has seen was already a question with the fingerprint in view; a key that is *recorded
  and different* was a refusal and a sentence, and then you had to go and find
  `known_hosts` yourself. Now the refusal is the same — a rebuilt server and somebody in the
  middle look exactly alike from here, and nothing accepts a changed key — but the dialog
  names the file and the line the old key is on, and its other button opens that file in the
  editor **on that line**. *Cancel* is under the cursor: this is not a question to answer by
  pressing `Enter` without reading it. Taking the line out stays the reader's own act, which
  is what `ssh` makes one do, for the same reason.

- **A large file goes up to object storage in parts** (SPEC 04 §8.3b). One `PUT` is the better
  answer while it works — one signature, one round trip, nothing left behind if it fails — and
  it stops working at five gigabytes, which is the service's own limit; well before that it is
  an all-or-nothing bet on a long transfer, an hour of upload thrown away by one dropped
  connection. So a file over 64 MiB is now cut into 8 MiB parts, each signed over its own
  bytes and read from where it lies in the file rather than copied, and the part size doubles
  as often as it takes to fit the file into the ten thousand parts the service allows. A
  failure aborts the upload — with a cancellation token of its own, so `Esc` cannot cancel the
  cleanup as well — because until the completion arrives the parts are stored and the object
  is not, and stored parts are a charge on somebody's account that no listing shows. The
  completion's own answer is read, too: it is the one request that can fail inside a `200`.
  From outside nothing changes: the answer is the number of bytes written, whichever way they
  went. `--set`-able limits, so the tests drive the whole path with a thirty-byte file.

## [0.16.0] — 2026-10-29

### Added

- **The clock in the corner** (`ui.clock`), FAR's own spot: `HH:MM` in the top right corner
  over the panel frame, or at the end of the menu bar when that is always on, costing the
  panels no row. It is the last dim row in *Interface settings*, and the reason it was dim is
  the interesting part: the event loop waits for something to happen rather than polling, and
  a clock is the one thing on screen that changes with nobody pressing anything. So the wait
  now has a deadline when the clock is on — it ends when the minute turns, which is one redraw
  a minute rather than the fifty-nine a second-hand would need. Off by default, because over
  `ssh` or inside a multiplexer that redraw is traffic somebody is paying for. The screen
  saver and the window title can use the same deadline when they come.

- **Every setting, as a list** (*Options → All settings*). The dialogs group the dozen
  settings one changes often and say what each means; this is the other seventy, which is what
  FAR reaches by typing `far:config`. It is generated from the configuration itself — the file
  is dumped and each scalar becomes a row — so a setting added to the program is in the list
  the same day and a list that drifts from it is impossible. Typing narrows it, `Enter` or
  `F4` types a value, `Space` flips a `true`/`false` one, `Del` puts back the value the
  program was built with, and a row that is no longer that is marked with a `·`. Every value
  goes in through the same function `--set key=value` uses, so a word a setting cannot take is
  refused with a reason rather than written and found out later; a test holds the two ends
  together, so the list can never offer a setting nobody can change. The `[[blocks]]` — the
  highlighting and sort groups, the mask groups, the sites, the column sets — are left out:
  each has an editor of its own. The other half of that work is that `--set` now reaches every
  setting too: the comparison options, the drive dialog's columns, the archive limits and the
  shell prompt were readable in the file and not settable by name.

- **The screen grabber** (`Alt+Ins`, from any screen). The copy keys take what the program
  *knows* — names, paths, a block of a file; this takes what it *drew*, which is the set with
  no other answer: a process card, a help page, a hex dump, the sentence a comparison ended
  with. The terminal's own selection is worse at it — it takes the frame, the column rule and
  the padding, and over `ssh` it takes whatever the far end last painted. It holds the screen
  as it is, so moving about cannot change what is being copied: the arrows move the cursor,
  `Shift` with them marks a rectangle **from where the cursor already was**, `PgUp`/`PgDn`,
  `Home`/`End` and `Ctrl+Home`/`Ctrl+End` cover the rest, `Ctrl+A` marks the whole screen,
  `Enter` or `Ctrl+Ins` copies and closes, `Esc` closes and copies nothing. With nothing
  marked it takes the row the cursor is on. Trailing blanks come off each line, a blank row
  inside the rectangle stays, and a double-width character is one character rather than its
  two cells. It also answers, in the one way that works in the hex dump and the table too,
  the backlog's *select and copy text in the viewer*.

- **Two settings that were dim rows.** *Always show the menu bar* (`ui.menu_bar`) keeps the
  menu titles on the top row and starts the panels under it, so it costs a file the way the
  column titles do, and `F9` lights a title and opens its menu on that same row. *Save the
  setup on exit* (`ui.save_setup_on_exit`) writes `fcmd.toml` on the way out, as *Save setup*
  writes it. Both are off by default, as they are in FAR: the row is a row of files, and a
  setting changed to see what it does should not become permanent by quitting.

- **`Ctrl+W` is FAR's task list**, and so is the *Task list* row in the Commands menu: the
  operating system's processes, which is what that key means in FAR — this program's own
  screens are `F12`. It opens the process panel the change-drive dialog has opened since
  0.10.0, so `F3` on a row still shows that process's card. On a line with something typed on
  it `Ctrl+W` stays the word-delete every readline has, as `Ctrl+K` stays kill-to-end.

- **`Options → Code pages`** opens the code page list, so the last dim row but one in that
  menu is a real one. It is the viewer's own `Shift+F8` list (`opt.codepages`), opened where
  there is no file to read: what it is for there is marking the three or four pages `F8` walks
  and giving one a name of its own, which is what FAR's dialog of that name is for too. One
  list rather than a second screen showing the same thirty-odd rows.

## [0.15.0] — 2026-10-29

### Added

- **Two files side by side** (`Ctrl+Shift+D`, and *View file differences* in the Commands
  menu). *Compare by content* says which of a hundred pairs differ; this says what differs in
  one pair — the gap a commander coming from Midnight Commander or Krusader notices first.
  The file under each panel's cursor, two columns with their own line numbers and a rule
  between them, and inside a changed line the part that actually changed marked in the
  selection's colours. `Alt+↓` and `Alt+↑` walk the differences, which is the key one presses
  forty times; the arrows, `PgUp`, `PgDn`, `Home` and `End` are the rows, `←`/`→` scroll both
  columns together, `Tab` changes which column one is reading, `F1` opens the chapter and
  `F12` lists it with the other screens. `W`, `C` and `E` ignore blanks, case and line endings
  and line the two files up again at once, so one watches the answer change; the line endings
  start from what the comparison dialog was last told, so the two agree about what "the same"
  means. Two files that are the same say so rather than leaving one scrolling for a difference
  that is not there. The alignment is patience diff in `fcmd-core`, with no dependency: the
  lines that occur once on each side are the anchors, the longest rising run of them is kept,
  and a region with no unique line left is finished with an exact table under four hundred
  lines. A moved function therefore reads as moved. Local files only, 16 MiB a side: the whole
  of both has to be in memory, and a silent download of two hundred megabytes is not what the
  key looks like it will do.

- **The rest of FAR's insertion keys.** `Ctrl+Enter` and its neighbours are now a table
  rather than four keys: *what* about a panel — the name, the path, the full name, or the
  full name in the form another machine could use — and *which* panel, the active one, the
  passive one, or the left or right one whatever has the focus. FAR's own keys are bound
  (`Ctrl+F` for the full name, `Ctrl+Alt+F` for its network form, `Ctrl+Shift+[`/`]` and
  `Ctrl+Alt+[`/`]` for the left and right panel), and a path on this disk has no network form,
  so that key says so rather than inserting a local path under a name that promises more.

- **`Masks select folders too`**, FAR's *Select folders*: `panel.select_folders`, off by
  default as in FAR, and a row in *Panel settings*. The keys that name a mask leave
  directories alone unless it is on — one types `*.rs` to select the sources — while *select
  all* and *invert all* include them either way, because they name no mask.

- **A file opens where it was left.** `F3` on a file you have read before puts back the
  position, the code page, the mode, the wrap and the ten numbered bookmarks — FAR's own
  habit, and the dim row in *Viewer settings* is a real one now (`[viewer] remember`, on by
  default). It lives in the state file, newest first and capped at five hundred and twelve
  files; each answer is put back on its own, so a file remembered in a mode this build no
  longer has still opens at the right offset. Somebody who reads log files and wants the end
  of every one turns it off.

- **The code pages are a list, and `F8` walks the ones you mark.** `Shift+F8` in the viewer
  opens every code page the program offers — the five it handles itself and everything
  `encoding_rs` knows: the Windows pages, CP866, the two Macintosh ones, the ISO-8859 family,
  KOI8-R and KOI8-U, Shift_JIS, EUC-JP, ISO-2022-JP, GBK, GB18030, Big5 and EUC-KR. `Enter`
  reads the file as one, `Ins` marks a page as one `F8` walks (shown with a `*`, and first in
  the list), `F4` gives it a name of your own. `F8` then cycles the marked ones rather than a
  fixed five — thirty-odd pages is not a cycle — and with nothing marked it walks the five it
  always did. Both answers live in `[[codepage]]` blocks that *Save setup* writes; an id is
  the WHATWG name, and `cp1251`, `CP-1251` and a bare `1251` all name the same page.

## [0.14.0] — 2026-10-29

### Added

- **The mask groups have an editor, and can be written down.** `<arc>`, `<temp>` and `<exec>`
  were three fixed lists; now a `[[mask_group]]` block adds a group of your own — `<images>`,
  `<sources>` — or replaces one of the three, which is how `<arc>` learns about `.cbz`.
  `F9 → Options → File masks` is the list: what each group stands for, `Ins` for a new one,
  `F4` to edit any row (a built-in one included), `Del` to take a block away. Everything
  compiled from a mask is compiled again at once — the colour groups, the sort bands, each
  panel's filter — and *Save setup* writes the blocks.

- **The dialogs' own fields keep a history.** `Ctrl+↓` on a field one types the same thing
  into again — the search mask and text, the replacement, where to copy to, where to unpack,
  a group's masks — opens what has been typed there before, as the same list one types into,
  with the same `Ins`, `Shift+Del` and `Del`. `Enter` puts the row in the field and `Esc`
  leaves it alone; the dialog waits behind the list either way. Every dialog's `OK` feeds
  them, so nothing has to remember to.

- **A history line can be pinned, and a history has a lifetime.** `Ins` in any of the three
  histories pins the line under the cursor — marked with a `*` — and neither the size limit
  nor the new `ui.history_lifetime` can push a pinned line out. `Shift+Del` takes one line
  away and refuses a pinned one; `Del` empties the list of everything nobody has pinned,
  after asking. Those are FAR's own three keys. The lifetime is FAR's own as well: ninety
  days by default, counted from when a line was last used rather than from when it was first
  typed, `0` to keep everything for ever, and swept once when the state file is written.

- **Descriptions wherever the panel stands.** A `descript.ion` on a server or in a bucket is
  now read and written like one on this disk: the description columns, the information panel
  and `Ctrl+Z` work in a remote panel, and the file is fetched only when something on screen
  shows a description, which is what kept the habit off a network share in the first place.
  Inside a container it is read and not written — that would be a rebuild of the whole
  archive — and `Ctrl+Z` says so.

- **A search result carries its descriptions.** A copy, a move or a delete whose files came
  from several directories at once — which is what `Alt+F7` leaves in the panel — now takes
  each line out of the directory it was in rather than none of them: the sources are grouped
  by their parent, and each side file is read, changed and written on its own.

- **The histories are lists one types into.** The command history, the folder history and the
  files read or edited all narrow as you type, with the frame saying what was typed, and the
  keys above work on what is left without closing the list.

- **`Alt+F10` finds a folder by name.** A list of every folder the tree knows under the
  panel's root, narrowed by what you type — the frame shows the filter — with `Enter` to go
  there. It opens on the tree's cache, which **now survives the session** (written into the
  state file, capped at twenty thousand directories), and one bounded walk fills the rest in
  behind the dialog, breadth first so that a walk stopped by its limit has read the
  directories near the top. The copy dialog's own `Alt+F10` opens the same list and puts the
  folder chosen into the destination — FAR's *Tree* button, under another key because `F10`
  here is `OK`.

### Fixed

- **`Tab` is the way out of the quick view as well as the way in.** In the viewer `Tab` moves
  the keys between the content and the structure pane, and the preview was taking the key even
  with no pane open — so the focus went into the quick view and stayed there. With no pane to
  move between, `Tab` is the panels' own key again.

## [0.13.0] — 2026-10-28

### Added

- **An archive inside an archive, and an archive on a server.** `Enter` on a `.zip` that is
  itself inside a `.tar.gz`, or on one in a remote panel or a bucket, now opens it. Every
  decoder here reads a file and seeks in it, so a container that is not a file on this disk is
  copied into `<temp>/fcmd-<pid>/nested/` first — once, kept for as long as what it came out
  of has not changed, and thrown away when the program ends. The path stays what it is,
  `sftp://srv/site.zip!/inner.tar.gz!/etc/nginx.conf`, because that is where the file really
  is: the copy happens below the VFS, so the panel, the viewer, the finder and the job engine
  all got this at once. `[arc] max_depth` stops the recursion, the new `[arc] max_spill_mb`
  (default 1024) stops the disk filling, and both refuse by name. Such an archive is read-only
  — a rebuild of the copy would report a change to a file that is about to be deleted — and
  says so rather than half-writing anything.

- **An encrypted archive asks for its password.** A 7z whose contents — or whose header, so
  that not even the names show — are encrypted now opens: the panel and the viewer answer the
  refusal with a masked prompt, and what is typed goes into the container cache, which keeps
  it for as long as the program runs and writes it nowhere. The listing and every entry opened
  out of that archive then ask nothing; a wrong password asks again.

- **Solid blocks in the archive dialog.** `Shift+F1` has 7-Zip's own *solid* box: ticked, the
  files of a 7z share one compressed stream, which is smaller. A block closes at 64 files or
  128 MB, so reading one file out of the archive never decodes a whole tree, and one file per
  block stays the default for the same reason. A tar is solid anyway and a zip cannot be, so
  both ignore it; the box is remembered for the next dialog.

- **The search looks inside archives.** `Alt+F7` has FAR's *Inside archives* box, off until
  it is ticked: with it, the walk steps into every container it can read and matches what is
  in them by name and by content. `x.zip` is walked as `x.zip!/`, so the mask, the size
  condition, the depth limit and the text scanner work in there exactly as they do on the
  disk, and a hit is an ordinary `src.zip!/doc/readme.md` that `Enter`, `F3` and `F5` already
  understand. It goes through the same container cache as the panels, so a nested archive or
  one that wanted a password behaves the same way; an entry over 64 MB is reported as a
  problem rather than read into memory.

- **`Ctrl+A` works inside an archive.** It asks the two questions the formats can answer —
  the permissions and the modification time — and `OK` writes the archive again with the
  entry's bytes and everybody else's untouched, through the same transactional rebuild as
  every other change. A dialog nobody touched writes nothing: the boxes are seeded from what
  the listing knows, and only what was changed is sent.

- **Descriptions are read inside an archive.** A `descript.ion` in a container is an entry
  like any other and the index is already in hand, so `Ctrl+6`, `Ctrl+7` and the information
  panel show what it says in there as they do on the disk. Writing one back is a rebuild and
  `Ctrl+Z` still says so.

- **The sort groups have a dialog.** `F9 → Options → Sort groups` edits the `[[sort_group]]`
  bands the way the colour groups are edited: the list in the order they cut the listing,
  `Ins` beside the row the cursor is on, `Del`, `F4`, `Ctrl+↑↓` to move one past its
  neighbour, and one dialog per band — a name, the masks, and whether it sits in front of the
  ungrouped files or behind them. Both panels re-sort at once; writing the first band turns
  the grouping on and taking the last one away turns it off, and *Save setup* writes the
  blocks.

### Changed

- **The menu letters are FAR's own.** Every row the `F9` menus share with FAR now wears the
  letter FAR marks for it, read out of FAR's own `far/farlang.templ.m4` in each of the five
  languages both programs speak — Russian, English, German, Spanish, Italian. The wording is
  still ours; only the marker moved, onto the occurrence of FAR's letter nearest where FAR
  marks it. `F9 F a` is *file attributes* in both programs again, and a hand that knows FAR
  does not have to learn a second set. French, Chinese and the rows FAR has not got — the
  tabs, the tunnels, the checksums — keep the rule this program had, and give way where one
  of them would claim a letter FAR has already taken.

## [0.12.0] — 2026-10-28

### Added

- **`Ctrl+A` says everything a file is.** The attributes dialog gained the two times FAR
  shows and we did not — *Created* and, on Unix, the dim *Record changed* that no system can
  set — a *Times in UTC* box that re-reads all of them in that clock, and FAR's three buttons
  under them: *Original*, *Current* and *Blank* fill the rows in and leave the dialog open. It
  now names who the file belongs to, and on Windows offers *temporary*, *offline* and whether
  the indexer may look at it, with a dim line for what the file also is — compressed,
  encrypted, sparse, a reparse point — which are done rather than ticked. The creation time is
  written where there is a call for it, which is Windows.

- **The quick view is the viewer, not a preview of it.** Give the preview panel the focus and
  its keys are the reader's: `F2` wraps, `F4` walks the ways of reading a file, `Shift+F4` is
  hex, `F5` disassembles, `F6` decompiles, `F7` searches, `F8` changes the code page, `F11`
  opens the structure pane — and the key bar says so. Letters still belong to the command
  line, and the keys that would leave the file stay with the panel. Beside a process, socket
  or programs list the preview holds that row's card, which is the page `F3` opens there.

- **A services panel, in the change-drive dialog.** `Alt+F1`/`Alt+F2` → *Services* lists what
  the machine runs in the background with the state of each, `F3` opens its card, and `F5`,
  `F6` and `F8` start, restart and stop it — the last two having asked first. systemd on
  Linux through `systemctl`, the Service Control Manager on Windows through its own API,
  `launchctl` on macOS. Nothing escalates: an action the system refuses is reported as needing
  root, which the program will not ask for on anybody's behalf, and the list is read again
  when an action comes back so the panel shows the state the service is in now.

- **The process card is a page now, as full as FAR's own.** `F3` on a row of the process list
  used to answer with a dozen rows in a dialog. It answers with a page in the viewer: what it
  is (owner and group, the SID on Windows, session, priority, threads, bitness, when it
  started and how long it has been up), what it has taken and spent (resident, private,
  mapped and swapped-out memory, page faults, processor time in its own code and in the
  kernel, handles held, bytes read and written), where it came from (executable, directory,
  the whole command line), its environment, and every library it has loaded. It is in the
  viewer because the last two lists are longer than any screen — so it scrolls, searches and
  wraps like any other page. `F3` on a row of the socket list opens the same page for whoever
  holds the socket, and on a row of the service list it is that service's card. Every row only where the system answers it: somebody else's
  process keeps its secrets, and the card says nothing rather than guessing.

- **An empty list of connections is still a list.** `Alt+F1` → NetBox with nothing stored
  used to refuse to open and say so in the status line, which told nobody where they were or
  how to make the first session. It opens as a panel now, empty, and its key bar says what
  the keys do there: `F4` edits the session under the cursor, `Shift+F4` makes a new one,
  `F8` forgets one — and the bar no longer offers `Copy` and `MkFold` on a list that has no
  files in it.

### Changed

- **The help is drawn by the viewer's Markdown view.** `F1` used to render its pages with a
  little wrapper of its own that knew a heading, a bullet and an indented block, and showed a
  heading in capitals. The pages are Markdown and the viewer already renders Markdown, so it
  renders them now: headings with a rule under them, real bullets, tables drawn with box
  characters, the inline marks taken off and the words under them coloured instead. The
  generated chapters — the keys, the commands, the settings — are written as Markdown too, so
  one renderer draws the whole book, and the dialog has inks of its own for it
  (`help.heading.fg`, `help.code.fg`, `help.marker.fg`, `help.link.fg`, `help.frame.fg`).

- **The menu bar wears its letters, and `F9` lights it without covering anything.** Each of
  the five titles is drawn with the letter it answers to in the bar's own ink, as FAR draws
  them, and that letter is now a key: `F9` then `F` is the files menu. `F9` on its own lights
  the bar and leaves the panels where they are — `Enter`, `↑`/`↓`, a title's letter or a click
  pulls a menu down, `Esc` rolls it back up, and a second `Esc` gives the panels back. Inside
  an open menu the rows keep the letters, as they do in FAR. The letter of the row one is
  standing on keeps its colour too, instead of disappearing into the cursor.

- **The lists one picks from are drawn in the menu's colours**, as FAR draws them: the drive
  dialog, the histories, the sort modes and the bookmarks are cyan with the key in yellow and
  the row under the cursor on black, instead of looking like a settings dialog. The two are
  different things — one is a list to choose from, the other a form to fill in — and in FAR
  they have looked different for thirty years.

- **The size of a folder, measured and kept.** `Space` (and `F3` on a directory) measured one
  folder and said the figure in the status line, where it was gone a moment later. It now
  measures **every selected folder** — finding out which of five trees is the big one is the
  whole reason the key exists — and the answer becomes that directory's own size: the panel
  draws the figure in the size column **in place of the word `Folder`**, as FAR does, the
  selected-totals bar counts it, and sorting by size puts the big tree where it belongs. A
  flag says the figure was measured, because a directory has a size of its own — four
  kilobytes of it on most filesystems — and drawing that would answer a question nobody asked.

- **Autocomplete on the command line** (SPEC 06 §7.4), the largest thing the line was still
  missing. Four sources, each one a setting with three states — on, off, or only when
  `Ctrl+Space` asks: the names in the directory the panel is showing (free, because that
  listing is already on screen), the whole line again out of the history, the executables on
  the `PATH` (gathered once, and waiting to be asked by default, because gathering them reads
  every directory on it), and the environment's variables for a word that starts with `$` or
  `%`. The first word of a line completes as a command — what the program runs itself, the
  prefixes, the `PATH` — and the rest as files. The list is drawn above the line, over the
  bottom of the panels, since below it are the status line and the key bar. It owns four keys
  and no others: `↑`/`↓` walk it, `Tab` takes a row, `Esc` closes it, and `Enter` still runs
  the line until somebody has walked into the list.

## [0.11.0] — 2026-10-28

### Added

- **Columns of your own, for any of the ten views** (SPEC 02 §3a). FAR has no separate keys
  for custom panel modes — its ten modes *are* the editable ones — so neither have we: a
  `[[column_set]]` block whose `id` is a view's digit is what that view draws, and a digit
  nobody has written a set for keeps the built-in list. Twelve column types, a width that is a
  number, a percentage, `*` or automatic, an alignment and a format per column: `bytes` writes
  a size out digit by digit, and a date takes a `strftime` string. A column the panel has no
  room for is dropped from the right, and a set that would leave the name unreadable drops
  another, which is the rule the detailed view already followed with its extra dates.
  **FAR's own `Column types` and `Column widths` strings are read as well** (`far_types`,
  `far_widths`), so a mode out of somebody's FAR configuration can be pasted in; what we have
  no column for — the stream size, the change time, the stream count — is named on the startup
  screen rather than dropped in silence. A set that asks for the owner, the group, the link
  count or the description puts the same worker to work as the view that shows it, and a view
  whose set asks for none of them pays nothing even though the mode it replaced would have.

- **The help is read in the language the program is running in.** The chapters' names are
  message ids in all seven catalogues, so the index is always the reader's own; the prose of a
  chapter is read from `docs/help/<lang>/<file>.md` when somebody has translated it and from
  the English otherwise, with one line above it — in that language — saying that is what it
  is. **Italian is translated**, all seven chapters, about eleven hundred lines; the other
  five languages are not yet, and now say so instead of quietly opening an English page under
  an Italian title. The site does the same from the same files: a help index per language, the
  chapters linked within that language, and the note on the ones that are still English — its
  navigation used to send an Italian reader to the English index.

- **Sorting by description (`Ctrl+F10`) and by owner (`Ctrl+F11`)**, FAR's own last two sort
  keys and the last two we were missing. Neither field is in the listing, so the key asks a
  worker for it exactly as the view that shows it does: the panel sorts without it in the
  meantime and sorts again when the answer lands, keeping the cursor on the file it was on
  rather than on the row. A file nothing has said anything about sorts after every file that
  has. Both are also sort-list rows and both work as tie-breakers in `panel.sort_chain`.

## [0.10.0] — 2026-10-28

### Added

- **The comparison goes into the subdirectories.** The dialog's last two rows switch it on and
  say how deep (1 is the two directories themselves, 0 all the way down). Both trees are
  walked in a worker and paired by the path each file has *under its own root*, so `a/b/c.txt`
  is compared with `a/b/c.txt`; every pair is then settled by whichever of the §11 rules are
  on. The answer is a panel of what differs — `≠` on both sides and not the same, `→` only on
  the left, `←` only on the right, `≈` the same but for the line endings — whose rows are the
  real files, so `F3`, `F5` and `F8` work on them. A difference three directories down cannot
  be shown as a marked row in a panel that is not showing it, which is why it is a panel and
  not a selection.

- The search engine's walker takes a **depth limit**, which is what makes that possible.

- **The drive dialog's columns are filled on Windows too.** The type, the label and the
  filesystem were empty there — which made three of the dialog's own switches switch nothing —
  because the standard library makes none of those calls. It asks for them now
  (`GetDriveTypeW`, `GetVolumeInformationW`), and for the share behind a mapped letter
  (`WNetGetConnectionW`), which is what the device column shows there. A letter is listed when
  the system will describe it at all, so an empty CD drive keeps its row; a drive that is not
  ready is not asked how big it is, because that is what puts *There is no disk in the drive*
  on the screen.

- **`$+` in the prompt** counts the directories `pushd` has put away, as `cmd.exe` writes it.

- **`F9` in the change-drive dialog** opens its switches as a dialog of boxes, as it does in
  FAR, with each row naming the `Ctrl` key it is — the keys are faster for anybody who knows
  them, and this is how one finds out they are there. `OK` keeps them and brings the list back
  for the same panel.


- **Find the duplicate files under a directory** (Commands → *Find duplicates*), the third
  thing FAR's compare plugin does. The tree is walked, the files grouped by size, and only
  what shares a size is hashed — so almost everything is settled without opening a file. What
  comes back is a panel of the copies, one row per file, numbered by group and ordered so the
  group that wastes the most space is first; the rows are the real files, so `F3` reads one and
  `F8` deletes what you pick. Nothing is deleted or even selected by the hunt itself, and empty
  files are left out.

- **The comparison has a dialog, and the options the plugin everybody uses has.** `tool.compare`
  (Commands → *Compare with options*) asks what *the same file* means before comparing: whether
  the names are matched with their case, whether the size and the time count, how many seconds
  apart two times may be and still be the same moment (two, for anything that has been to a
  memory stick), whether the bytes are read at all, read only where the times differ, or read
  only for the first so many KiB, whether a difference in line endings alone counts, whether
  only the selected rows are compared, whether to stop at the first difference, and whether to
  select only the newer copy or both sides of every difference. The answers go into `[compare]`,
  so the next comparison is the one that was set up. The sentence at the end now also says how
  many pairs were taken as the same without being read, and whether the line-ending-only
  differences were counted.


- **A panel of what is installed**, under the sockets in `Alt+F1`/`Alt+F2`, with the size
  column holding what each program takes on disk — so `Ctrl+F6` sorts by size, which is why
  one opens such a list. Windows reads the three registry keys the Control Panel's own list
  reads and hides what it hides; Linux reads dpkg's `status` file or pacman's descriptions.
  `F3` is the program's card, including the command that would remove it, and `F4` puts that
  command on the command line **unrun**. Nothing here uninstalls anything by itself.

- **The prompt says which branch you are on**, and is a format rather than a fixed string.
  `[cmdline] prompt` takes FAR's own letters — `$p` the path, `$w` its last part, `$n` the
  drive or the server, `$d` the date, `$t` the time, `$@` for an elevated session, the
  bracket-and-sign letters, `$h` to take a decoration back out — plus `$r`, which writes
  ` (branch)` inside a git repository and nothing outside one, and `%GITBRANCH%` for the bare
  name, spelled the way the plugin everybody installs spells it. The default is `$p$r$g `.
  Nothing is run to find the branch: the walk up to the first `.git` reads `HEAD`, follows the
  `gitdir:` of a worktree, and shows a detached head as the short commit — once per directory,
  not once per keystroke.

- **A socket list, as a panel of its own**, under the processes in `Alt+F1`/`Alt+F2`: the
  protocol, this end, the other end, the state and the process holding it, in columns. `F3` on
  a row opens that process's card — the question a socket list is opened to ask. Linux reads
  the four tables in `/proc/net` and maps each row back to the process holding it through its
  open files; Windows asks `GetExtendedTcpTable` and `GetExtendedUdpTable`, which give the
  owning pid with the row. Nothing here closes a socket.

- **`F3` on the process list shows the process.** It used to try to read `/proc/<pid>` as a
  file, which does nothing on Linux and nothing at all on Windows. Now it opens a card, asked
  for when the key is pressed rather than read from the listing: name, pid, what started it,
  whose it is, what it is doing, threads, resident and mapped memory, 32 or 64 bits, when it
  started, the executable, the working directory and the command line — every row the platform
  will answer and none that it will not. Windows gets the name, the parent, the threads, both
  memory figures, the bitness, the path and the start time, which is the whole of what it will
  give without reading another process's own memory.

- **The change-drive dialog is FAR's.** It opens over the panel it is about — five columns in
  from that panel's frame, which is FAR's own arithmetic — instead of in the middle of the
  screen, so a list that will send the right panel somewhere no longer looks like one about
  the left. The table is laid out the way FAR lays it out, with a rule between each pair of
  columns and a rule across the frame above the rows that are not drives. The cursor opens on
  the row for the panel one is actually looking at: a panel showing the process list is not on
  a drive, and was being offered the drive it was on before.

  Its columns are switched from inside it with the keys FAR uses — `Ctrl+1` the type, `Ctrl+2`
  the device, `Ctrl+3` the label, `Ctrl+4` the filesystem, `Ctrl+5` the sizes, `Ctrl+6`,
  `Ctrl+8` and `Ctrl+9` whether removable, optical and network drives are listed at all,
  `Ctrl+7` the panels that are not drives, and `Ctrl+R` to read the drives again. What one
  chooses goes into `[drives]` in the configuration, so it is still chosen tomorrow.

- **The session editor has pages**, as the NetBox plugin's own does: Session, Environment,
  Directories and the protocol's own page along the top of the frame. `Ctrl+PgDn` and
  `Ctrl+PgUp` turn them and wrap round, stepping over a page that the chosen protocol has
  nothing to say about — an SSH key file on an FTP session — which is drawn grey. The cursor
  comes with the page, and `OK` still reads every row whichever page is open. A third button,
  `Connect`, stores the session and goes there at once instead of returning to the list.

## [0.8.0] — 2026-10-28

### Added

- **Text can be marked out on the command line.** `Shift` with an arrow extends a selection
  from where it started, `Ctrl+Shift` with one takes a word at a time, `Shift+Home` and
  `Shift+End` reach either end. What is marked is drawn with its colours swapped, `Ctrl+Ins`
  copies that rather than the whole line — having marked it is having said what to copy —
  and typing, pasting or a backspace replaces it. Any arrow without `Shift` drops the mark.

- **Three panels that are not drives**, under the drives in `Alt+F1`/`Alt+F2` and numbered as
  FAR numbers its own:

  **NetBox — the stored sessions.** Every `[[site]]` and `[[s3]]` block, plus whatever is open
  now, as a panel of rows named `sftp://user@host`. A panel rather than a dialog, so `Enter`
  connects and the panel becomes that server's directory, and the quick search and the keys
  work on it the way they work on files.

  **Process list.** What is running: the name with its pid beside it, and the memory it has
  resident in the size column — so `Ctrl+F6` sorts by memory, which is what anybody opening it
  came to see. Linux reads `/proc/<pid>/status`; Windows takes a process snapshot and asks each
  one for its working set, and a process that refuses keeps its row with the column empty.
  macOS waits for a tested way to read its own list. Nothing here kills anything.

  **Temporary panel.** A panel of paths rather than a directory: what is in it can come from
  several directories at once, and `F5` out of it copies the real files. It opens empty, and
  `Ctrl+Shift+Ins` puts what is selected in the other panel into it — adding to what is there,
  never the same path twice, and leaving every file exactly where it was.

- **The last two view modes FAR has**, so all ten are now there on FAR's own numbers.
  `Ctrl+9` is the **links view**: how many names a file has, between the name and the size.
  One is an ordinary file, two or more is a file in two directories at once, and a column of
  them is how a hundred copies that are really one file stop looking like a hundred files'
  worth of space. The count costs a lookup per row — the `nlink` of a `stat` on Unix, a file
  that has to be opened on Windows — so it is fetched by a worker while that view is the one
  on screen, exactly as the owners are, and a file nobody may open leaves the column blank
  rather than claiming one name.

  `Ctrl+0` is the **alternative full view**: the full view with the sizes written out, digits
  grouped — `1 234 567` where the full view says `1.18 M` — for when the difference between
  1.2M and 1.9M is the thing one came to see. The totals in the bottom frame stay
  abbreviated, being a summary rather than a figure to compare.

### Changed

- **The wide view fits its columns to the names it has.** `Ctrl+4` is name and size, and it
  always drew two columns: a directory of download names was then two columns of cut-off
  names, which is the one thing that view is supposed to prevent. It now fits as many columns
  as the longest name allows — two for short names, one for long ones, with the whole name
  readable — which is FAR's own rule. The longest name is measured once per listing, in the
  pass that filters it.

- **`F4` in the viewer turns a wheel** instead of toggling hex: text → dump → hex → the
  laid-out document → the table → text again, skipping the turns a given file has no use for.
  A file is often none of the things its name says it is, and trying the next way of reading
  it should be one key rather than a choice from a list. The key bar says where the next turn
  lands rather than naming one destination for ever, which is also the answer to what `F4`
  should be called there. `Shift+F4` is hex and straight back, for when hex is the one that
  was wanted, and disassembly stays on `F5`.

- **The font reset moved from `Ctrl+0` to `Ctrl+Alt+0`.** A number key in the panels is a
  view mode, all ten of them, and `Ctrl+Shift+0` was already the first folder shortcut: the
  font is the newcomer and gives way. `Ctrl++` and `Ctrl+-` are unchanged.

## [0.7.3] — 2026-10-28

### Added

- **File descriptions** (`descript.ion`), the one FAR habit this program had none of. A
  directory can keep a line about each of its files in a plain file beside them, one name and
  the rest of the line, a name with a space in it quoted — 4DOS's format, which FAR kept and
  which travels with the directory rather than living in a database somewhere.

  `Ctrl+Z` writes one: the dialog opens on what the directory already says about the file
  under the cursor, so changing a description is a key and a few characters rather than
  retyping it, and an empty answer takes the line away. `Ctrl+6` shows names with their
  descriptions and `Ctrl+7` adds the size and the time — FAR's own numbers, which were empty
  here until now. The information panel (`Ctrl+L`) spells out the description of the row under
  the cursor, and the panel's status line shows it beside the size and the date.

  The file is read in whatever code page it is in — one written under DOS is in a DOS code
  page — and written back in UTF-8; both of FAR's names are looked for, `descript.ion` first
  and then `files.bbs`. It is read only while something on screen shows a description, because
  a side file nobody is looking at is a round trip per directory on a network share;
  `panel.descriptions = false` turns the habit off altogether.

  **And the descriptions follow what they describe**: a copy leaves the line where it was and
  writes one at the other end, a move takes it with it, a rename takes it to the new name, and
  a delete takes it away — a description that outlived its file would land on the next file to
  be given that name. Only if the job really did something, so a cancelled copy leaves no
  description pointing at a file that never arrived.

## [0.7.2] — 2026-10-28

### Added

- **A sort criteria chain.** `panel.sort_chain = ["ext", "name"]` says what settles two rows
  the sort key leaves equal — up to three keys, asked in order, with the *name* having the
  last word after them whatever was asked, so the comparison is total and the order does not
  wander between two reads of the same directory. A key the chain repeats is skipped, and
  `reverse` turns the whole comparison round rather than only its first step. A word that is
  not a sort key is said on the startup screen rather than quietly sorting by name.

- **Sort groups** (`Shift+F11`, FAR's `Group`). Bands written in the configuration as
  `[[sort_group]]` blocks — a name, a mask in the filter's own grammar, and `place = "front"`
  (the default) or `"back"` — that pull some kinds of file in front of the listing and push
  others behind it whatever the panel is sorted by: sources first and build output last, in a
  directory that holds both. It is the thing no sort key can say, because it is about kinds of
  file rather than one field of them. The front bands keep the order they are written in, so do
  the ones behind, and what matches nothing sits between them; inside a band the ordinary sort
  still applies. Folders are not banded — they have their place at the top already — and
  `reverse` turns each band round rather than swapping the bands over. Writing the blocks turns
  them on, `Shift+F11` toggles them per panel and says how many there are, and a block that
  does not parse is named on the startup screen and skipped. `Shift+F11` was folders-first,
  which is a setting in the panel dialog here as it is in FAR; `Group` over that key is what
  FAR writes and what it has always meant.

- **A memo field says what is in it.** A dBase table keeps its long text in a second file
  beside it — a `.dbt` or an `.fpt` — and a memo field holds only the block number, which is
  what the cell showed: `memo:42`. That file is now opened and read, and the cell says what
  the block holds, flattened onto the one line a cell has. All three formats, each recognised
  by what it actually holds rather than by its name, because the name does not say which:
  FoxPro's `.fpt` (the block size big-endian at offset 6, a block opening with its type and
  its length), dBase IV's `.dbt` (the block size at 20, a block opening `FF FF 08 00` and its
  length), and dBase III's `.dbt` (blocks of 512, the text running to the first `1A`). The
  application looks for `.dbt` then `.fpt` beside the table, in either spelling — a table
  shouting in capitals usually has a memo file shouting back. Where there is no memo file to
  open, the cell says the block number, as before.

- **The name, drawn, above what the program says on the command line.** `fcmd --help`,
  `--version`, `--paths`, `--list-themes` and the rest now open with the name in letters.
  Into a terminal only: `--dump-config > config.toml` and `--list-commands | grep` are how
  these outputs are used, and a banner in front of them is something to strip. `--man` never
  gets one — its bytes are a file a packager installs.

### Fixed

- **An arrow ends a quick search.** `Alt+`letters finds the row, and then `↓` did nothing at
  all: the search swallowed the key and there was no way out but `Esc`, which put the cursor
  back where it started — undoing the search one had just made. An arrow now closes the
  search and leaves the cursor on the row it found, so the first press ends the search and
  the next one moves. `Esc` still goes back to where you were, which is what it is for.

## [0.7.1] — 2026-10-28

### Added

- **A command's output arrives while it runs.** It was read to the end of both pipes and
  handed over in one piece, so a build that takes a minute was a minute of nothing followed
  by everything: a command one cannot watch. The worker now reads both pipes into one channel
  — interleaved the way they were printed, rather than all of one and then all of the other —
  and reports whole lines as they arrive; the application writes the `$ command` heading when
  the command *starts* and the lines land under it as they come. And the panels go off while
  it runs, the way the console comes forward in FAR, and come back when it ends. `Ctrl+O`
  gives the screen back early and the command carries on behind the panels; *Options →
  Interface → Show a command's output while it runs* (`ui.command_screen`) turns that off and
  leaves the panels where they are, with the output still filling the screen `Ctrl+O` opens
  as it is printed. A line rewritten in place with a carriage return — a progress bar — is
  kept as what it ended up saying.

### Fixed

- **The table mode's keys.** `←` and `→` did nothing in a table — the sideways scroll was
  written for unwrapped text and the mode was not in its match — `Ctrl+←`/`Ctrl+→` shifted the
  hex dump by a byte, which in a table means nothing, and `Home` and `End` moved the byte
  offset while the table went on showing the row it was showing, so both looked broken. Now:
  the arrows move the window sideways and stop where the last column ends rather than
  scrolling into empty space; `Ctrl` and an arrow jump a whole column, which is the unit a
  table has and a wall of text has not; `Home` and `End` go to the first and last screenful
  of rows.

- **`Shift+F3` makes a table of any file that reads as one.** It took the name's word for it,
  so tab-separated columns in a `.txt` — as common as in a `.tsv` — had no way into the mode,
  and neither had a log laid out in columns. The key now refuses only a file with no table in
  it: one whose first bytes are not text, or one that does not come apart into two columns or
  more. What opens as a table *by itself* is unchanged and still goes by the name, because
  what a file opens as has to be predictable; the key is the less fussy of the two, and since
  the same key goes back to the source a wrong guess costs one keystroke.

- **A DBF's numbers are read down their last digit.** Every cell was laid out to the left, so
  a column of prices was a staircase. The numeric field types (`N`, `F`, `I`, `Y`, `B`, `O`)
  now sit on the right of their column and everything else stays where it was.

- **A deleted DBF record has its own column.** The `*` that marks one was stuck on the front
  of the first cell, and in a column of numbers `*1` reads as a number nobody can tell from
  `1`. It is now a column one character wide in front of the fields. The status line's
  `rows × columns` still counts the fields, not that column.

- **A DBF written under DOS is read as DOS.** The header's language byte was mapped to
  whatever Windows code page was nearest, and code page 437 — the one a table written under
  DOS actually carries — was mapped to 866, which is Cyrillic: a German table came out with
  `temporäre` as `tempor„re`. The 437 family now goes through the viewer's own 437 table, the
  one it has had since it learned to show a DOS text file, and the Shift-JIS, GBK, Big5,
  EUC-KR and Thai drivers are mapped as well. 850, 852 and the other DOS tables `encoding_rs`
  does not carry still fall back to the Windows table that agrees with them on the accented
  letters, which is what a field holds.

- **The command line walks its history with the arrows.** `↑` and `↓` with something on the
  line now step back and forth through what was run before, which is what every shell does
  and what anybody typing there reaches for; FAR's own `Ctrl+E` and `Ctrl+X` still work, and
  on an empty line the arrows belong to the panel, as every shared key does.

### Changed

- **Three keys the editor answered and never admitted to**, found by reading its bar against
  its keymap: `F3` marks a block, `F6` hands the file to the viewer, `F9` assembles what is in
  it. The bar says so now. It also promised `8 Replac` while replace was on `Ctrl+F7` alone,
  so `F8` does it too — a bar that promises a key has to be telling the truth. And the panels'
  bar now says `11 Plugin`, which `F11` has done since there were plugins.

- **The key bar always shows all twelve keys.** The editor's bar had seven boxes, because the
  editor uses seven of the function keys; the viewer's had eleven; the panels' had eleven. So
  the same key sat in a different place on each screen, and `F4` in the editor was the third
  box along. All twelve are drawn now, in twelve equal boxes, with the ones a screen has no
  use for left empty — the bar is read sideways, and `F5` is the fifth box along wherever one
  is. Clicking an empty box does nothing, as before. The same rule covers the modifier rows:
  holding `Shift` empties the boxes `Shift` has nothing on rather than closing up the gaps.

- **The hex dump widens in blocks of eight.** `→` and `←` in a dump set how many bytes a row
  holds, and the steps were the powers of two — 4, 8, 16, 32, 64 — so on a wide screen there
  were three widths where there is room for eight, and a press doubled the row instead of
  adding to it. They are now 8, 16, 24, 32, 40, 48, 56, 64: eight bytes is the group the dump
  already draws a gap between, so every press adds one whole group and the columns stay where
  the eye expects them. A window too narrow for even eight bytes still draws four, which is
  the one width that is not a block of eight and the reason it is there.

## [0.7.0] — 2026-10-28

### Added

- **A dBase table opens as a table** — `.dbf`, in the mode the last release added, and this is
  the reader the format was waiting for. A DBF says in its first bytes how many records it
  holds, how long one is, where they start and what the columns are called with the width of
  each, so the table is laid out without a single record being read and row *n* is one
  multiplication and one read. Nothing but the window on screen is ever touched: a
  two-gigabyte table opens as fast as a log does, and the sixteen-megabyte limit the text
  formats have does not apply to it at all.

  The field types are read as what they are: text with its padding off, numbers as written,
  a date as `YYYY-MM-DD` (and empty where it was never set), a logical as `true`/`false`, the
  binary integer, currency, double and timestamp types, and a memo field as the block number
  it holds — reading the `.dbt`/`.fpt` beside the table is a second file and still owed. A
  **deleted** record is shown with a `*` in front of its first cell rather than being skipped:
  a table whose deleted rows vanish is one whose row count does not add up, and finding them
  is half of why anybody opens a DBF. The header's language byte picks the text encoding, the
  status line names the flavour (dBase III, Visual FoxPro…), and a header that claims more
  records than the file can hold is read as far as it goes and says `cut`.

- **The colour groups have a dialog.** `F9 → Options → Files highlighting` — the entry has
  said *not here yet* since the menu bar was written, and the `[[highlight]]` groups that
  shipped in 0.6.0 had no interface but the configuration file. The list shows them in the
  order they are tried, which is the order that decides, and **each row is drawn in its own
  colours**: a list of colours one cannot see would send you back to the file to find out
  what a group does.

  `Enter` or `F4` edits the group under the cursor, `Ins` makes a new one beside it — where a
  new group lands matters, since the first match wins — `Del` takes one away, and `Ctrl+↑` /
  `Ctrl+↓` move one past its neighbour. The dialog itself takes exactly the words the file
  takes, because it is the same condition set and the same parser: a mask list, files or
  folders, a size window, an age in days on any of the four timestamps, the attribute
  letters, and the two inks. A condition that does not parse comes back on the dialog with
  every field still filled in and the problem named; a group with no condition at all is
  refused rather than painting every row.

  A change is in force on the panels the moment it is accepted, and `Options → Save setup`
  writes it to `fcmd.toml` — which the status line says each time. The dialog deliberately
  does not write the file by itself: that file is one people edit by hand, with comments in
  it, and saving regenerates it.

- **`Alt+F11`: the files you have read or edited.** FAR's own key, and the last of the three
  histories the Commands menu promised — the entry used to say *not here yet*. Newest first,
  `Enter` reads a row, `F4` edits it, and it survives the session in the state file like the
  command and folder histories (`ui.file_history` says how many to keep).

  It is written in the one place every viewer and every editor passes through, which is also
  what makes it right for a file on a server: the viewer is given the remote path, so that is
  what is remembered, while the copy fetched into the program's own temporary directory is
  left out — a history of paths that will not exist tomorrow is worse than no history. A file
  opened again moves to the top rather than appearing twice, because the list is *where I have
  been* and a file read five times is one place. A row whose file has gone says so rather than
  opening nothing.

## [0.6.0] — 2026-10-28

### Added

- **A `.csv` opens as a table.** `Shift+F3` in the viewer, and a file whose name says it is
  tabular opens that way: a heading and a rule that stay where they are while the records
  scroll under them, columns lined up, one row to a record. `.csv`, `.tsv`, `.tab`, `.jsonl`
  and `.ndjson` for now; the same key goes back to the commas, `Shift+F4` has the table in the
  cycle with the text, the dump and the hex, and `←`/`→` move the window across a table wider
  than the screen.

  The delimiter is *guessed* — a comma, a semicolon, a tab or a bar, whichever gives the most
  lines with the same number of fields, which is what a table is — and the status line admits
  the guess along with the shape of the table, so a wrong one is obvious at a glance rather
  than silently shaping the screen. A quoted field may hold the delimiter and a newline, as
  RFC 4180 says, so a record is not a line. A row with more fields than the header has them,
  under a column named after its number, because an export with a short header is common and
  dropping data would be worse.

  For JSON Lines the columns are the keys of the first two hundred records, in the order they
  are written in the file; a record carrying a key that turned up later shows it in a last
  column called `…` rather than losing it, and a line that is not an object — a stray `[` from
  somebody's JSON array — is counted in the status line rather than drawn as a row of
  nonsense. A nested value is its own compact JSON, and a null is an empty cell.

  Like a laid-out Markdown document a table is read whole, because a column's width cannot be
  known from the part of the file on screen; over sixteen megabytes the file opens as text and
  says so. `ui.table = false`, or the box in the interface settings, turns the first screen
  back into the source for good. What this mode is not, and deliberately: a spreadsheet and a
  query tool — no editing, no sorting, no filtering, each of which wants an answer about
  writing the file back that a viewer does not have.

- **Every menu row has its letter.** The mechanism shipped a while ago — `&` in a label marks
  the letter and the dropdown draws it highlighted — but only four labels carried a marker,
  so the rest fell back to "the first row whose label starts with it" and several rows could
  not be reached by a letter at all. Every row of every menu now claims a letter of its own,
  in all six languages written in an alphabet, by a rule rather than one at a time: the first
  letter of the first word that no other row of that menu has taken, or any free letter of
  the label where every word's initial is gone. A test asserts that no two rows of one menu
  claim the same letter, in every language, so a new entry cannot quietly shadow an old one.

  Two things fell out of the sweep. The *Files owners* view had no menu row at all, and the
  Commands menu listed *Folder shortcuts* twice — the same words, the same `Ctrl+D`, the same
  command — so it now appears once, after the associations, where FAR puts it. And a label
  that doubles as a dialog title is drawn without the marker: a frame is not a menu row.

- **`Ctrl+8`: who the files belong to.** FAR's file-owners view, on FAR's own number: name,
  size, owner, group. The two names take the room the date column had, because somebody
  asking who a file belongs to is not asking when it was written, and on a panel too narrow
  for both the group goes and the owner stays.

  Who a file belongs to is deliberately not part of a listing: it costs a lookup per file —
  `getpwuid` through NSS on a Unix, a security descriptor and a SID lookup on Windows, either
  of which can reach across a network — so nothing is paid for it unless this is the view on
  screen. Choosing the mode asks a worker, and the two columns fill in a moment later with
  the listing already up. One cache serves a whole directory, since a listing is nearly
  always one or two accounts repeated. A row nothing has come back for is blank rather than a
  guess, a number with no account behind it is shown as the number, and a panel on a server
  or inside an archive asks nothing at all.

- **A fix the dialogs needed**: the mouse box in `F9 → Options → Interface settings` reported
  *unknown setting `ui.mouse`* when the dialog was accepted, because that one key had never
  been added to the configuration's own setter. A test now walks every row of every settings
  dialog and asserts the configuration knows the key, so a row that looks like it works
  cannot fail only when somebody presses OK.

- **The system clipboard, through the terminal.** `Ctrl+Ins` in the panel copies the names of
  what is selected, one per line; `Alt+Shift+Ins` copies them with their whole paths;
  `Ctrl+Ins` copies the command line instead while there is something on it; and a block
  copied or cut in the editor now goes on the system clipboard as well as on the editor's own.

  It is done with `OSC 52` — the terminal is asked to carry it — rather than with a clipboard
  library, and that is the whole point: no X11 or Wayland dependency, no `pbcopy`, no
  `unsafe`, no platform code, and it reaches the clipboard of the machine you are sitting at
  even when the program is running on another one through `ssh`, which a clipboard library on
  the far end cannot do. `ui.clipboard = false`, or the box in the interface settings, turns
  it off for anyone who would rather their terminal were not asked.

  Reading the clipboard stays the terminal's job: its own paste key hands us the text, and
  that now goes where the text was meant to go.

- **Pasted text lands where it belongs.** A paste used to reach only a prompt; it now reaches
  the command line, a dialog row, the editor and a terminal. A paste is not typing, and is
  not treated as typing: the editor and a real terminal take all of it, newlines and all,
  while everything that is one line takes the first line and says how many more there were.
  A command copied from a web page arrives with its newline still attached, and running it
  before it can be read is the accident this avoids. A carriage return from a Windows
  clipboard is no longer a character to insert.

- **A size condition in the find dialog** (`Alt+F7`). The row takes the same words as every
  other size condition in the program: `100M` finds everything from a hundred megabytes up,
  `1M-1G` a window, `-4K` everything smaller. Leave it empty and size is not asked about.
  It answers the question a full disk raises — *where did the space go* — without a separate
  tool, and the results frame repeats the condition next to the mask so a list of hits read
  later still says what was asked for. A size that is not a size comes back on the dialog,
  with the field still holding what was typed.

- **Colouring files by what they are.** A theme says what an archive or a hidden file looks
  like; a `[[highlight]]` group in `fcmd.toml` now says it for whatever you like — a mask, a
  size window, an age in days on any of the four timestamps, attributes, files or folders —
  written in the same words the filter dialog takes, because it is the same condition set.
  The first group that matches a row decides, so the order in the file is the order that
  counts, and FAR's built-in six come after them, which leaves a file nothing is said about
  looking exactly as it did.

  The ink is a colour (`#d7875f`, `red`, `idx:208`) or the name of a theme colour such as
  `file.archive` — and a theme colour is looked up every time the row is drawn rather than
  once at startup, so a group follows `Ctrl+Shift+T` from one theme to the next. A group with
  no condition is refused, because it would paint every row, and a group that does not parse
  is a warning on the startup screen with its name in it: a colour is not worth refusing to
  start over.

## [0.5.0] — 2026-10-26

The first release under the name it keeps. The program is FinalCommander and the command is
`fcmd`; a configuration written for 0.4.0 is read exactly as it was, under its old file names,
with one line on the startup screen saying which file that was.

The rest of the release is a panel that notices what it did not do itself, a panel that shows
what FAR's shows — the extension in its own column, the dates and the allocated size of the
detailed view, the device and the memory in the information panel — and the first crate in
this workspace allowed to make the calls the standard library does not, which is what finally
puts free space on Windows.

### Added

- **Free space on Windows, and the rest of what a device can say.** The panel frame has
  shown the free space on Unix since it had a frame and nothing at all on Windows, because
  `statvfs` is not a call Windows has. There is now one crate — `fcmd-platform` — for the
  handful of answers the standard library does not have, and it makes each platform's own
  call: `statvfs`, `GetDiskFreeSpaceEx` and `GetDiskFreeSpace`, `/proc/meminfo`, `sysctl`,
  `GlobalMemoryStatusEx`. It is the only place in the workspace allowed `unsafe`, and every
  call in it carries a line saying why it is sound.

  What it buys, beyond the frame: the drives dialog (`Alt+F1`/`Alt+F2`) fills its two size
  columns on Windows as it already did on Unix — the letters were there, the sizes were not.
  The information panel (`Ctrl+L`) now says what the device
  holds, what is left on it and how big its allocation unit is, how much memory the machine
  has and how much of it is free, and what the cluster size costs the directory on screen —
  the figure FAR calls slack. And the detailed view has its **allocated** column: what each
  file takes on the device rather than what it holds, rounded to whole clusters, which is
  the column FAR puts second. A panel whose device said nothing — a server, an archive —
  has no such column rather than a blank one.

- **A quick view of a folder says something.** `Ctrl+Q` on a directory used to print one
  line — that it is a directory — and leave the rest of the frame empty. It now fills with
  what FAR puts there: the folder and when it was last written, then what the device under it
  holds, what is left on it, how big its clusters are, how much memory the machine has, what
  the directory holds and what the cluster size costs it. The same block the information
  panel draws, because it is the same question asked from the other side.

- **The detailed view has FAR's columns.** `Ctrl+5` showed the name, the size, one date and
  the attributes; it now shows what the file was written, when it was made and when it was
  last read, in FAR's order, with the attributes after them. The two extra dates are dropped,
  the last one first, as soon as keeping them would leave the name unreadable, so a half-width
  panel shows what the full view shows with the attributes after it; a filesystem that does
  not keep one of the dates leaves that column blank rather than inventing an epoch. The
  allocated size is the column above.

- **The keys the keyboard reference had promised.** A sweep through `docs/03-keymap.md`
  comparing every row against the keymap the program actually resolves turned up keys that
  were documented and dead. Three of them were worth having and are now bound: `Ctrl+P` hides
  the panel that does not have the focus (FAR's own, and the focus stays where it is),
  `Shift+F10` saves and leaves the editor in one key, and `Ctrl+W` takes the word before the
  caret off the command line, as every readline does. The editor also gained delete-word-right
  (`Ctrl+Del`, `Ctrl+T`), which it had only in one direction.


- **The full view separates the extension from the name**, as FAR does: the names run down the
  left of the name field and the extensions down its right, against the size column, so a
  directory of one kind of file reads as a list rather than as a paragraph. A folder with a dot
  in its name is not a file with an extension (`docs.old` stays whole), a dotfile's leading dot
  starts its name rather than an extension, and an extension too long for the column is left
  where it was written. The other views, the status line and everything that names a file still
  show the name whole.

- **A panel notices what it did not do itself.** A local panel watches the directory it is
  looking at — `inotify` on Linux, FSEvents on macOS, `ReadDirectoryChangesW` on Windows — and
  re-reads it when something lands there: a build that wrote a file, a download that finished,
  another program that moved one out. Until now the directory stayed as it was read and
  `Ctrl+R` was the only way to find out. The watch is debounced, so a `git checkout` that
  touches a thousand files is one re-read and not a thousand, and the re-read keeps the cursor
  on the name it was on and the selection on the names that are still there. One watch per
  panel and local paths only: a server reports nothing locally, and an archive is a file.
  `panel.auto_refresh` turns it off, and `Options → Panel settings` has the box.

### Changed

- **The program is called FinalCommander, and its command is `fcmd`.** It shipped as
  SupremeCommander up to 0.4.0; the name is now the one it will keep. Everything that carried
  the old name carries the new one: the command, the crates (`fcmd-core`, `fcmd-vfs`, …), the
  configuration (`fcmd.toml`, `fcmd-state.toml`, `fcmd-macros.toml`, `fcmd-menu.toml`,
  `fcmd-plugins/`), the manual page, the plugin environment (`$FCMD_CWD` and the rest) and the
  release archives (`FinalCommander-linux-x86_64-<version>.tar.gz`, with `fcmd` inside).
  Nothing about what the program does
  has changed with it.

  A configuration from 0.4.0 keeps working untouched: every file is looked for under its new
  name first and its old one after, so `scmd.toml` is still read — and still written — with one
  line on the startup screen saying which file it was and what to call it instead. The same
  goes for the state, the macros, the user menu and the plugins folder.


- **`Ctrl+O` takes the panels off rather than opening a screen of its own.** It used to show
  the output of past commands as a read-only view you could only scroll and leave; FAR's
  `Ctrl+O` hides the panels and hands the screen back to the console with everything else still
  live. It does that now: the command line, the menus and every key keep working with the
  panels off, `PgUp` and `PgDn` turn pages through the output, and the same key brings the
  panels back. The Commands menu row that says `Panels on/off` finally does.


- **The confirmations have buttons, and the buttons name the verb.** `{ Move }` and
  `[ Cancel ]`, as FAR has it, rather than a line of text explaining that Enter means yes: a
  reader who glances at the dialog and its button should know what is about to happen without
  reading the sentence again. The label comes from the action itself — the delete, the
  wastebasket, the wipe, the overwrite, the discard, the quit — so a button cannot say one
  thing and do another. The arrows and `Tab` move between the two, `Enter` takes the one that
  is lit, `Esc` is still the way out, and the dialog opens on the action because that is what
  the person who opened it meant to do.


- **Three bindings moved off `Alt+<letter>`**, which belongs to the quick search. A binding
  always wins over the search, so each of them cost a letter one could never search a listing
  for: the site list (`Alt+N` → `Ctrl+Shift+N`), the quick filter (`Alt+I` → `Ctrl+Shift+I`)
  and the theme (`Alt+T` → `Ctrl+Shift+T`). The editor's `Alt+I` and `Alt+U` stay where FAR
  has them — there is no quick search in an editor to collide with.

### Fixed

- **The panel's column titles were English in every language.** `Name`, `Size`, `Date
  Time` and `Attr` were literals in the drawing code, beside `Up` and `Folder`, which have
  been translated since the panels were written. They go through the catalogues now, with
  the three new headings of the detailed view, so a panel in Italian says `Nome` and
  `Dimens.` and one in Russian says `Имя` and `Размер`.

- **The keyboard reference says what the program does.** The rows that named a key nothing was
  bound to now say what is true: the command line's path insertion is `Ctrl+[` / `Ctrl+]` (one
  row, not two contradicting each other), `Esc` clears the line and `Ctrl+U` swaps the panels,
  multi-rename and copying names to the clipboard are named as not built yet, and compare
  directories, test archive and "other panel follows cursor" are named as what they are —
  menu entries with no default key, because `Alt+<letter>` belongs to the quick search. The
  profile table listed five profiles where two are built; the other three are now under a
  heading that says they are planned.


- **The attributes dialog was taking bits away on Windows.** `Hidden`, `System` and `Archive`
  were seeded with `false` rather than read off the file, and `OK` writes every box — so
  opening `Ctrl+A` on a hidden file and pressing `OK` cleared all three on a file nobody meant
  to change. The archive bit is set on nearly everything Windows writes, so it happened every
  time the dialog was used. The boxes now open on what the file carries, which also means the
  listing carries it: `SYSTEM` and a new `ARCHIVE` flag are read where `HIDDEN` and `READONLY`
  already were.

- **A junction sorted among the files.** A link that points at a directory is a link, and
  `dirs_first` asked the kind, so `Documents and Settings` sat below every folder — where
  nobody reading a `C:\` listing looks for it. What decides now is whether the row *opens* as
  a directory, which is also what the size column answers (`Folder`, not the link's own
  meaningless byte count). What the entry **is** has not changed: the `@` stays on the name,
  and deleting it still deletes the link rather than the tree.

- **The first `Enter` of a quick search did nothing one could see.** Every keystroke had
  already moved the cursor, so the `Enter` that closed the search only confirmed what was on
  the screen and a second one was needed to open the folder. It now closes the search *and*
  opens what it found, which is FAR's behaviour: one key, one result.

- **"Delete 1 items".** The confirmation counted where it should have named: deleting one file
  asked about `1 items`, in every language, which is the sentence nobody means to write — and
  then listed the name underneath anyway. One file is now named in the question (`Delete
  a.txt (4 K)?`) and not repeated below it; several are still a count with the first three
  names under it. The three confirmations that count — delete, wastebasket, wipe — carry a
  plural selector in all seven catalogues rather than one form that is wrong half the time.

- **`Ctrl+PgDn` did nothing.** It is FAR's way into a directory or an archive and the other
  half of `Ctrl+PgUp`, which worked; the keymap document has listed it since the feature
  inventory and no profile ever bound it, so the key was dead. It opens what the cursor is on,
  and the keymap test now asserts both halves of the pair rather than only the one somebody
  thought of.

- **The hex dump widened in steps it could not draw.** The row width was a free number moved
  eight at a time, but a dump is only ever drawn at a power of two: from 16 the first press
  set 24 and drew 16 — nothing happened — the second set 32 and drew it, and every press past
  the widest the window could hold kept counting up where nobody could see it, so coming back
  took as many presses as had been wasted. The steps are now the widths it is drawn at (4, 8,
  16, 32, 64), one press is one visible step, and the setting stops at what the window can
  hold rather than storing a width for a window nobody has. The arrows do both directions:
  widening with a key that cannot narrow again is a key pressed once too often.

- **The drive letter takes the drive.** `Alt+F1` then `C` goes to `C:\` instead of moving a
  cursor one then has to press `Enter` on — thirty years of fingers expect the letter to be
  the key. The connections and buckets keep the numbers FAR gives them.

- **`&Medium` in the menu.** The ampersand is the marker for the letter a row answers to —
  the theme has had a colour for it (`menu.hotkey.fg`) since the first menu — and nothing ever
  consumed it, so the rows whose labels carried one showed the marker instead of a key. It now
  names the key: the letter is drawn in the hotkey colour and typing it takes the row, with
  `&&` for a literal ampersand and the first marker winning. Rows with no marker still answer
  to the letter they start with, which is what FAR does and what a catalogue with no markers
  needs.

## [0.4.0] — 2026-10-25

The things a user reaches for that were not there. Three more ways to reach files — FTP, FTPS
and WebDAV in the netbox, 7z read *and* written in pure Rust — and the keys and dialogs that
make the program somebody's own rather than everybody's: the user menu on `F2`, file
associations behind `Enter`, the palette editor, the editor's and the viewer's own settings,
the mouse, and the viewer's chrome on FAR's keys. Underneath, every path handed to Windows now
goes past `MAX_PATH`. And the project has a site, generated from the repository's own documents.

### Added

- **The viewer's chrome, and the settings behind it.** Three rows a reader may take away one
  at a time, on FAR's own keys: the key bar (`Ctrl+B`), the top line with the name and the
  position (`Ctrl+Shift+B`), and a scrollbar down the right edge (`Ctrl+S`), which is the one
  that is off until asked for. Every row taken away is a row of content gained, which is the
  whole reason for the keys: one long line, or a wide hex dump, wants the window.

  The `Ctrl+B` question the backlog had left open is settled and it cost nothing: it was one of
  *three* spellings of page-up here — a vi habit — and `PgUp` and `Ctrl+Up` still page up, so
  FAR's key went to FAR's feature.

  The scrollbar says **where** in the file the reader is rather than how much of it is on
  screen: how many bytes a screenful holds is not a fixed fraction of a file — a wrapped line,
  a hex dump and a rendered Markdown page each answer differently — so the thumb is one row at
  the percentage the top line already shows.

  Behind them, `[viewer]` and `F9 → Options → Viewer settings`, the dialog the Options menu had
  been offering as a dim row: the three chrome rows, the wrap a file opens with (`off`, `char`
  or `word`) and the tab width, which is eight because that is what a terminal and `cat` do —
  a file that lined something up with tabs was written against that number. The division is
  FAR's: the setting is what a file *opens* with, and the key is what this reader wants of the
  file in front of them, so a key writes nothing back. Remembering a file's own position and
  code page is a dim row there, because it wants the store the histories want.

- **The user menu** (`F2`), which is the feature that makes a commander somebody's own: a few
  lines they run ten times a day, each with a label and a letter. It lives in `fcmd-menu.toml`
  beside the settings — `fcmd --paths` prints where, and `F2` with no file says so rather than
  opening an empty box — and a directory can carry its own `.fcmd-menu.toml`, whose rows come
  above the ones that are everywhere, because the menu of the project one is in is the one
  wanted while in it.

  A row holds **command lines**, not a dialect of its own: the same lines the command line
  takes, so `{file}`, `{selection}`, `{list}` and `{ask:title=…}` all work, internal commands
  and prefixes included. Several lines run in order and stop at the first failure, one at a
  time. The rule that makes that work is who owns the next step: a line that started a shell
  owns it and the rest waits for that shell to report; a line that asked a question owns it,
  because the answer is what starts the shell; a line that was internal (`cd /tmp`) is finished
  when it returns, so the next goes at once; and a line that put the viewer or the editor on
  the screen ends the row, because commands running behind what the user is now looking at is
  the opposite of what the screen says.

  `when` keeps a row out of the way where it has no use: four facts — `panel.has('Cargo.toml')`,
  `file.is('*.rs')`, `panel.selected`, `panel.local` — with `!`, `&&` and `||`. No parentheses,
  because a condition that needs them belongs in a script. A condition it cannot read is
  refused **by name** and the row stays on screen with the reason: an entry that vanished over
  a typo in its own `when` would be impossible to work out, which is the argument against the
  usual "evaluate to false and say nothing".

  Submenus nest, a row that opens one says so (`▸`), and `Esc` inside a submenu goes back to
  the menu it came from rather than out of the menu. Still to come, and in the backlog: reading
  FAR's `FarMenu.ini` as it stands, separators between groups, a dialog to edit the entries,
  and `Shift+F10` for the row taken last.

- **A site for the project**, published on GitHub Pages from the repository's own documents.
  `crates/fcmd-site` is a generator, not a site: it reads `README.md`, `CHANGELOG.md`, the
  commander survey in `docs/00-landscape.md`, the help chapters in `docs/help/` and the
  screenshots the renderer itself draws into `docs/shots/`, and writes 38 static pages — the
  landing page with the argument and the screenshots, what the one file replaces, what works
  today, the comparison with the other commanders, the download page for the current release on
  each platform, the navigable help and the changelog.

  Two decisions are worth writing down. The pages carry no framework, no web font and no
  script: one hand-written stylesheet, inline SVG screenshots, and a page that works out of a
  `file://` URL and in a text browser. And nothing is written twice — a page is its document
  rendered, so it cannot drift from it; a section renamed in the README fails the build instead
  of publishing a page with a hole in it. The landing page, the two README sections and the
  download page exist in the seven languages the interface speaks, and a page whose source
  document is English says so in a note rather than pretending.

  `cargo run -p fcmd-site -- --out site` writes it; `.github/workflows/pages.yml` builds it on
  every push, checks that the pages that matter are there and not empty, and keeps them as the
  run's artifact, which is downloadable and reads correctly off the disk. Publishing is a
  separate decision and nothing takes it automatically: a site on GitHub Pages is public to
  everyone, so it goes up only when a person starts that workflow from the Actions tab and
  answers `yes` to its one question. It also needs Pages switched on for the repository with
  "GitHub Actions" as the source, which no workflow can change, so a manual run that finds it
  off says the site was built and not published rather than failing.

- **Paths past `MAX_PATH` on Windows.** A Windows API call given a path of 260 characters or
  more fails, however much room the filesystem has — NTFS has allowed 32,767 since 1993, and it
  is the Win32 layer in front of it that does not. A file manager meets the difference every
  day: `node_modules` nested four deep, a build tree, a path from a server that was fine where
  it came from.

  So every path handed to the operating system now goes through `fcmd_core::os_path`, which on
  Windows adds the verbatim prefix — `\\?\`, or `\\?\UNC\` for a share — to an absolute path of
  240 characters or more, and on every other platform is the identity. 240 rather than 260
  because a directory is allowed twelve fewer characters than a file: what goes in it has to
  fit too.

  Two things it deliberately does not do. It does not prefix a path holding `.` or `..`,
  because a verbatim path is *not* normalised by Windows and the `..` would arrive at the
  filesystem as a name — such a path keeps the behaviour it had, rather than gaining a new way
  to fail. And it is not a manifest declaring the process long-path aware, because that also
  needs a registry setting on the machine and is not something a program can rely on by itself.

  The test is end-to-end rather than about the string: a directory twelve deep, a file copied
  into it through the job engine, read back, renamed and deleted. It passes on a Unix because
  `PATH_MAX` is 4096; on Windows it passes only because the prefix is there, and the CI runner
  is what proves it.

- **Editor settings** (`F9 → Options → Editor settings`), and the `[editor]` section behind
  them. The tab policy is three choices and not a box, as FAR has it: `keep` leaves every tab
  alone, `typed` turns a newly typed tab into spaces, and `all` also expands the tabs already in
  the file when it is opened — which changes the file, so it is not the default and the status
  line says how many lines it touched, because the next save will write that difference. Beside
  it: the tab width, the automatic indent, **spaces and tabs drawn** (`·`, and `→` followed by
  its padding, which is how a mixed indent becomes visible), and whether the cursor lands at the
  start of what a search found or just past it — the first is for reading, the second for typing
  over it.

  The cursor beyond the end of a line is in the dialog as a dim row and stays unimplemented:
  every movement, every insert and the block arithmetic would have to agree about a column that
  is not in the text, which is a change through the whole editor rather than a row in a dialog.

- **Four editor keys** that were each somebody's habit. `Ctrl+Q` arms the next key to go in as
  itself — a control character, or a real tab where tabs are expanded, which is the only way
  either gets into a file; a key with no character of its own says so rather than inserting
  nothing. `Ctrl+Shift+Ins` adds the block to what is on the clipboard instead of replacing it,
  so a few pieces are gathered and pasted in one go. `Ctrl+N` and `Ctrl+E` put the cursor on the
  first and the last line *on the screen*, leaving the screen where it is. And `Alt+I` now moves
  the line the cursor is on by one indent when nothing is selected, instead of typing a tab —
  `Tab` is the key for that, and `Alt+U` already worked this way.

- **A palette editor** (`F9 → Options → Colours…`, or `opt.colors`). Every colour the
  interface asks for, one per row and in the order they are declared, with the value as a theme
  file writes it and the colour shown twice — as ink on the panel background, and as a
  background behind the panel's ink. Both, because a colour is only ever used as one or the
  other and one swatch would be half an answer.

  `Enter` asks for a colour and takes everything a theme file takes: a name, `default`,
  `idx:208`, `ansi:4`, `#ff8000`, `#f80`. The field opens on what the row is set to, so a shade
  is nudged rather than retyped, and anything that is not a colour is refused by name and
  changes nothing.

  The change goes onto the **live** theme at once — a colour one cannot see is a colour one
  cannot choose — so it is on the screen before it is in a file. `F2` writes the theme into
  `fcmd.toml` as a `[[theme]]` block under its own id, which the theme loader already treats as
  an override of the built-in of that name, so the next run starts with these colours and
  `Shift+F12` still cycles the themes. `Esc` without `F2` says plainly that the colours changed
  on the screen only.

- **The mouse**, behind `ui.mouse` in *Interface settings* and **off** by default. That
  default is a decision: while a terminal is reporting the mouse it stops acting on it, so
  selecting text with the mouse and the middle-click paste go away — the user's own tools,
  which this program cannot replace. Somebody who wants the mouse says so once.

  A click moves the cursor to the row and makes that panel active; past the last row it only
  moves the focus. A double click is `Enter` on the row. The right button toggles a row's
  selection, which is the button FAR and Midnight Commander both use for it. The wheel scrolls
  whatever is **under the pointer**, three rows a notch, without taking the focus — reading the
  other panel is not working in it. The key bar is clickable and presses the key written in the
  cell with whatever modifiers are held, so `Shift` and a click is `Shift+F5`; the menu bar
  opens a menu and one click on a dropdown row runs it; a dialog's list moves its cursor and a
  double click answers it. A click *outside* an open dialog does nothing at all, because it must
  not move a cursor nobody can see.

  Every one of those ends up in the same `run` the keyboard uses, so the macro recorder, the
  confirmations and the error reporting are the ones that were already there. Where a click
  lands is worked out from a layout the renderer records as it draws, which is arithmetic over
  rectangles and therefore tested without a terminal — including the awkward case, a brief
  view whose columns run down and then across.

- **A dialog for `Alt+F6`**, where there was one line and a fixed kind of link. It offers the
  three kinds and says what each one is for: a *relative* symbolic link survives the whole tree
  being moved or copied elsewhere, an *absolute* one survives the link alone being moved, and a
  *hard* link is a second name for the same bytes, which survives the original name being
  deleted and cannot cross a filesystem or name a directory. The choice is remembered for the
  next one, as the pack and checksum dialogs remember theirs.

- **WebDAV panels**, `webdav://` and `webdavs://`, the netbox's third protocol. The ports are
  80 and 443 when a URL does not say; the path is the one the server publishes, which on
  Nextcloud is `/remote.php/dav/files/<user>/`; a share that needs no login is reached by
  leaving the user out.

  The client is hand-written over `reqwest`, in the same shape as the object store's: WebDAV is
  six HTTP methods and one XML document, and a crate for it would have brought a date library
  and a serialiser along to do what one readable, testable file does. A listing is a `PROPFIND`
  of depth 1 asking for four properties by name — `allprop` would make some servers answer with
  a page of locks and quotas per entry — and the reader skips a property the server reports as
  missing, which is what keeps a directory from being shown as an empty file.

  A `PUT` streams: the caller writes to one end of a pipe and the request reads the other, so a
  four-gigabyte file is a transfer rather than an allocation. Closing the writer is the moment
  the upload finishes and it waits for the server's answer — a refused `PUT` reported as a
  success would be a save that silently did nothing.

  What WebDAV cannot do is refused with the reason rather than quietly skipped. A file's
  modification time is a *live* property in RFC 4918, which means the server owns it, and HTTP
  has no permission bits at all — so a copy onto a WebDAV panel says it could not carry the
  timestamp over instead of leaving the file dated now and reporting success. A `MOVE` is sent
  with `Overwrite: F`, so `F6` onto a name that is already taken is refused rather than
  replacing something.

  Its tests run against a WebDAV server inside the test binary — a small HTTP server that
  answers the six methods, including a chunked `PUT`, so the URL building, the XML reading, the
  streamed body and the error messages are all exercised over a real socket.

- **FTP and FTPS panels**, in the netbox beside SFTP. `ftp://me@host/pub` and
  `ftps://me@host/pub` are panels; the session editor has a protocol row; the port follows the
  protocol when a URL does not name one, and a server offering both spellings on port 21 is two
  connections rather than one, because a connection is now keyed by its scheme as well as its
  host.

  It is the same `RemoteFs` trait the SFTP client implements, so everything above that line —
  the panel, the viewer, the editor, the job engine, the synchroniser, the finder — works on an
  `ftp://` path without knowing that it is one. What the protocol *cannot* do is said once, in
  the capability table, rather than discovered halfway through an operation: no links, no
  truncate, no random write, and no cheap seek.

  Three things in it are worth knowing about:

  **A transfer gets a connection of its own.** FTP sends a file's bytes over a second socket
  and a control connection can only have one of those at a time, so holding it for the length
  of a download would make a listing wait — and would deadlock a copy from one directory of a
  server to another, which reads and writes at once. So a transfer dials and logs in again,
  which is what every graphical FTP client does. The password is therefore kept in memory
  while the connection is open; it is never written anywhere, and `anonymous` and `ftp` are
  not asked for one at all.

  **An upload is not finished until the server says so.** Closing the writer shuts the data
  connection *and* reads the reply on the control connection, and a refusal there is reported
  as the error it is. Skipping that is exactly how an upload comes to look successful and leave
  nothing on the server.

  **A `LIST` row carries no date.** `MLSD` is asked for first and gives the time in UTC to the
  second. A server too old for it answers with the text its own `ls` prints, where the year is
  missing on recent files and the time zone is never stated — so those rows are shown with no
  date at all. A date wrong by a year is worse than no date, because sorting by it and "only
  files newer than these" would both believe it.

  Times and modes travel where the server can take them, through `MFMT` and `SITE CHMOD`, and
  a server that has neither says so in its own words rather than quietly losing a timestamp.
  FTPS is explicit TLS — `AUTH TLS` before the password, protected data connections, the
  certificate checked against this machine's own authorities, which is what makes a server
  behind a company CA work with nothing turned off.

  The tests run against an FTP server inside the test binary: a login, both listing commands,
  both passive negotiations, a download, an upload, and the awkward parts of the protocol over
  a real socket.

- **7z, read and written, in pure Rust.** `Enter` opens a `.7z` as a panel, `F3` reads an entry
  out of it, `F5` copies out, `Shift+F1` builds one, `Shift+F2` extracts one, `Shift+F3` tests
  every entry, and `F4`/`F6`/`F8` inside one change it — the same seven things that already
  worked for a zip and a tarball, now for the seventh format, with no `7z.dll` and no external
  program.

  The format's own shape shows through in two places, and it is better to say so than to
  pretend otherwise. Its index is a header read in one go, so *listing* a 7z is as quick as
  listing a zip. Its files, on the other hand, are usually **solid** — several of them in one
  compressed stream — so reading one means decoding what is in front of it, and changing one
  means decoding and packing the rest again. A zip entry nobody touched is copied across as
  the compressed bytes it already is; a 7z has no such shortcut.

  What this program *writes* is LZMA2 with each file in a block of its own. That is larger than
  what 7-Zip writes by default, and it is the right trade for a file manager: the common act
  here is reading one file out of an archive, and a solid archive makes that mean decoding
  everything before it.

  An encrypted 7z — including one whose header is encrypted, where not even the names are
  readable — is recognised and reported as wanting a password, rather than as a broken
  archive. There is nowhere to type one yet, and the message says that too.

- **File associations.** A `[[assoc]]` block in `fcmd.toml` is a mask, a name and three
  commands — `open` for `Enter`, `view` and `edit` for `Alt+F3` and `Alt+F4` — and the first
  block whose mask matches the file wins. The mask is a whole name (`Makefile`) or `*.` and an
  extension (`*.md`), compared without regard to case, by the same rule the decompile rules
  use, so the two cannot drift apart. Commands take the substitutions the user menu and
  `Ctrl+G` take: `{file}` quoted, `{file.raw}` bare, `{name}`, `{dir}`, `{other.dir}`,
  `{list}`, `{ask:…}`.

  `F3` and `F4` stay **ours** whatever a block says, which is FAR's arrangement and the reason
  its alternative-viewer keys exist: a key that sometimes opened something else would be a key
  nobody could trust. What a block names for reading and writing is on `Alt+F3` and `Alt+F4`,
  and with nothing named there either those two fall back to `$PAGER` and to `$VISUAL` or
  `$EDITOR` — already set on a machine that has an opinion.

  With no block to go on, `Enter` hands the file to the desktop: `xdg-open`, `open`, or
  `cmd /c start`. `general.desktop_open` turns that off for a machine where it is the wrong
  thing to do, and with it off `Enter` says there is nothing to open the file with rather than
  guessing. Archives are still checked first, so an association for `*.zip` cannot take away
  `Enter` on an archive.

  **Open with**, in the Commands menu, lists every block that matches the file under the
  cursor plus the desktop, and runs the one chosen — the way to use a block's `view` without
  rebinding anything.

- **Synchronising two directories** (`Ctrl+Shift+Y`, and the Commands menu). Three steps, and
  the middle one is the point: a dialog saying how to decide, a **table** saying what was
  decided, and only `F2` setting anything going. Nothing is written before that, so the table
  is the whole of what is about to happen and the dry run at the same time, and every row of
  it can be changed first — `Space` walks the answers a row allows, `←` and `→` say which way,
  `=` leaves it alone.

  One way (the left panel is the source) or both ways with the newer side winning, into the
  subdirectories, an offer to take out what the other side does not have, a mask, and *compare
  the bytes* for the change that a size and a time cannot see. Two seconds of slack on the
  times, because that is what a trip through a memory stick or a zip does to a timestamp and
  treating it as a change would copy everything, every time.

  Two directories both sides have are walked rather than paired — what differs is inside them
  — and a directory only one side has is one row, because the copy takes its tree and a row
  per file inside it would be a hundred answers to one question.

  There is no sync job kind: the table hands out the ordinary copies and deletes, copies first
  so a row that takes something away cannot run before the row that was going to replace it,
  queued one at a time, overwriting without asking because the table *was* the question. A row
  that takes a file out follows the wastebasket setting, as `F8` does. What made it fit the
  existing engine is one new copy option, `relative_to`: with a root to be relative to,
  `a/b/c.txt` lands at `target/a/b/c.txt`, so one job carries a whole tree at the paths it had.

  Both sides have to be on this machine for now, and it is said before the dialog opens.

- **`Ctrl++`, `Ctrl+−` and `Ctrl+0`** ask the terminal for a larger font, a smaller one, or
  the one it started with. A program in a terminal does not own its font — the emulator does,
  and there is exactly one control sequence for asking: xterm's `OSC 50`, where a `#` and a
  signed number step through the font menu. So that is what these send, and what happens next
  is the terminal's business. On most emulators (VTE, Konsole, Windows Terminal, alacritty,
  foot) `Ctrl` with plus or minus is the emulator's own zoom and never reaches a program at
  all, which gets the same result by a shorter road. The status line says the request was
  made, not that it was granted, because there is no reply to wait for.

  FAR's `Ctrl+Gray +`/`−` — select and unselect by the current file's extension — moves one
  modifier along to `Ctrl+Shift++`/`Ctrl+Shift+−`, and both are now in the Select block of the
  Files menu, which the keymap file already claimed and did not have.

- **Checksums.** MD5, SHA-1 and SHA-256 over what is selected, written in the format
  `md5sum`, `sha1sum` and `sha256sum` read — `<hex>  <name>` — because the point of a checksum
  file is that whoever you send it to can check it with the tool they already have. One list
  for the lot, named after the directory or anything else you type, or one beside each file.
  *Check against a list* goes the other way, on the list under the cursor.

  Which hash made a list is read off the digests in it rather than off the file's name: 32 hex
  characters is MD5, 40 is SHA-1, 64 is SHA-256. A list is called `SHA256SUMS` or `sums.txt`
  or nothing in particular, and a name can lie where a length cannot. BSD's
  `SHA256 (name) = …`, which macOS and FreeBSD write, is read too.

  What does not match is named in the report and **left selected in the panel**, the way a
  comparison by content leaves what differs: the next thing one does with a bad download is
  fetch it again or delete it, and both of those work on the selection. `Esc` stops a run.

  The reading is `JobFs::read_chunks`, the reading half of a copy with the writing half left
  out, so the day a provider implements it this works there unchanged. Today the local
  filesystem is the one that does, and anything else is refused before the dialog opens. The
  three hashes were already in the tree under the SSH client, so this costs no new dependency.

- **The system's own wastebasket.** `F8` hands what you chose to the wastebasket your
  desktop's file manager shows, so what goes in can be taken back out from there — the XDG
  trash spec on Linux, `NSFileManager` on macOS, `IFileOperation` on Windows, through the
  `trash` crate. On by default, as it is in FAR and in Total Commander: the one key everybody
  presses by accident is the one that should land in a safety net. `Shift+Del` deletes
  outright whatever the setting says, which is what that key means everywhere else, and *Use
  the system recycle bin* in the system settings is the setting — a row that was in that
  dialog, drawn dim, since the dialog existed.

  It is a job kind of its own rather than an option on a delete, because the shape of the work
  is different: a wastebasket takes a directory with everything in it in one operation, so
  there is no tree to walk and no file to count. Only this machine has one — a server, an
  archive and a bucket are deleted outright, and the dialog says which of the two is about to
  happen rather than leaving it to be guessed. If the wastebasket refuses a path it ought to
  have taken, the error names the key that deletes it instead.

- **The folders kept to hand.** `Ctrl+Shift+0` to `Ctrl+Shift+9` put the directory you are in
  on a digit and `Alt+0` to `Alt+9` take you back to it — ten of them, which is FAR's own
  number, with `Alt` rather than FAR's right-hand control key because no terminal can tell the
  two control keys apart. `Ctrl+D` opens the whole hotlist, Total Commander's key for it: the
  folders with a digit first and in order, then the rest, each with a name of its own if it
  was given one. It is the one list that is edited from inside it — `Ins` keeps the directory
  you are in, `Del` gives a folder up, `F4` names it — because that is where a folder is read.

  They live in the state file rather than in `fcmd.toml`, because a *key* sets them: a
  shortcut that waited for *Save setup* and vanished without it would be a bug, not a setting.
  A shortcut on a server or in a bucket works like any other and opens the connection first,
  the same route the drive list takes.

- **Tabs per panel.** A panel can hold more than one directory at a time: `Ctrl+Alt+T` opens
  another tab — on the directory under the cursor if the cursor is on one, and otherwise a
  duplicate of this one — `Ctrl+Alt+N` and `Ctrl+Alt+P` step along the row, `Ctrl+Alt+W` closes
  one, and `Ctrl+Alt+M` hands one to the other panel, which is a move rather than a copy. The
  Left and Right menus hold all of it, and *Name this tab*.

  A tab **is** a panel, and that is the whole design: everything a tab has to remember is
  already what a panel remembers, so a side's tabs are a list of panels with one of them on
  screen — and the one on screen is deliberately not in the list, because a parked copy of it
  beside the live one would be a second answer to where the panel is. Switching a tab is a
  move rather than a copy, whatever the directory holds.

  `Ctrl+Alt+K` locks a tab to its directory. A locked tab refuses to move whichever way it is
  asked — `Enter`, the history, the tree driving it from the other side, a plugin — and says
  why rather than doing nothing; the one thing that takes the lock off is the connection under
  it closing, because a tab cannot keep a promise about a directory it can no longer reach.

  The row is drawn only by a panel that has more than one tab: a bar that was always there
  would cost every panel a row of files to say that it has one tab. Tabs are kept between
  sessions, with their names and their locks; the directory the shell was in still wins for
  the tab one lands on, unless that tab is locked.

  The keys are the `Ctrl+Alt` row and not FAR's, because FAR has no tabs and the two keys this
  once wanted are taken: `Ctrl+T` is the tree panel and `Ctrl+Tab` steps between the open
  screens. `Ctrl+Alt` with an arrow is avoided because most desktops switch workspaces with
  it, and `Ctrl+Alt+L` because most of them lock the screen.

  Still to come: reordering a tab within its row, and a list of them to pick from.

- **A real terminal** (`Ctrl+Alt+E`). The command line under the panels runs a command and
  keeps what it printed; this runs `vim`, `top` and `ssh`. It is a pseudo-terminal — `openpty`
  on Unix, ConPTY on Windows — so the program asks the system what it is talking to and gets
  the answer it needs: it is told the size, told again when the window changes, and draws with
  the escape sequences it would send to any terminal.

  Those sequences go through a VT parser into a grid of the same type the panels draw into, so
  putting a terminal on the screen is a copy and the colours a program asks for are the ones
  the backend already knows how to send. What is implemented is what a shell, a pager, an
  editor and a full-screen monitor actually emit: cursor movement, erase in display and in
  line, insert and delete characters and lines, scrolling regions, the SGR attributes with
  256-colour and 24-bit colour, save and restore, autowrap, the title, and the alternate
  screen — which is what makes `vim` leave the shell's output alone when it exits.

  It is a screen like the viewer: `Ctrl+Alt+E` again parks it with the program still running,
  `F12` lists it and brings it back, and pressing the key from the panels reuses a parked one
  rather than starting a second shell. Inside it every key belongs to the program and there is
  no key bar, because one offering our keys in there would be a lie. `[term] shell` names what
  to run; empty means the shell the session already uses.

  Still to come: scrollback one can scroll (the lines are kept, nothing shows them yet),
  searching the output, and the mouse.

### Fixed

- **`Ctrl+↑` made the panels taller.** It brings their bottom edge *up*, so they get shorter
  and the command output underneath gets the rows; `Ctrl+↓` gives them back. The keys were
  right and the sign behind them was not: `panel.height(N)` now means the panels' height,
  which is what the keys read as, rather than the number of rows they give up.

## [0.3.0] — 2026-10-25

An archive and a bucket stop being read-only windows, a `.md` file opens as a document, and
the two dialogs that had the least to say — the copy prompt and the missing attributes one —
say what the engine underneath them could already do.

### Fixed

- Three tests were written in the shape of Unix and had been failing on Windows: a path like
  `/tmp/x` is *relative* there, where a path with no drive letter is; the dialog-scrolling
  test assumed the attributes dialog is twenty rows tall, which it is only where there is a
  mode to offer; and it looked for the file's name as the dialog's first row and found the
  panel's row of the same name behind it. Reading them again turned up one real limit, now
  documented: a folder's *own* times cannot be set on Windows through the standard library,
  so they are left alone rather than reported once per folder.

### Added

- **Writing into an archive.** An archive is no longer a read-only window. `F4` on an entry
  opens it in the editor and saving puts it back in; `F6` renames an entry or moves it to
  another directory inside the same archive; `F8` takes entries out, the selection included
  and a directory with its tree; and `F5` or `F6` *into* an archive — with the other panel
  standing inside one — put files in, `F6` deleting the originals once the archive is
  written. It works for zip and the whole tar family (plain, gzip, bzip2, xz, zstd).

  None of these formats can be edited in place, so a change means writing the archive again,
  and the mechanism is the honest one: the old archive is streamed into a `.part` file beside
  it with the change applied, the result is opened and checked against what was asked for, and
  only then does it replace the original. An interrupted change — a full disk, `Esc`, a crash
  — therefore leaves the archive exactly as it was and a `.part` file to delete. Entries
  nobody named are copied across as the compressed bytes they already are, so taking one file
  out of a hundred-megabyte zip is a copy and not a recompression. It reports as a job, with a
  progress bar and `Esc`, and the archive cache is dropped either side of it so no panel
  answers from a stale index.

- **The copy dialog** (`F5`, `F6`). The one-line prompt became a dialog, and every row below
  the destination is something the engine could already do and the prompt had no way to ask
  for: what to do if the file is already there (ask, overwrite, overwrite when the source is
  newer, skip, give the copy a free name, add to the end of it), *only files newer than the
  ones there* as a box of its own beside that choice, a mask so only the matching names
  travel, whether to copy what a link points at instead of the link, and whether to keep the
  times and the permissions. The defaults are the prompt's behaviour exactly, so typing a path
  and pressing Enter does what it always did; the answers are remembered for the next dialog.

  The destination line takes **several destinations**, `;`-separated and quoted when a name
  contains one. They run one after the other, and only the first of a *move* is a move —
  deleting the source after it would leave the second destination nothing to carry. A
  cancelled or failed destination drops the rest.

  The mask applies to files and never to directories, and a directory is planned only once
  something under it has matched: copying `*.rs` out of a tree gives the directories that have
  Rust in them and no others, which is the difference between a filtered copy and a copy
  followed by a cleanup.

- **The attributes dialog** (`Ctrl+A`). What a file *is*, rather than what is in it: on Unix
  the mode as twelve boxes — owner, group and world × read, write, run, plus setuid, setgid
  and sticky — with the line above them saying what it is now (`rw-r--r-- (0644)`); on Windows
  its four bits instead; read-only on both, because on Unix it is the write bits. Then the
  modification and access times, and, with a folder among what is selected, *and everything
  inside the folders*.

  It reports like a copy, with a progress box, a count and `Esc`, and a file that refuses is
  listed with its reason instead of being passed over — which is why this is `fcmd_jobs::attrs`
  and not `JobFs::apply_meta`: that one is a copy's best effort and swallows what it cannot do,
  which is right after a copy and wrong when somebody has just ticked a box. A directory is
  changed after its contents, so taking the write bit off one cannot put its own contents out
  of reach halfway through. A file on a server gets the mode and the modification time, which
  is what SFTP carries; an entry in an archive and an object in a bucket have nothing to set
  and say so.

- `Ctrl+A` is therefore the attributes dialog, as it is in FAR, and **select-all moved to
  FAR's own `Shift+Gray +`** (`Shift++`), with `Ctrl+Shift+A` beside it for terminals that do
  not report `Shift` with a printable key. Deselect-all is `Shift+-`.

- **A dialog taller than the screen scrolls to the cursor**, with a mark on the frame where a
  scrollbar would be. The attributes dialog has twenty rows on Unix, and a terminal of
  twenty-four lines would otherwise have drawn its last rows below the bottom of the screen,
  where no key could reach them.

- **Writing to object storage.** A bucket is no longer a read-only window. `F5`, `F6`, `F7`
  and `F8` work on one as they do anywhere else, with the same dialog, progress bar, overwrite
  question and `Esc`, and `F4` edits an object and saves it back. A copy into a bucket is an
  upload streamed off the disk; a copy out of one is a download written straight to the file;
  a copy between two keys of the same service is done by the service, so the bytes never come
  through here.

  The engine knows nothing about object storage: `fcmd_s3::jobs::ObjectFs` wraps the job
  filesystem the way the archive layer does, takes the `s3://` paths and hands everything else
  through. What the protocol cannot do is said rather than faked — there are no directories
  (`F7` writes the empty slash-named object every console uses to show an empty prefix), a
  rename is a copy and a delete and therefore not atomic, and an object carries only the time
  the service wrote it, so a copy into a bucket does not preserve a file's modification time.
  A service configured without credentials still reads and nothing more, and its rows say so.

- **Markdown laid out to be read.** A `.md` file opens in the viewer as a document rather
  than as its source: the marks come off and the space they held is put to use. A heading is
  bold with a rule under it, a list is bullets with hanging indents and a task list is boxes,
  a quote gets a bar down its side, a link shows its text and its address, a fenced block is
  indented and — where the fence named a language — coloured as that language, and a table is
  drawn with box-drawing characters, its columns lined up and aligned as its delimiter row
  asked. `Shift+F2` switches between the document and its source, `Shift+F4` has it in the
  ring with the text, the dump and the hex, and *Interface → Markdown laid out to be read*
  decides which of the two the first screen is.

  It is the one mode that reads the whole file, because a table's columns cannot be worked out
  from the part of it on screen; over four megabytes a file stays its source and says so.
  Searching a laid-out document searches what is on screen rather than the marks behind it, so
  a word in a table is found in its cell.

- `fcmd-syntax` gained `of_language`, which turns a language's *name* into a syntax — what a
  fenced code block's info string says — where `of_name` turns a file name into one.

- The interface settings now have the two colouring switches that were only in the file:
  *Colour the languages we recognise* and *Markdown laid out to be read*.

- A path typed into a dialog may now name a place inside an archive
  (`/home/me/a.zip!/src`), which is what the other panel's directory reads as when that panel
  is inside one.

## [0.2.0] — 2026-10-24

Buckets in a panel, and the chrome of the thing finally looks like what it is modelled on.
Object storage browses by prefix, so a bucket of 1.7 million objects opens as fast as one with
ten; the panels wear FAR's double frame, its column rules and its foot; the key bar follows
the modifier you are holding; the editor and the viewer colour twenty-five languages; and
`F1` now answers the question people actually ask it, which is what the keys do *here*.

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
  an `[[s3]]` block of `fcmd.toml`. `s3://work@bucket/` picks a service by name and
  `s3://work@/` lists the buckets it can see.

  A bucket has no directories: `2024/08/13/access.log` is one key with slashes in it. What the
  panel shows as a folder is a *common prefix*, which the service computes, so opening a prefix
  costs one request whether the bucket holds a thousand keys or ten million. Asked for
  everything under a prefix instead, the listing really is the whole bucket and it stops at
  `listing_cap` rows (100 000 by default) — and when it stops the footer says `(partial)` for
  as long as that listing is on screen, with a line saying why. SPEC 04 §8 has the measured
  numbers for a bucket of 1.7 million objects, since that is the question this started from.

  No access key is stored in `fcmd.toml`, ever: a block says *where* its keys come from, and
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

- **`reqwest` stays at 0.12**, and the bot's upgrade to 0.13 is refused — by the build rather
  than by an opinion. In 0.13 the rustls support is built on `rustls-platform-verifier`, which
  carries `webpki-root-certs`: the Mozilla root data again, under CDLA-Permissive-2.0, which is
  the licence this crate was moved off `webpki-roots` to avoid in the first place. It is not an
  optional dependency there, so no combination of features gets round it, and `cargo deny`
  refuses the tree. Worth revisiting when `rustls-platform-verifier` stops carrying it.
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
  readable list of names in `fcmd-macros.toml` that one can read, edit, diff and carry to
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
- **A manual page**, `docs/fcmd.1`, generated from the command line's own options with the
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

Everything is a file you can read: `fcmd --paths` says where they are, `fcmd --dump-config`
prints the settings as the program has them, and `fcmd --check-keymap` says whether a keymap is
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
- **`fcmd-arc` reads zip and the tar family** — plain, gzip, bzip2, xz and zstd — and builds
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
- The release archives are now `fcmd-<platform>-<version>` and hold the files at their root,
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
- **The engine is its own crate** (`fcmd-find`), so the dialog does not know how a walk works
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

- `fcmd-plug`: manifests discovered from disk, with themes, keymaps, associations, decompile
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
  force — rebind it in `fcmd.toml` and the menu says the new key.
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
- `Save setup` (`Shift+F9`) writes `fcmd.toml` as the configuration now stands, themes that came
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
  keep-alive, compression. Connections no longer need `fcmd.toml` to be edited by hand.
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
  `fcmd.toml` (if you want one) and an optional plugins folder.
- Non-interactive modes for scripts and CI: `--list-themes`, `--list-commands`,
  `--list-languages`, `--check-keymap`, `--export-theme`, `--dump-config`, `--paths`.

### Not here yet

Archives, the trash (`F8` deletes permanently), the command line and the user screen, the menu
(`F9`), find files (`Alt+F7`), FTP/WebDAV/S3, the plugin host, syntax highlighting, several
open editor buffers, tabs, the diff viewer and the background job queue. Anything
unimplemented says so rather than doing nothing.

[0.0.1]: https://github.com/matteobaccan/FinalCommander/releases/tag/v0.0.1
