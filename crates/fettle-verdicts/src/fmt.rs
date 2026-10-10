//! Numbers and dates the way the TypeScript engine prints them through `Intl` (en-GB and it-IT)
//! and `String(n)`. No locale or time-zone data: the rules are spelled out here.

use crate::Locale;

/// The shortest decimal digits that read back as `x` (> 0), and where the point goes:
/// `x = 0.DIGITS × 10^point` (ECMAScript's Number::toString "k" and "n").
fn decimal_parts(x: f64) -> (String, i32) {
    // Rust's `{:e}` prints the shortest round-trip mantissa, e.g. "4.65e1".
    let s = format!("{x:e}");
    let (mantissa, exp) = s.split_once('e').expect("exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digits = digits.trim_end_matches('0');
    (if digits.is_empty() { "0".into() } else { digits.to_string() }, exp.parse::<i32>().expect("exponent") + 1)
}

/// JavaScript's `String(n)`: `46`, `46.5`, `1e+21`.
pub fn js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == 0.0 {
        return "0".into();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let (digits, point) = decimal_parts(n.abs());
    let k = digits.len() as i32;
    let body = if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let e = point - 1;
        let e = if e >= 0 { format!("+{e}") } else { e.to_string() };
        if k == 1 { format!("{digits}e{e}") } else { format!("{}.{}e{e}", &digits[..1], &digits[1..]) }
    };
    format!("{sign}{body}")
}

/// `x` (>= 0) rounded to `max_fraction` decimals, half away from zero on its shortest decimal form
/// (as V8's Intl does: 2.05 becomes 2.1, though the double is a hair under 2.05). Returns the
/// integer digits and the fraction digits without trailing zeros.
fn rounded(x: f64, max_fraction: u32) -> (String, String) {
    if x == 0.0 {
        return ("0".into(), String::new());
    }
    let (digits, point) = decimal_parts(x);
    let (mut int, mut frac) = if point <= 0 {
        ("0".to_string(), format!("{}{digits}", "0".repeat((-point) as usize)))
    } else if point as usize >= digits.len() {
        (format!("{digits}{}", "0".repeat(point as usize - digits.len())), String::new())
    } else {
        (digits[..point as usize].to_string(), digits[point as usize..].to_string())
    };
    let keep = max_fraction as usize;
    if frac.len() > keep {
        let round_up = frac.as_bytes()[keep] >= b'5';
        frac.truncate(keep);
        if round_up {
            // Add one at the last kept digit, carrying into the integer part.
            let mut all = format!("{int}{frac}").into_bytes();
            let mut i = all.len();
            loop {
                if i == 0 {
                    all.insert(0, b'1');
                    break;
                }
                i -= 1;
                if all[i] == b'9' {
                    all[i] = b'0';
                } else {
                    all[i] += 1;
                    break;
                }
            }
            let all = String::from_utf8(all).expect("digits");
            let split = all.len() - frac.len();
            int = all[..split].to_string();
            frac = all[split..].to_string();
        }
    }
    (int, frac.trim_end_matches('0').to_string())
}

/// `Intl.NumberFormat` with `maximumFractionDigits`: en-GB `3,633` / `1.3`; it-IT `3633`, `12.345`, `1,3`.
pub fn group(n: f64, max_fraction: u32, locale: Locale) -> String {
    let (int, frac) = rounded(n.abs(), max_fraction);
    let (sep, point) = match locale {
        Locale::En => (',', '.'),
        Locale::It => ('.', ','),
    };
    // it-IT groups only from five digits (Intl's minimumGroupingDigits of 2).
    let grouped = if locale == Locale::It && int.len() < 5 {
        int.clone()
    } else {
        let mut out = String::new();
        for (i, c) in int.chars().enumerate() {
            if i > 0 && (int.len() - i) % 3 == 0 {
                out.push(sep);
            }
            out.push(c);
        }
        out
    };
    let sign = if n < 0.0 && (int != "0" || !frac.is_empty()) { "-" } else { "" };
    if frac.is_empty() { format!("{sign}{grouped}") } else { format!("{sign}{grouped}{point}{frac}") }
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 31,
    }
}

