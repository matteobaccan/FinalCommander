//! Two files that are not text, byte against byte (SPEC 05 §3a).
//!
//! Lines are what the side-by-side view lines up, and a binary has none: a firmware image
//! with one byte changed is one "line" of sixteen megabytes on each side. The question one
//! asks of two binaries is a different one anyway — are they the same, and if not, *where* do
//! they stop being — and the answer is an offset and a count, with the bytes around it in hex
//! so one can see what the difference is.
//!
//! The comparison is by position: byte `n` of one file against byte `n` of the other. A byte
//! inserted near the start therefore makes everything after it differ, which is the truth
//! about two files compared this way and what `cmp` says too; lining two binaries up around
//! an insertion is a different tool.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

use fcmd_core::CancelToken;

/// The most runs of differing bytes that are remembered by position. Two files that differ
/// in more places than this are not walked difference by difference by anybody; the count of
/// bytes that differ stays exact, and the screen says the list was cut.
pub const MOST_RUNS: usize = 65_536;

/// What a whole comparison found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    pub left_len: u64,
    pub right_len: u64,
    /// Each run of consecutive bytes that differ: where it starts and how long it is. What
    /// one file has past the end of the other is the last run.
    pub runs: Vec<(u64, u64)>,
    /// How many byte positions differ in all, the longer file's tail included.
    pub differing: u64,
    /// There were more runs than [`MOST_RUNS`]; `runs` holds the first of them.
    pub capped: bool,
}

impl Scan {
    pub fn identical(&self) -> bool {
        self.differing == 0
    }

    /// The first position at which the two files differ.
    pub fn first(&self) -> Option<u64> {
        self.runs.first().map(|r| r.0)
    }
}

/// Fill `buf` from `from`, however many reads that takes; less than a full buffer means the
/// end of the file.
fn fill(from: &mut dyn Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match from.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(got)
}

/// Compare two streams from their first byte to the end of the longer one.
///
/// A block at a time, and a block that is the same on both sides is one comparison rather
/// than sixty-four thousand, which is what makes two near-identical disk images quick.
/// `None` when it was cancelled.
pub fn scan(
    left: &mut dyn Read,
    right: &mut dyn Read,
    cancel: &CancelToken,
) -> io::Result<Option<Scan>> {
    const BLOCK: usize = 64 * 1024;
    let mut a = vec![0u8; BLOCK];
    let mut b = vec![0u8; BLOCK];
    let mut out = Scan::default();
    // The run being read, if the last byte seen differed: where it began.
    let mut open: Option<u64> = None;
    let mut at = 0u64;
    let close = |out: &mut Scan, open: &mut Option<u64>, end: u64| {
        if let Some(start) = open.take() {
            if out.runs.len() < MOST_RUNS {
                out.runs.push((start, end - start));
            } else {
                out.capped = true;
            }
        }
    };
    loop {
        if cancel.is_cancelled() {
            return Ok(None);
        }
        let (got_a, got_b) = (fill(left, &mut a)?, fill(right, &mut b)?);
        out.left_len += got_a as u64;
        out.right_len += got_b as u64;
        let both = got_a.min(got_b);
        if both == 0 {
            // One file has ended: what follows is the other's tail, handled below.
        } else if a[..both] == b[..both] {
            close(&mut out, &mut open, at);
        } else {
            for i in 0..both {
                if a[i] != b[i] {
                    out.differing += 1;
                    open.get_or_insert(at + i as u64);
                } else {
                    close(&mut out, &mut open, at + i as u64);
                }
            }
        }
        // What one side has and the other has not differs, all of it.
        let longer = got_a.max(got_b);
        if longer > both {
            out.differing += (longer - both) as u64;
            open.get_or_insert(at + both as u64);
        }
        at += longer as u64;
        if got_a < BLOCK && got_b < BLOCK {
            break;
        }
    }
    close(&mut out, &mut open, at);
    Ok(Some(out))
}

