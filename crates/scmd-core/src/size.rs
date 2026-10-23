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
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
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
