//! The viewer's offset from UTC at an instant, from Windows' own time-zone rules (summer time
//! included), so the receipt shows the scan time where the reader is. The core carries no
//! time-zone database; each platform asks its OS.

/// Minutes east of UTC at `at_ms` (ms since 1970); 0 when Windows can't say.
#[cfg(windows)]
pub fn local_offset_minutes(at_ms: i64) -> i32 {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime};
    // FILETIME counts 100 ns since 1601.
    let ticks = (at_ms + 11_644_473_600_000) as u64 * 10_000;
    let utc_ft = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    let mut local_ft = FILETIME::default();
    unsafe {
        if FileTimeToSystemTime(&utc_ft, &mut utc).is_err() || SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).is_err() || SystemTimeToFileTime(&local, &mut local_ft).is_err() {
            return 0;
        }
    }
    let local_ticks = (u64::from(local_ft.dwHighDateTime) << 32) | u64::from(local_ft.dwLowDateTime);
    ((local_ticks as i64 - ticks as i64) / 600_000_000) as i32
}

#[cfg(not(windows))]
pub fn local_offset_minutes(_at_ms: i64) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_whole_quarter_hours_within_a_day() {
        for at in [0, 1_791_376_320_000, 1_800_000_000_000] {
            let off = local_offset_minutes(at);
            assert!((-14 * 60..=14 * 60).contains(&off), "{off}");
            assert_eq!(off % 15, 0, "{off}");
        }
    }
}
