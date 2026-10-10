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
