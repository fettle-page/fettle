// Every threshold the verdicts use, named. Starting points, tuned with real data from the spike.

pub const NVME_WEAR_KEEP_AN_EYE: u8 = 80;
pub const NVME_WEAR_ACT_SOON: u8 = 100;
pub const DISK_TEMP_KEEP_AN_EYE_C: i32 = 70;
pub const FREE_SPACE_KEEP_AN_EYE: f64 = 0.1;
pub const BATTERY_KEEP_AN_EYE: f64 = 0.8;
pub const BATTERY_ACT_SOON: f64 = 0.6;
pub const STARTUP_ITEMS_KEEP_AN_EYE: usize = 15;
pub const UPTIME_NUDGE_DAYS: u64 = 14;
pub const DRIVER_AGE_INFO_MONTHS: i32 = 12;

/// The summary shows at most this many actions.
pub const SUMMARY_MAX_ACTIONS: usize = 3;
