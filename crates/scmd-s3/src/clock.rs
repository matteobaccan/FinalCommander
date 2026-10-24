//! Dates, in the two shapes object storage speaks: the compact stamp a signature carries, and
//! the ISO 8601 instant a listing gives for every object.
//!
//! Neither is worth a dependency. The conversion both ways is Howard Hinnant's, which counts
//! in eras of four hundred years — the span over which the calendar repeats exactly — and so
//! has no leap-year special case to get wrong in either direction.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Days since 1970-01-01 to a calendar date.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A calendar date to days since 1970-01-01.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2024-08-13T10:11:12.000Z` — the only form S3 writes a time in, and the one `LastModified`
/// carries on every row of every listing.
///
/// Anything else gives `None` rather than a guess: a listing with an unreadable date is a
/// listing whose rows have no date, which the panel already knows how to show, while a date
/// invented from a half-parsed string is a lie that sorts.
pub fn parse_iso8601(text: &str) -> Option<SystemTime> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { text.get(from..to)?.parse().ok() };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let (h, mi, s) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    // A fraction of a second is allowed and thrown away; a zone offset other than Z is not
    // what S3 sends, and the one thing not to do with it is pretend it is Z.
    let rest = &text[19..];
    let rest = match rest.find(|c: char| !c.is_ascii_digit() && c != '.') {
        Some(i) => &rest[i..],
        None => "",
    };
    if !matches!(
        rest,
        "" | "Z" | "z" | "+00:00" | "-00:00" | "+0000" | "-0000"
    ) {
        return None;
    }
    let secs = days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + s;
    if secs >= 0 {
        Some(UNIX_EPOCH + Duration::from_secs(secs as u64))
    } else {
        UNIX_EPOCH.checked_sub(Duration::from_secs((-secs) as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs_of(time: SystemTime) -> i64 {
        time.duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_else(|e| -(e.duration().as_secs() as i64))
    }

    #[test]
    fn the_two_directions_agree_for_every_day_of_two_centuries() {
        for days in -25_000..45_000 {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y:04}-{m:02}-{d:02}");
        }
    }

    #[test]
    fn a_last_modified_is_read_as_the_instant_it_names() {
        assert_eq!(
            secs_of(parse_iso8601("1970-01-01T00:00:00.000Z").unwrap()),
            0
        );
        assert_eq!(
            secs_of(parse_iso8601("2013-05-24T00:00:00Z").unwrap()),
            1_369_353_600
        );
        // The fraction is dropped, not misread as seconds.
        assert_eq!(
            secs_of(parse_iso8601("2024-08-13T10:11:12.987Z").unwrap()),
            secs_of(parse_iso8601("2024-08-13T10:11:12Z").unwrap())
        );
        // A leap second is not rejected, and a leap day is a day.
        assert!(parse_iso8601("2016-12-31T23:59:60Z").is_some());
        assert_eq!(
            secs_of(parse_iso8601("2024-02-29T00:00:00Z").unwrap()),
            1_709_164_800
        );
        // Nothing that is not that shape comes back as a date.
        for bad in [
            "",
            "2024-08-13",
            "yesterday",
            "2024-08-13 10:11:12Z",
            "2024-13-01T00:00:00Z",
            "2024-08-32T00:00:00Z",
            "2024-08-13T25:00:00Z",
            "2024-08-13T10:11:12+02:00",
            "Tue, 13 Aug 2024 10:11:12 GMT",
        ] {
            assert!(parse_iso8601(bad).is_none(), "{bad:?}");
        }
    }
}
