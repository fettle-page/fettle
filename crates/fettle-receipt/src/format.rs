//! Byte sizes, counts, temperatures, days and the scan stamp, as receipt-ui's format.ts prints them.

use fettle_verdicts::{Locale, fmt};

/// Where the reader of a receipt is, for the scan stamp. Reports store UTC, which doesn't reveal
/// where the PC is; whoever reads one sees their own time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// For server-drawn pages, where the viewer is unknown: the stamp says "UTC".
    Utc,
    /// The viewer's offset from UTC at the scan instant, in minutes east (the app asks the OS).
    Offset(i32),
}

/// Decimal units, like the label on the drive's box: one decimal under 10, none from 10 up.
pub fn bytes(n: f64, locale: Locale) -> String {
    let (value, unit) = if n >= 1e12 {
        (n / 1e12, "TB")
    } else if n >= 1e9 {
        (n / 1e9, "GB")
    } else {
        (n / 1e6, "MB")
    };
    format!("{} {unit}", fmt::group(value, if value < 10.0 { 1 } else { 0 }, locale))
}

pub fn int(n: f64, locale: Locale) -> String {
    fmt::group(n, 0, locale)
}

pub fn temp(c: f64) -> String {
    format!("{} °C", fmt::js_number(c))
}

/// YYYY-MM-DD as a short local date; impossible dates (31 Feb) come back as written.
pub fn iso_day(day: &str, locale: Locale) -> String {
    fmt::date_medium(day, locale)
}

/// The scan time as "07/10/2026 22:29" in the zone; with " UTC" when the zone is UTC.
pub fn stamp(rfc3339: &str, zone: Zone) -> String {
    let Some(ms) = fmt::parse_rfc3339(rfc3339) else { return rfc3339.to_string() };
    let offset = match zone {
        Zone::Utc => 0,
        Zone::Offset(minutes) => i64::from(minutes),
    };
    let local = ms + offset * 60_000;
    let (y, m, d) = fmt::civil_from_days(local.div_euclid(86_400_000));
    let minutes = local.rem_euclid(86_400_000) / 60_000;
    let text = format!("{d:02}/{m:02}/{y} {:02}:{:02}", minutes / 60, minutes % 60);
    if zone == Zone::Utc { format!("{text} UTC") } else { text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Locale::{En, It};

    #[test]
    fn bytes_in_decimal_units_one_decimal_under_10() {
        assert_eq!(bytes(1_000_204_886_016.0, En), "1 TB");
        assert_eq!(bytes(1_800_000_000_000.0, En), "1.8 TB");
        assert_eq!(bytes(1_800_000_000_000.0, It), "1,8 TB");
        assert_eq!(bytes(512_110_190_592.0, En), "512 GB");
    }

    #[test]
    fn groups_integers_per_locale() {
        assert_eq!(int(2310.0, En), "2,310");
        // Italian (CLDR) only groups from five digits.
        assert_eq!(int(2310.0, It), "2310");
        assert_eq!(int(23100.0, It), "23.100");
    }

    #[test]
    fn temperatures() {
        assert_eq!(temp(41.0), "41 °C");
    }

    #[test]
    fn days_and_impossible_days_as_written() {
        assert_eq!(iso_day("2026-10-07", En), "7 Oct 2026");
        assert_eq!(iso_day("2026-10-07", It), "7 ott 2026");
        assert_eq!(iso_day("2026-02-31", En), "2026-02-31");
    }

    #[test]
    fn stamps_in_the_zone_given() {
        // UTC is only used where the viewer is unknown (server-drawn pages), so it says so.
        assert_eq!(stamp("2026-10-07T14:32:00+02:00", Zone::Utc), "07/10/2026 12:32 UTC");
        assert_eq!(stamp("2026-10-07T20:29:30Z", Zone::Offset(120)), "07/10/2026 22:29");
        assert_eq!(stamp("2026-10-07T23:59:59Z", Zone::Offset(540)), "08/10/2026 08:59");
        assert_eq!(stamp("2026-10-07T00:10:00Z", Zone::Offset(-300)), "06/10/2026 19:10");
    }

    #[test]
    fn leaves_an_unreadable_timestamp_as_written() {
        assert_eq!(stamp("garbage", Zone::Utc), "garbage");
    }
}
