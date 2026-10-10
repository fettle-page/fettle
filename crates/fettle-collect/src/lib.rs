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
