//! The Rust rules against the goldens (crates/fettle-receipt/tests/parity): every case's verdicts
//! must be what they say. They were the TypeScript engine's until it was retired (2026-10-10); now
//! `npm run parity:goldens` regenerates them from Rust.

use fettle_facts::Report;
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

fn cases() -> Vec<(String, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fettle-receipt/tests/parity/cases");
    let mut files: Vec<_> = std::fs::read_dir(&dir).expect("parity cases").map(|e| e.unwrap().path()).collect();
    files.sort();
    files.into_iter().map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap())).collect()
}

#[test]
fn verdicts_match_the_goldens_on_every_case() {
    let cases = cases();
    assert!(cases.len() > 300, "the corpus is missing");
    let mut failures = Vec::new();
    for (name, case) in &cases {
        let report: Report = serde_json::from_value(case["report"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let now = fettle_verdicts::fmt::parse_rfc3339(&report.collected_at).unwrap_or_else(|| panic!("{name}: collected_at"));
        let receipt = fettle_verdicts::evaluate(&report.facts, now, report.elevated);
        if let Some(diff) = first_difference("", &serde_json::to_value(&receipt).unwrap(), &case["receipt"]) {
            failures.push(format!("{name} {diff}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), cases.len(), failures.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn every_golden_records_this_engine_version() {
    // Shared receipts record the engine that judged them; a version bump means regenerating goldens.
    for (name, case) in cases() {
        assert_eq!(case["receipt"]["engineVersion"], fettle_verdicts::ENGINE_VERSION, "{name}");
    }
}
