//! Turning a date into seconds since the epoch, and back.
//!
//! Three protocols in this crate carry a time in their own spelling — FTP's `MLSD` says
//! `20260101120000`, FTP's `MFMT` wants the same, and WebDAV answers with `Mon, 01 Jan 2026
//! 12:00:00 GMT` — and all three are UTC, which is the one mercy in it. The conversion is
//! Howard Hinnant's, the same algorithm the archive provider uses for a zip's own clock, and
//! it lives here rather than in either protocol so that the two cannot drift apart.

/// Days since 1970-01-01 for a proleptic Gregorian date, or `None` for a date that is not one.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The inverse: a day count back to year, month and day.
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A whole UTC date and time as seconds since the epoch. `None` for anything before it: the
/// protocols here cannot say such a time and the panel has nowhere to put one.
pub fn seconds_from_utc(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
) -> Option<u32> {
    if !(0..24).contains(&hour) || !(0..60).contains(&min) || !(0..=60).contains(&sec) {
        return None;
    }
    let secs = days_from_civil(year, month, day)? * 86_400 + hour * 3_600 + min * 60 + sec;
    u32::try_from(secs).ok()
}

/// `YYYYMMDDHHMMSS` in UTC, which is what `MFMT` takes and what `MLSD` gives back.
pub fn compact_stamp(secs: u32) -> String {
    let (days, rest) = (i64::from(secs) / 86_400, i64::from(secs) % 86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, min, sec) = (rest / 3_600, (rest % 3_600) / 60, rest % 60);
    format!("{year:04}{month:02}{day:02}{hour:02}{min:02}{sec:02}")
}

/// The three-letter month names of an HTTP date, which is the only form WebDAV answers with.
pub fn month_of(name: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let name = name.to_ascii_lowercase();
    MONTHS.iter().position(|m| *m == name).map(|i| i as i64 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_directions_are_each_others_inverse() {
        for days in [0i64, 1, 10_957, 20_750, -1, -719_468] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), Some(days), "{days}");
        }
        assert_eq!(days_from_civil(1970, 1, 1), Some(0));
        assert_eq!(days_from_civil(2026, 1, 1), Some(20_454));
        assert_eq!(days_from_civil(2026, 13, 1), None);
        assert_eq!(days_from_civil(2026, 1, 32), None);
    }

    #[test]
    fn a_stamp_round_trips_and_a_bad_time_is_refused() {
        assert_eq!(compact_stamp(0), "19700101000000");
        assert_eq!(compact_stamp(1_767_268_800), "20260101120000");
        assert_eq!(seconds_from_utc(2026, 1, 1, 12, 0, 0), Some(1_767_268_800));
        assert_eq!(seconds_from_utc(1970, 1, 1, 0, 0, 0), Some(0));
        // Before the epoch, and out-of-range clocks.
        assert_eq!(seconds_from_utc(1960, 1, 1, 0, 0, 0), None);
        assert_eq!(seconds_from_utc(2026, 1, 1, 24, 0, 0), None);
        assert_eq!(seconds_from_utc(2026, 1, 1, 12, 60, 0), None);
        // A leap second is a real time and is not refused.
        assert!(seconds_from_utc(2016, 12, 31, 23, 59, 60).is_some());
    }

    #[test]
    fn months_are_read_by_name_whatever_the_case() {
        assert_eq!(month_of("Jan"), Some(1));
        assert_eq!(month_of("DEC"), Some(12));
        assert_eq!(month_of("sep"), Some(9));
        assert_eq!(month_of("Sept"), None);
        assert_eq!(month_of(""), None);
    }
}
