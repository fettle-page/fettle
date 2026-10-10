//! The Rust receipt against the goldens in `parity/cases`: the model in both languages (and its
//! markdown) must be what they say. They were the TypeScript engine's until it was retired
//! (2026-10-10); now `npm run parity:goldens` regenerates them from Rust, for a rule or copy change.

use fettle_facts::Report;
use fettle_receipt::{Zone, build};
use fettle_verdicts::{Locale, evaluate, fmt};
use serde_json::Value;
use std::path::PathBuf;

/// The first place two JSON values differ, as a JSON pointer with both values.
fn first_difference(path: &str, a: &Value, b: &Value) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().find_map(|k| first_difference(&format!("{path}/{k}"), x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null)))
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x.iter().zip(y).enumerate().find_map(|(i, (p, q))| first_difference(&format!("{path}/{i}"), p, q)),
        _ if a == b => None,
        _ => Some(format!("{path}: rust {a} ≠ ts {b}")),
    }
}

pub fn cases() -> Vec<(String, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/cases");
    let mut files: Vec<_> = std::fs::read_dir(&dir).expect("parity cases").map(|e| e.unwrap().path()).collect();
    files.sort();
    files.into_iter().map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap())).collect()
}

/// Runs `check` on every case and lists the ones that differ.
fn each_case(check: impl Fn(&str, &Report, &Value) -> Option<String>) {
    let cases = cases();
    assert!(cases.len() > 300, "the corpus is missing");
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(name, case)| {
            let report: Report = serde_json::from_value(case["report"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
            check(name, &report, case).map(|d| format!("{name} {d}"))
        })
        .collect();
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), cases.len(), failures.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
}

fn receipt(report: &Report) -> fettle_verdicts::Receipt {
    evaluate(&report.facts, fmt::parse_rfc3339(&report.collected_at).expect("collected_at"), report.elevated)
}

#[test]
fn the_english_model_matches_the_goldens() {
    each_case(|_, report, case| first_difference("", &serde_json::to_value(build(report, &receipt(report), Locale::En, Zone::Utc)).unwrap(), &case["en"]));
}

#[test]
fn the_italian_model_matches_the_goldens() {
    each_case(|_, report, case| first_difference("", &serde_json::to_value(build(report, &receipt(report), Locale::It, Zone::Utc)).unwrap(), &case["it"]));
}

#[test]
fn the_markdown_matches_the_goldens() {
    each_case(|_, report, case| {
        let receipt = receipt(report);
        let en = fettle_receipt::markdown(&build(report, &receipt, Locale::En, Zone::Utc));
        let it = fettle_receipt::markdown(&build(report, &receipt, Locale::It, Zone::Utc));
        first_difference("/markdownEn", &Value::String(en), &case["markdownEn"]).or_else(|| first_difference("/markdownIt", &Value::String(it), &case["markdownIt"]))
    });
}

/// `npm run parity:goldens` sets FETTLE_REGEN_GOLDENS=1 and runs this: each case's expected parts are
/// rewritten from Rust, but only where they changed, so an unchanged file stays byte for byte (the
/// old goldens' key order included). A case with a report only (new or changed) is filled in.
#[test]
fn regenerate_goldens() {
    if std::env::var_os("FETTLE_REGEN_GOLDENS").is_none() {
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/cases");
    let mut rewritten = Vec::new();
    for (file, case) in cases() {
        let report: Report = serde_json::from_value(case["report"].clone()).unwrap_or_else(|e| panic!("{file}: {e}"));
        let receipt = receipt(&report);
        let en = build(&report, &receipt, Locale::En, Zone::Utc);
        let it = build(&report, &receipt, Locale::It, Zone::Utc);
        let expected = [
            ("receipt", serde_json::to_value(&receipt).unwrap()),
            ("markdownEn", Value::String(fettle_receipt::markdown(&en))),
            ("markdownIt", Value::String(fettle_receipt::markdown(&it))),
            ("en", serde_json::to_value(&en).unwrap()),
            ("it", serde_json::to_value(&it).unwrap()),
        ];
        if expected.iter().all(|(key, value)| &case[*key] == value) {
            continue;
        }
        let part = |key: &str, value: &Value| format!(",\"{key}\":{}", serde_json::to_string(value).unwrap());
        let mut text = format!("{{\"name\":{}", serde_json::to_string(&case["name"]).unwrap());
        text.push_str(&part("report", &case["report"]));
        for key in ["receipt", "en", "it", "markdownEn", "markdownIt"] {
            text.push_str(&part(key, &expected.iter().find(|(k, _)| *k == key).unwrap().1));
        }
        text.push_str("}\n");
        std::fs::write(dir.join(&file), text).unwrap();
        rewritten.push(file);
    }
    eprintln!("parity: {} case(s) rewritten{}", rewritten.len(), if rewritten.is_empty() { String::new() } else { format!(": {}", rewritten.join(", ")) });
}

#[test]
fn receipt_for_evaluates_at_the_scan_time() {
    let cases = cases();
    let (_, case) = cases.iter().find(|(name, _)| name.starts_with("dump-")).expect("a dump case");
    let report: Report = serde_json::from_value(case["report"].clone()).unwrap();
    let (receipt, model) = fettle_receipt::receipt_for(&report, Locale::It, Zone::Utc).expect("a receipt");
    assert_eq!(serde_json::to_value(&receipt).unwrap(), case["receipt"]);
    assert_eq!(serde_json::to_value(&model).unwrap(), case["it"]);
}

#[test]
fn receipt_for_reads_scan_times_as_javascript_does() {
    let (_, case) = cases().into_iter().next().unwrap();
    let mut report: Report = serde_json::from_value(case["report"].clone()).unwrap();
    // The schema lets "31 February" through; JavaScript rolls it to 3 March, and so does Rust.
    report.collected_at = "2026-02-31T10:00:00Z".into();
    let (_, model) = fettle_receipt::receipt_for(&report, Locale::En, Zone::Utc).expect("a receipt");
    assert_eq!(model.meta.when, "03/03/2026 10:00 UTC");
    // What JavaScript can't read at all, the engine refuses (TypeScript's evaluate throws).
    report.collected_at = "2026-02-32T10:00:00Z".into();
    assert!(fettle_receipt::receipt_for(&report, Locale::En, Zone::Utc).is_err());
}
