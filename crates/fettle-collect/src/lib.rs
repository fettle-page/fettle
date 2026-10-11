//! Scans this PC and builds a Fettle report.
//!
//! Sources (Windows only) return raw JSON in the same shape as a spike dump entry, all in parallel.
//! The mapping turns that raw data into typed facts on any OS, and the privacy pass runs last.

pub mod map;
pub mod names;
pub mod private;
pub mod runner;
#[cfg(windows)]
pub mod sources;

pub use runner::{Raw, Source, SourceResult, run_all, run_all_with};

use fettle_facts::redact::Redactor;
use fettle_facts::{Report, SCHEMA_VERSION};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How a scan runs.
#[derive(Debug, Clone)]
pub struct Options {
    /// How long any one source may take before its section is marked failed.
    pub timeout: Duration,
    /// The version the report names (the receipt's footer shows it): the app passes its own.
    pub app_version: String,
}

impl Default for Options {
    fn default() -> Self {
        Options { timeout: Duration::from_secs(4), app_version: env!("CARGO_PKG_VERSION").to_string() }
    }
}

/// Scans this PC: every source at once, then the mapping and the privacy pass.
#[cfg(windows)]
pub fn collect(opts: &Options) -> Report {
    collect_with(opts, |_, _| {})
}

/// `collect`, reporting each source as it settles (see `run_all_with`). The callback sees raw
/// results, which are not redacted: pass on names and timings only.
#[cfg(windows)]
pub fn collect_with(opts: &Options, on_done: impl Fn(&str, &SourceResult)) -> Report {
    let elevated = sources::is_elevated();
    let collected_at = SystemTime::now();
    let raw = run_all_with(sources::sources(elevated), opts.timeout, on_done);
    build_report(&raw, collected_at, elevated, &names::redactor(), &opts.app_version)
}

/// The privacy pass again, with the names of the account running this: a Deep check's elevated scan
/// was cleaned with the administrator's names, which may not be the user's. Running it twice
/// changes nothing already hidden.
#[cfg(windows)]
pub fn redact_for_this_user(report: &mut Report) {
    names::redactor().report(report);
    fettle_facts::fit::fit_limits(report);
}

/// The finished report: the envelope, the mapped facts, then the privacy pass over all of it.
pub fn build_report(raw: &Raw, collected_at: SystemTime, elevated: bool, redactor: &Redactor, app_version: &str) -> Report {
    let mut report = Report {
        schema_version: SCHEMA_VERSION,
        app_version: app_version.to_string(),
        engine_version: None,
        collected_at: rfc3339(collected_at),
        elevated,
        facts: map::facts(raw, collected_at),
    };
    redactor.report(&mut report);
    // The share server refuses text over the schema's limits.
    fettle_facts::fit::fit_limits(&mut report);
    report
}

/// "2026-10-06T13:24:07Z", to the second.
fn rfc3339(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (y, m, d) = map::civil_from_days(secs.div_euclid(86400));
    let s = secs.rem_euclid(86400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

#[cfg(test)]
mod options_tests {
    use super::Options;

    #[test]
    fn a_scan_names_this_crate_s_version_unless_the_caller_gives_its_own() {
        assert_eq!(Options::default().app_version, env!("CARGO_PKG_VERSION"));
    }
}

#[cfg(test)]
mod limits_tests {
    use super::*;

    // The share server refuses text over its schema's limits, so a scan never produces any.
    #[test]
    fn a_scan_s_text_fits_the_report_limits() {
        let long = "🙂".repeat(200);
        let raw: Raw = serde_json::from_value(serde_json::json!({
            "physical_disks": { "ok": true, "ms": 1, "error": null, "data": [{ "DeviceId": "0", "FriendlyName": long, "BusType": 17, "MediaType": 4, "Size": 1 }] },
        }))
        .unwrap();
        let report = build_report(&raw, UNIX_EPOCH, false, &Redactor::default(), "1.0.1");
        let model = report.facts.storage.unwrap().data.unwrap().disks[0].model.clone().unwrap();
        assert!(model.encode_utf16().count() <= fettle_facts::limits::NAME);
    }
}

#[cfg(all(test, windows))]
mod end_to_end {
    use super::*;
    use fettle_verdicts::{Level, evaluate};

    /// A drive's NVMe health log (page 0x02): warning bits, 40 °C, spare 100% of a 10% floor, 3% used.
    fn log(warning: u8) -> [u8; 512] {
        let mut log = [0u8; 512];
        log[0] = warning;
        log[1..3].copy_from_slice(&313u16.to_le_bytes());
        (log[3], log[4], log[5]) = (100, 10, 3);
        log
    }

    /// The verdict lines for one NVMe drive whose log has these warning bits, through the real parser,
    /// the mapping and the verdict engine.
    fn verdict_lines(warning: u8) -> Vec<(String, Level)> {
        let health = serde_json::to_value(sources::nvme::parse_nvme_health_log(&log(warning))).unwrap();
        let raw: Raw = serde_json::from_value(serde_json::json!({
            "physical_disks": { "ok": true, "ms": 1, "error": null, "data": [{ "DeviceId": "0", "FriendlyName": "Test NVMe", "BusType": 17, "MediaType": 4, "Size": 1 }] },
            "nvme_health": { "ok": true, "ms": 1, "error": null, "data": [{ "drive": 0, "attempts": [{ "access": "none", "ok": true, "health": health }] }] },
        }))
        .unwrap();
        let report = build_report(&raw, UNIX_EPOCH, false, &Redactor::default(), "1.0.1");
        let receipt = evaluate(&report.facts, 0, false);
        receipt.sections.iter().flat_map(|s| &s.lines).map(|l| (l.headline.key.clone(), l.level)).collect()
    }

    // The drive's own "reliability degraded" flag must reach the receipt as act-soon.
    #[test]
    fn a_drive_reporting_a_problem_gets_act_soon() {
        assert!(verdict_lines(0x04).contains(&("storage.nvme_critical".into(), Level::ActSoon)));
        assert!(verdict_lines(0x08).contains(&("storage.nvme_critical".into(), Level::ActSoon)), "read-only media");
    }

    #[test]
    fn a_healthy_drive_or_one_that_is_only_hot_is_not_called_failing() {
        for warning in [0x00, 0x02] {
            assert!(!verdict_lines(warning).iter().any(|(k, _)| k == "storage.nvme_critical"), "{warning:#04x}");
        }
    }
}
