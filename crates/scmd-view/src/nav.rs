use crate::source::Source;

/// How much we read at a time while looking for line boundaries.
pub const SCAN_WINDOW: usize = 64 * 1024;
/// A line longer than this is treated as if it ended here, so one 2 GB "line" cannot hang the
/// viewer (FAR does the same with its maximum line length).
pub const MAX_LINE: usize = 1024 * 1024;

/// The start of the line containing `offset`: scan back to just after the previous `\n`.
pub fn line_start_at(source: &mut dyn Source, offset: u64) -> u64 {
    let mut end = offset.min(source.len());
    let mut scanned = 0usize;
    while end > 0 && scanned < MAX_LINE {
        let window = (SCAN_WINDOW as u64).min(end);
        let start = end - window;
        let bytes = match source.read_vec(start, window as usize) {
            Ok(b) => b,
            Err(_) => return start,
        };
        if let Some(pos) = memchr::memrchr(b'\n', &bytes) {
            return start + pos as u64 + 1;
        }
        scanned += bytes.len();
        end = start;
    }
    end
}

/// The start of the next line after the line containing `offset`, if any.
pub fn next_line_start(source: &mut dyn Source, offset: u64) -> Option<u64> {
    let len = source.len();
    let mut at = offset;
    let mut scanned = 0usize;
    while at < len && scanned < MAX_LINE {
        let bytes = match source.read_vec(at, SCAN_WINDOW) {
            Ok(b) if !b.is_empty() => b,
            _ => return None,
        };
        if let Some(pos) = memchr::memchr(b'\n', &bytes) {
            let next = at + pos as u64 + 1;
            return (next < len).then_some(next);
        }
        scanned += bytes.len();
        at += bytes.len() as u64;
    }
    // A line longer than MAX_LINE: treat the cut as a boundary rather than stalling.
    (at < len).then_some(at)
}

/// The start of the line before the one containing `offset`, if any.
pub fn prev_line_start(source: &mut dyn Source, offset: u64) -> Option<u64> {
    let start = line_start_at(source, offset);
    if start == 0 {
        return None;
    }
    Some(line_start_at(source, start - 1))
}

/// The bytes of the line starting at `offset`, without the line terminator, plus the offset of
/// the next line (or the end of the file).
pub fn read_line(source: &mut dyn Source, offset: u64) -> (Vec<u8>, u64) {
    let len = source.len();
    if offset >= len {
        return (Vec::new(), len);
    }
    let mut collected: Vec<u8> = Vec::new();
    let mut at = offset;
    loop {
        let bytes = match source.read_vec(at, SCAN_WINDOW) {
            Ok(b) if !b.is_empty() => b,
            _ => break,
        };
        if let Some(pos) = memchr::memchr(b'\n', &bytes) {
            collected.extend_from_slice(&bytes[..pos]);
            at += pos as u64 + 1;
            break;
        }
        collected.extend_from_slice(&bytes);
        at += bytes.len() as u64;
        if collected.len() >= MAX_LINE || at >= len {
            break;
        }
    }
    // A CRLF file should not show a stray carriage return.
    if collected.last() == Some(&b'\r') {
        collected.pop();
    }
    (collected, at.min(len))
}

/// Snap an arbitrary offset (from a percentage jump) to the start of its line.
pub fn snap_to_line(source: &mut dyn Source, offset: u64) -> u64 {
    if offset == 0 {
        0
    } else {
        line_start_at(source, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::BytesSource;

    fn src() -> BytesSource {
        BytesSource::new(b"one\ntwo\r\nthree\nfour".to_vec())
    }

    #[test]
    fn line_boundaries_are_found_in_both_directions() {
        let mut s = src();
        assert_eq!(line_start_at(&mut s, 0), 0);
        assert_eq!(line_start_at(&mut s, 2), 0);
        assert_eq!(line_start_at(&mut s, 4), 4);
        assert_eq!(line_start_at(&mut s, 6), 4);
        assert_eq!(next_line_start(&mut s, 0), Some(4));
        assert_eq!(next_line_start(&mut s, 4), Some(9));
        assert_eq!(
            next_line_start(&mut s, 15),
            None,
            "the last line has no next"
        );
        assert_eq!(prev_line_start(&mut s, 9), Some(4));
        assert_eq!(prev_line_start(&mut s, 0), None);
    }

    #[test]
    fn reading_a_line_strips_the_terminator_and_any_cr() {
        let mut s = src();
        let (bytes, next) = read_line(&mut s, 0);
        assert_eq!(bytes, b"one");
        assert_eq!(next, 4);
        let (bytes, next) = read_line(&mut s, 4);
        assert_eq!(bytes, b"two", "CRLF must not leave a carriage return");
        assert_eq!(next, 9);
        let (bytes, next) = read_line(&mut s, 15);
        assert_eq!(bytes, b"four");
        assert_eq!(next, 19, "the last line ends at the end of the file");
    }

    #[test]
    fn an_empty_source_has_one_empty_line() {
        let mut s = BytesSource::new(Vec::new());
        assert_eq!(line_start_at(&mut s, 0), 0);
        assert_eq!(next_line_start(&mut s, 0), None);
        assert_eq!(read_line(&mut s, 0), (Vec::new(), 0));
    }

    #[test]
    fn a_line_longer_than_the_window_still_advances() {
        let mut long = vec![b'x'; SCAN_WINDOW * 2 + 17];
        long.push(b'\n');
        long.extend_from_slice(b"after");
        let mut s = BytesSource::new(long);
        let (bytes, next) = read_line(&mut s, 0);
        assert_eq!(bytes.len(), SCAN_WINDOW * 2 + 17);
        assert_eq!(next_line_start(&mut s, 0), Some(next));
        assert_eq!(read_line(&mut s, next).0, b"after");
    }

    #[test]
    fn percentage_jumps_land_on_a_line_start() {
        let mut s = src();
        let len = s.len();
        let offset = snap_to_line(&mut s, len / 2);
        assert_eq!(offset, 9, "half of the file is inside `three`");
        assert_eq!(snap_to_line(&mut s, 0), 0);
    }
}