/// A real YYYY-MM-DD day (no 13th month, no 31 February).
pub fn iso_day(s: &str) -> Option<(i32, u32, u32)> {
    let b = s.as_bytes();
    let shape = b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shape {
        return None;
    }
    let (y, m, d): (i32, u32, u32) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    ((1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)).then_some((y, m, d))
}

/// Shaped like YYYY-MM-DD, whether or not the day exists (the TypeScript `ISO_DAY` pattern).
pub fn looks_like_day(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

const EN_SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec"];
const EN_LONG: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const IT_SHORT: [&str; 12] = ["gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic"];
const IT_LONG: [&str; 12] = ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"];

fn date_with(s: &str, months: [&str; 12]) -> String {
    match iso_day(s) {
        Some((y, m, d)) => format!("{d} {} {y}", months[m as usize - 1]),
        None => s.to_string(),
    }
}

/// `dateStyle: "medium"`: `23 Sept 2026` / `23 set 2026`; anything else comes back as written.
pub fn date_medium(s: &str, locale: Locale) -> String {
    date_with(s, if locale == Locale::It { IT_SHORT } else { EN_SHORT })
}

/// `dateStyle: "long"`: `9 October 2028` / `9 ottobre 2028`; anything else comes back as written.
pub fn date_long(s: &str, locale: Locale) -> String {
    date_with(s, if locale == Locale::It { IT_LONG } else { EN_LONG })
}

/// An RFC 3339 instant (as reports store it) in milliseconds since 1970, as JavaScript's Date reads it.
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| -> Option<i64> {
        let part = s.get(r)?;
        part.bytes().all(|c| c.is_ascii_digit()).then(|| part.parse().ok())?
    };
    if b.len() < 20 || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    // As V8 reads them (the schema lets these through): any day from 1 to 31 rolls into the next
    // month ("31 February" is 3 March), and 24:00:00 is the next midnight.
    if b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let (h, min, sec) = (digits(11..13)?, digits(14..16)?, digits(17..19)?);
    if h > 24 || min > 59 || sec > 59 {
        return None;
    }
    let mut rest = &s[19..];
    // Fractional seconds: JavaScript keeps milliseconds and drops the rest.
    let mut ms = 0;
    if let Some(frac) = rest.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return None;
        }
        ms = format!("{:0<3}", &frac[..n.min(3)]).parse::<i64>().ok()?;
        rest = &frac[n..];
    }
    let offset_min = match rest {
        "Z" | "z" => 0,
        _ if rest.len() == 6 && (rest.starts_with('+') || rest.starts_with('-')) && rest.as_bytes()[3] == b':' => {
            let (oh, om): (i64, i64) = (rest[1..3].parse().ok()?, rest[4..6].parse().ok()?);
            if oh > 23 || om > 59 {
                return None;
            }
            let sign = if rest.starts_with('-') { -1 } else { 1 };
            sign * (oh * 60 + om)
        }
        _ => return None,
    };
    if h == 24 && (min, sec, ms) != (0, 0, 0) {
        return None;
    }
    let days = days_from_civil(y as i32, m as u32, 1) + d - 1;
    Some(((days * 24 + h) * 60 + min - offset_min) * 60_000 + sec * 1000 + ms)
}

/// Days since 1970-01-01 for a civil date (proleptic Gregorian; Howard Hinnant's algorithm).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date for days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((yoe + era * 400 + i64::from(m <= 2)) as i32, m, d)
}

/// The UTC day of an instant, as YYYY-MM-DD (JavaScript's `toISOString().slice(0, 10)`).
pub fn utc_day(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(86_400_000));
    format!("{y:04}-{m:02}-{d:02}")
}

