//! Raw sources to the finished report: envelope, mapping and the privacy pass together.

use fettle_collect::{Raw, build_report};
use fettle_facts::redact::Redactor;
use fettle_facts::{CheckStatus, Report, SCHEMA_VERSION};
use serde::Deserialize;
use std::time::{Duration, UNIX_EPOCH};

#[derive(Deserialize)]
struct Dump {
    collected_at_unix: u64,
    elevated: bool,
    sources: Raw,
}

fn dump() -> Dump {
    serde_json::from_str(include_str!("../../../spike/dumps/dev-workstation-user-1791321847.json")).unwrap()
}

fn build(raw: &Raw, d: &Dump) -> Report {
    let redactor = Redactor::new(["jdoe"], "DESKTOP-ABC");
    build_report(raw, UNIX_EPOCH + Duration::from_secs(d.collected_at_unix), d.elevated, &redactor, "0.1.0")
}

#[test]
fn the_envelope_is_filled_in() {
    let d = dump();
    let report = build(&d.sources, &d);
    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert_eq!(report.app_version, "0.1.0");
    assert!(!report.elevated);
    // 1791321847 = 2026-10-06T21:24:07Z
    assert_eq!(report.collected_at, "2026-10-06T21:24:07Z");
}

#[test]
fn names_are_hidden_and_startup_names_cleaned() {
    let d = dump();
    let mut raw = d.sources.clone();
    let memory = raw.get_mut("memory_modules").unwrap();
    memory.ok = false;
    memory.data = None;
    memory.error = Some(r"WMI on DESKTOP-ABC as jdoe failed: C:\Users\jdoe\x".into());
    let report = build(&raw, &d);
    let check = report.facts.memory.unwrap().check;
    assert_eq!(check.status, CheckStatus::Failed);
    assert_eq!(check.detail.as_deref(), Some(r"WMI on <pc-name> as <user> failed: C:\Users\<user>\x"));
    let names: Vec<String> = report.facts.startup.unwrap().data.unwrap().items.into_iter().filter_map(|i| i.name).collect();
    assert!(names.iter().any(|n| n == "Microsoft Edge"), "{names:?}");
    assert!(!names.iter().any(|n| n.contains("AutoLaunch_")), "{names:?}");
}

#[test]
fn the_report_round_trips_as_json() {
    let d = dump();
    let report = build(&d.sources, &d);
    let back: Report = serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(back, report);
}

/// The dump with values no PC should send: impossible dates in every date the scan reads.
fn hostile() -> (Dump, Raw) {
    let d = dump();
    let mut raw = d.sources.clone();
    let mut set = |source: &str, key: &str, value: serde_json::Value| {
        let rows = raw.get_mut(source).and_then(|s| s.data.as_mut()).and_then(|v| v.as_array_mut()).unwrap_or_else(|| panic!("no rows in {source}"));
        for row in rows {
            row[key] = value.clone();
        }
    };
    set("bios", "ReleaseDate", "20260000000000.000000+000".into());
    set("video_controllers", "DriverDate", "20261345000000.000000+000".into());
    set("os", "LastBootUpTime", "20261006256100.000000+000".into());
    set("hotfixes", "InstalledOn", "29/9/2026".into());
    (d, raw)
}

const HOSTILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hostile.report.json");

/// Whatever a PC sends, the report it makes passes the schema the share server checks uploads
/// against: packages/report-schema/test/hostile-report.test.ts parses this file. To rewrite it, run
/// this test with FETTLE_REGEN_GOLDENS=1.
#[test]
fn a_report_from_impossible_values_still_fits_the_schema() {
    let (d, raw) = hostile();
    let report = build(&raw, &d);
    let w = report.facts.windows.as_ref().and_then(|s| s.data.as_ref()).unwrap();
    assert_eq!((w.last_update_installed.as_deref(), w.uptime_seconds), (None, None));
    assert_eq!(report.facts.system.as_ref().and_then(|s| s.data.as_ref()).unwrap().bios_date, None);
    assert!(report.facts.graphics.as_ref().and_then(|s| s.data.as_ref()).unwrap().adapters.iter().all(|a| a.driver_date.is_none()));

    let json = serde_json::to_value(&report).unwrap();
    if std::env::var_os("FETTLE_REGEN_GOLDENS").is_some() {
        std::fs::write(HOSTILE, format!("{}\n", serde_json::to_string_pretty(&json).unwrap())).unwrap();
    }
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(HOSTILE).expect("run with FETTLE_REGEN_GOLDENS=1 once")).unwrap();
    assert_eq!(saved, json, "rewrite with FETTLE_REGEN_GOLDENS=1");
}
