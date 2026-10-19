//! Windows 11 on a processor that isn't on Microsoft's list: one calm note, nothing more.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Level, Line, Locale, Receipt, evaluate, fmt, render};
use serde_json::{Value, json};

fn receipt(product: &str, edition: &str, build: u32, version: &str, cpu_name: Option<&str>, secure_boot: Option<bool>) -> Receipt {
    let mut windows = json!({ "product": product, "edition_id": edition, "build": build, "display_version": version });
    if let Some(on) = secure_boot {
        windows["secure_boot"] = Value::Bool(on);
    }
    let mut report = json!({ "windows": { "check": { "status": "ok" }, "data": windows } });
    if let Some(name) = cpu_name {
        report["system"] = json!({ "check": { "status": "ok" }, "data": { "cpu_name": name } });
    }
    let facts: Facts = serde_json::from_value(report).expect("facts");
    evaluate(&facts, fmt::parse_rfc3339("2026-10-19T10:00:00Z").expect("now"), true)
}

fn win11(cpu_name: Option<&str>) -> Receipt {
    receipt("Windows 11 Home", "Core", 26200, "25H2", cpu_name, None)
}

fn line<'a>(r: &'a Receipt, id: &str) -> Option<&'a Line> {
    r.sections.iter().flat_map(|s| &s.lines).find(|l| l.id == id)
}

const UNSUPPORTED: &str = "Intel(R) Core(TM) i5-7200U CPU @ 2.50GHz";
const SUPPORTED: &str = "Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz";

#[test]
fn windows_11_on_an_unsupported_processor_gets_a_note() {
    let r = win11(Some(UNSUPPORTED));
    let l = line(&r, "windows-cpu").expect("cpu line");
    assert_eq!(l.level, Level::Info);
    assert_eq!(l.headline.key, "windows.cpu_unsupported_w11");
    assert!(l.action.is_none());
    for locale in [Locale::En, Locale::It] {
        // A key with no copy renders as the key itself.
        assert_ne!(render(&l.headline, locale), l.headline.key);
    }
}

#[test]
fn windows_11_says_nothing_when_the_processor_is_supported_or_unknown() {
    assert!(line(&win11(Some(SUPPORTED)), "windows-cpu").is_none());
    assert!(line(&win11(Some("Intel(R) Celeron(R) N4020 CPU @ 1.10GHz")), "windows-cpu").is_none());
    assert!(line(&win11(None), "windows-cpu").is_none());
}

#[test]
fn the_note_does_not_change_the_summary() {
    assert_eq!(win11(Some(UNSUPPORTED)).summary, win11(Some(SUPPORTED)).summary);
}

#[test]
fn the_note_sits_beside_the_secure_boot_note() {
    let r = receipt("Windows 11 Home", "Core", 26200, "25H2", Some(UNSUPPORTED), Some(false));
    assert_eq!(line(&r, "windows-cpu").expect("cpu line").headline.key, "windows.cpu_unsupported_w11");
    assert_eq!(line(&r, "windows-secure-boot").expect("secure boot line").headline.key, "windows.secure_boot_off_w11");
}

#[test]
fn windows_10_keeps_its_readiness_line() {
    let r = receipt("Windows 10 Pro", "Professional", 19045, "22H2", Some(UNSUPPORTED), None);
    assert_eq!(line(&r, "windows-cpu").expect("cpu line").headline.key, "windows.cpu_unsupported");
}