/// The screen: two files, a place in them, and the bytes of both at that place.
#[derive(Debug)]
pub struct BinDiff {
    /// Which comparison is this screen's: one that was abandoned may still answer.
    pub id: u64,
    pub left_name: String,
    pub right_name: String,
    left: Option<File>,
    right: Option<File>,
    pub cancel: CancelToken,
    /// What the comparison found; `None` while it is still running.
    pub scan: Option<Scan>,
    /// The offset of the first byte on the screen, always a whole number of rows.
    pub top: u64,
    /// Bytes to a row and rows to the screen, from the room the last frame had.
    pub per_row: usize,
    rows: usize,
    /// The bytes of each file from `top`, as many as the screen shows: read when the place or
    /// the room changes, so a frame only paints.
    left_bytes: Vec<u8>,
    right_bytes: Vec<u8>,
    loaded: Option<(u64, usize)>,
}

impl BinDiff {
    pub fn new(
        id: u64,
        left_name: String,
        left: File,
        right_name: String,
        right: File,
        cancel: CancelToken,
    ) -> BinDiff {
        BinDiff {
            id,
            left_name,
            right_name,
            left: Some(left),
            right: Some(right),
            cancel,
            scan: None,
            top: 0,
            per_row: 8,
            rows: 0,
            left_bytes: Vec::new(),
            right_bytes: Vec::new(),
            loaded: None,
        }
    }

    /// The comparison is back: the screen goes to the first difference, which is where one
    /// wanted to be.
    pub fn arrived(&mut self, scan: Scan) {
        let first = scan.first();
        self.scan = Some(scan);
        self.top = self.row_of(first.unwrap_or(0));
        self.load();
    }

    /// How many hex digits an offset takes: eight, and twelve for files past four gigabytes.
    pub fn offset_digits(&self) -> usize {
        match self.longest() {
            n if n > u32::MAX as u64 => 12,
            _ => 8,
        }
    }

    /// The length of the longer file, which is how far the screen scrolls.
    pub fn longest(&self) -> u64 {
        match &self.scan {
            Some(scan) => scan.left_len.max(scan.right_len),
            None => 0,
        }
    }

    /// The room the rows have, told by the renderer before it draws: how many bytes fit on a
    /// row — the offset, then each file's bytes in hex and as characters, with a rule between
    /// the two — and how many rows there are.
    pub fn fit(&mut self, w: u16, h: u16) {
        // The offset and a space, the rule with a space each side, and for each file three
        // cells a byte in hex and one cell a byte as a character.
        let fixed = self.offset_digits() + 1 + 3;
        let room = (w as usize).saturating_sub(fixed) / 8;
        let per_row = [32, 16, 8, 4].into_iter().find(|n| *n <= room).unwrap_or(4);
        let rows = h as usize;
        if per_row != self.per_row || rows != self.rows {
            self.per_row = per_row;
            self.rows = rows;
            self.top = self.row_of(self.top);
            self.load();
        }
    }

    fn row_of(&self, offset: u64) -> u64 {
        offset - offset % self.per_row as u64
    }

    /// Read what the screen shows of both files. A file that cannot be read shows as ending
    /// there, which is the most a screen of bytes can say about it.
    fn load(&mut self) {
        let want = self.per_row * self.rows;
        if self.loaded == Some((self.top, want)) {
            return;
        }
        self.loaded = Some((self.top, want));
        let top = self.top;
        let read = |file: &mut Option<File>, into: &mut Vec<u8>| {
            into.clear();
            into.resize(want, 0);
            let got = file
                .as_mut()
                .and_then(|f| {
                    f.seek(SeekFrom::Start(top)).ok()?;
                    fill(f, into).ok()
                })
                .unwrap_or(0);
            into.truncate(got);
        };
        read(&mut self.left, &mut self.left_bytes);
        read(&mut self.right, &mut self.right_bytes);
    }

