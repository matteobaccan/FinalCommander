/// What a file of `size` bytes takes on a device whose allocation unit is `cluster` bytes:
/// the size rounded up to the next whole cluster, which is what every file manager calls the
/// allocated size, and `size` again for a device that would not say.
///
/// Arithmetic rather than a syscall on purpose. Asking the system per file would cost a call
/// per row of the panel, and the answer differs only for what a listing cannot show anyway —
/// a sparse file, a compressed one, a stream small enough to live in the record itself. The
/// cluster size comes from the device, once per directory.
pub fn allocated(size: u64, cluster: u64) -> u64 {
    if cluster == 0 {
        return size;
    }
    size.div_ceil(cluster) * cluster
}

/// Human-readable size, binary units, three significant digits — the format panels use.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "K", "M", "G", "T", "P", "E"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.2} {}", UNITS[unit])
    } else if value < 100.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// Group digits with a thin separator, as the status line does (`1 234 567`).
pub fn format_count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_file_takes_whole_clusters_and_an_empty_one_takes_none() {
        use super::allocated;
        assert_eq!(allocated(0, 4096), 0);
        assert_eq!(allocated(1, 4096), 4096);
        assert_eq!(allocated(4096, 4096), 4096);
        assert_eq!(allocated(4097, 4096), 8192);
        // A device that would not say its cluster size cannot round anything.
        assert_eq!(allocated(4097, 0), 4097);
    }

    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 K");
        assert_eq!(format_size(1536), "1.50 K");
        assert_eq!(format_size(10 * 1024), "10.0 K");
        assert_eq!(format_size(100 * 1024), "100 K");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 G");
    }

    #[test]
    fn counts() {
        assert_eq!(format_count(7), "7");
        assert_eq!(
            format_count(1234567)
                .chars()
                .filter(|c| *c == '\u{202f}')
                .count(),
            2
        );
    }
}