/// JavaScript's `setUTCMonth(getUTCMonth() - months)`: the same day and time that many months
/// earlier, rolling a day the month doesn't have into the next (29 Feb 2028 − 12 → 1 Mar 2027).
pub fn months_earlier(ms: i64, months: i32) -> i64 {
    let day_ms = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(ms.div_euclid(86_400_000));
    let total = y * 12 + m as i32 - 1 - months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
    (days_from_civil(ny, nm, 1) + i64::from(d) - 1) * 86_400_000 + day_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use Locale::{En, It};

    #[test]
    fn numbers_print_like_javascript() {
        assert_eq!(js_number(46.0), "46");
        assert_eq!(js_number(46.5), "46.5");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(123456789012.0), "123456789012");
        assert_eq!(js_number(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(js_number(1e21), "1e+21");
    }

    #[test]
    fn numbers_group_like_intl() {
        assert_eq!(group(3633.0, 0, En), "3,633");
        assert_eq!(group(3633.0, 0, It), "3633");
        assert_eq!(group(12345.0, 0, It), "12.345");
        assert_eq!(group(1234567.85, 1, En), "1,234,567.9");
        assert_eq!(group(1234567.85, 1, It), "1.234.567,9");
        assert_eq!(group(-1234.0, 0, En), "-1,234");
        assert_eq!(group(-1234.0, 0, It), "-1234");
    }

    #[test]
    fn halves_round_up_on_the_decimal_value() {
        assert_eq!(group(1.25, 1, En), "1.3");
        assert_eq!(group(1.25, 1, It), "1,3");
        assert_eq!(group(2.05, 1, En), "2.1");
        assert_eq!(group(9.95, 1, En), "10");
        assert_eq!(group(0.05, 1, It), "0,1");
        assert_eq!(group(2.0, 1, En), "2");
    }

    #[test]
    fn days_must_exist() {
        assert_eq!(iso_day("2026-09-23"), Some((2026, 9, 23)));
        assert_eq!(iso_day("2028-02-29"), Some((2028, 2, 29)));
        assert_eq!(iso_day("2026-02-29"), None);
        assert_eq!(iso_day("2026-02-31"), None);
        assert_eq!(iso_day("2026-13-01"), None);
        assert_eq!(iso_day("2026-9-23"), None);
        assert_eq!(iso_day("2026-09-23T00:00"), None);
    }

    #[test]
    fn instants_read_like_javascript_dates() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2026-10-07T14:32:00+02:00"), Some(1_791_376_320_000));
        assert_eq!(parse_rfc3339("2026-10-07T12:32:00.1239Z"), Some(1_791_376_320_123));
        // As V8 reads them: days up to 31 roll into the next month, 24:00 is the next midnight.
        assert_eq!(parse_rfc3339("2026-02-31T10:00:00Z"), Some(1_772_532_000_000));
        assert_eq!(parse_rfc3339("2026-04-31T00:00:00Z"), Some(1_777_593_600_000));
        assert_eq!(parse_rfc3339("2026-10-07T24:00:00Z"), Some(1_791_417_600_000));
        assert_eq!(parse_rfc3339("2026-10-07T24:00:00.000Z"), Some(1_791_417_600_000));
        assert_eq!(parse_rfc3339("2026-10-07T12:00:00+23:59"), Some(1_791_288_060_000));
        for bad in [
            "2026-02-32T00:00:00Z",
            "2026-10-07T24:00:01Z",
            "2026-10-07T12:00:00+24:00",
            "2026-10-07T12:00:00+23:60",
            "2026-13-01T00:00:00Z",
            "2026-00-10T00:00:00Z",
            "2026-10-00T00:00:00Z",
            "2026-10-07T12:60:00Z",
            "2026-10-07T12:00:60Z",
        ] {
            assert_eq!(parse_rfc3339(bad), None, "{bad}");
        }
        assert_eq!(utc_day(1_791_376_320_000), "2026-10-07");
    }

    #[test]
    fn a_year_back_rolls_like_set_utc_month() {
        let leap = parse_rfc3339("2028-02-29T10:00:00Z").unwrap();
        assert_eq!(utc_day(months_earlier(leap, 12)), "2027-03-01");
        let plain = parse_rfc3339("2026-10-07T23:59:59Z").unwrap();
        assert_eq!(months_earlier(plain, 12), parse_rfc3339("2025-10-07T23:59:59Z").unwrap());
    }

    #[test]
    fn dates_print_like_intl() {
        assert_eq!(date_medium("2026-09-23", En), "23 Sept 2026");
        assert_eq!(date_medium("2026-09-23", It), "23 set 2026");
        assert_eq!(date_medium("2026-01-01", En), "1 Jan 2026");
        assert_eq!(date_medium("0999-05-03", It), "3 mag 999");
        assert_eq!(date_long("2028-10-09", En), "9 October 2028");
        assert_eq!(date_long("2028-10-09", It), "9 ottobre 2028");
        assert_eq!(date_long("2026-02-31", En), "2026-02-31");
        assert_eq!(date_medium("soon", It), "soon");
    }
}