    /// The bytes of one row of the screen, left and right: shorter than a row, or empty,
    /// where a file has ended.
    pub fn row(&self, row: usize) -> (&[u8], &[u8]) {
        let from = row * self.per_row;
        let part = |bytes: &'_ [u8]| -> std::ops::Range<usize> {
            from.min(bytes.len())..(from + self.per_row).min(bytes.len())
        };
        (
            &self.left_bytes[part(&self.left_bytes)],
            &self.right_bytes[part(&self.right_bytes)],
        )
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    fn last_top(&self) -> u64 {
        self.row_of(self.longest().saturating_sub(1))
    }

    pub fn scroll(&mut self, rows: i64) {
        let step = self.per_row as i64;
        let top = (self.top as i64).saturating_add(rows.saturating_mul(step));
        self.top = self.row_of(top.clamp(0, self.last_top() as i64) as u64);
        self.load();
    }

    pub fn to_start(&mut self) {
        self.top = 0;
        self.load();
    }

    pub fn to_end(&mut self) {
        let back = (self.rows.saturating_sub(1) * self.per_row) as u64;
        self.top = self.row_of(self.last_top().saturating_sub(back));
        self.load();
    }

    /// The next difference that starts below the top row. `false` when there is none, which
    /// is the caller's cue to say this was the last.
    pub fn next_difference(&mut self) -> bool {
        let Some(scan) = &self.scan else {
            return false;
        };
        let below = self.top + self.per_row as u64;
        match scan.runs.iter().find(|r| r.0 >= below) {
            Some(run) => {
                self.top = self.row_of(run.0);
                self.load();
                true
            }
            None => false,
        }
    }

    /// The last difference that starts above the top row.
    pub fn previous_difference(&mut self) -> bool {
        let Some(scan) = &self.scan else {
            return false;
        };
        match scan.runs.iter().rev().find(|r| r.0 < self.top) {
            Some(run) => {
                self.top = self.row_of(run.0);
                self.load();
                true
            }
            None => false,
        }
    }

