// Every threshold the verdicts use, named. Starting points, tuned with real data from the spike.

pub const SSD_WEAR_KEEP_AN_EYE: u8 = 80;
pub const SSD_WEAR_ACT_SOON: u8 = 100;
pub const DISK_TEMP_KEEP_AN_EYE_C: i32 = 70;
/// Hard drives are rated to about 60 °C (WD Blue, Seagate Barracuda), so they warn sooner.
pub const HDD_TEMP_KEEP_AN_EYE_C: i32 = 55;
pub const FREE_SPACE_KEEP_AN_EYE: f64 = 0.1;
/// Volumes smaller than this (2 GiB) get no free-space warning: a recovery or system partition that
/// has a drive letter is meant to be nearly full. The system drive is checked whatever its size.
pub const SMALL_VOLUME_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const BATTERY_KEEP_AN_EYE: f64 = 0.8;
pub const BATTERY_ACT_SOON: f64 = 0.6;
pub const STARTUP_ITEMS_KEEP_AN_EYE: usize = 15;
pub const UPTIME_NUDGE_DAYS: u64 = 14;
/// Days before Windows support ends when Fettle starts warning about it.
pub const SUPPORT_END_KEEP_AN_EYE_DAYS: i64 = 60;
pub const DRIVER_AGE_INFO_MONTHS: i32 = 12;

/// The summary shows at most this many actions.
pub const SUMMARY_MAX_ACTIONS: usize = 3;