    /// Which difference the top row is at or after, one-based, for the foot line.
    pub fn difference_at(&self) -> usize {
        let below = self.top + self.per_row as u64;
        match &self.scan {
            Some(scan) => scan.runs.iter().take_while(|r| r.0 < below).count(),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn scanned(left: &[u8], right: &[u8]) -> Scan {
        scan(
            &mut Cursor::new(left),
            &mut Cursor::new(right),
            &CancelToken::new(),
        )
        .unwrap()
        .expect("nobody cancelled")
    }

    #[test]
    fn two_files_with_the_same_bytes_are_the_same() {
        let bytes = vec![7u8; 200_000];
        let scan = scanned(&bytes, &bytes);
        assert!(scan.identical());
        assert_eq!((scan.left_len, scan.right_len), (200_000, 200_000));
        assert_eq!(scan.first(), None);
        assert!(scanned(b"", b"").identical(), "two empty files too");
    }

    #[test]
    fn the_first_differing_offset_and_every_run_after_it_are_found() {
        let left = vec![0u8; 150_000];
        let mut right = left.clone();
        right[5] = 1;
        right[6] = 1;
        // A run that crosses the end of a block is still one run.
        for b in &mut right[65_530..65_540] {
            *b = 9;
        }
        right[149_999] = 1;
        let scan = scanned(&left, &right);
        assert_eq!(scan.first(), Some(5));
        assert_eq!(scan.runs, [(5, 2), (65_530, 10), (149_999, 1)]);
        assert_eq!(scan.differing, 13);
        assert!(!scan.capped);
    }

    #[test]
    fn what_one_file_has_past_the_end_of_the_other_differs() {
        let scan = scanned(b"abcdef", b"abc");
        assert_eq!((scan.left_len, scan.right_len), (6, 3));
        assert_eq!(scan.runs, [(3, 3)]);
        assert_eq!(scan.differing, 3);
        // And a difference that runs into the tail is one run with it.
        let scan = scanned(b"abX", b"abcdef");
        assert_eq!(scan.runs, [(2, 4)]);
        assert_eq!(scan.differing, 4);
        // The tail in a later block than the end of the shorter file.
        let scan = scanned(&vec![1u8; 70_000], &[1u8; 10]);
        assert_eq!(scan.runs, [(10, 69_990)]);
    }

    #[test]
    fn more_runs_than_are_kept_are_counted_and_said_to_be_cut() {
        let left = vec![0u8; (MOST_RUNS + 5) * 2];
        let right: Vec<u8> = (0..left.len()).map(|i| (i % 2) as u8).collect();
        let scan = scanned(&left, &right);
        assert_eq!(scan.runs.len(), MOST_RUNS);
        assert!(scan.capped);
        assert_eq!(scan.differing, (MOST_RUNS + 5) as u64, "the count is exact");
    }

    #[test]
    fn a_cancelled_comparison_has_no_answer() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let got = scan(&mut Cursor::new(b"a"), &mut Cursor::new(b"b"), &cancel).unwrap();
        assert!(got.is_none());
    }

    fn view(left: &[u8], right: &[u8]) -> (tempfile::TempDir, BinDiff) {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.bin"), dir.path().join("b.bin"));
        std::fs::write(&a, left).unwrap();
        std::fs::write(&b, right).unwrap();
        let mut view = BinDiff::new(
            1,
            "a.bin".into(),
            File::open(&a).unwrap(),
            "b.bin".into(),
            File::open(&b).unwrap(),
            CancelToken::new(),
        );
        // Eighty columns and ten rows: eight bytes to a row.
        view.fit(78, 10);
        view.arrived(scanned(left, right));
        (dir, view)
    }

    #[test]
    fn the_screen_opens_on_the_first_difference_and_walks_the_others() {
        let left = vec![0x11u8; 1000];
        let mut right = left.clone();
        right[100] = 0xAA;
        right[101] = 0xBB;
        right[500] = 0xCC;
        let (_dir, mut view) = view(&left, &right);
        assert_eq!(view.per_row, 8);
        assert_eq!(view.top, 96, "the row that holds offset 100");
        let (l, r) = view.row(0);
        assert_eq!(l, [0x11; 8]);
        assert_eq!(r, [0x11, 0x11, 0x11, 0x11, 0xAA, 0xBB, 0x11, 0x11]);
        assert_eq!(view.difference_at(), 1);

        assert!(view.next_difference());
        assert_eq!(view.top, 496);
        assert_eq!(view.row(0).1[4], 0xCC);
        assert_eq!(view.difference_at(), 2);
        assert!(!view.next_difference(), "that was the last");
        assert!(view.previous_difference());
        assert_eq!(view.top, 96);
        assert!(!view.previous_difference(), "and that the first");

        view.to_end();
        assert_eq!(
            view.top,
            992 - 9 * 8,
            "the last row of the file is the last row shown"
        );
        view.scroll(1000);
        assert_eq!(view.top, 992);
        assert_eq!(view.row(0).0.len(), 8);
        assert!(view.row(1).0.is_empty(), "nothing past the end");
        view.to_start();
        view.scroll(-5);
        assert_eq!(view.top, 0);
    }

    #[test]
    fn a_shorter_file_ends_where_it_ends_and_a_wider_screen_holds_more() {
        let (_dir, mut view) = view(&[1u8; 20], &[1u8; 12]);
        assert_eq!(view.top, 8, "the row where the shorter one stops");
        let (l, r) = view.row(0);
        assert_eq!((l.len(), r.len()), (8, 4));
        let (l, r) = view.row(1);
        assert_eq!((l.len(), r.len()), (4, 0));
        // Wide enough for sixteen bytes to a row, and the place kept on a whole row.
        view.fit(140, 10);
        assert_eq!(view.per_row, 16);
        assert_eq!(view.top, 0);
        let (l, r) = view.row(0);
        assert_eq!((l.len(), r.len()), (16, 12));
    }
}
