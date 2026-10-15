//! The TPM row says only what Windows answered: a TPM with no version reported is "found", not
//! "on", because the verdict can't tell whether it's switched on.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn tpm_row(tpm: serde_json::Value, locale: Locale) -> String {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.5", "collected_at": "2026-10-15T10:00:00Z", "elevated": true,
        "facts": {
            "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 10 Pro", "edition_id": "Professional", "build": 19045, "display_version": "22H2", "tpm": tpm } }
        }
    }))
    .expect("report");
    let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&model).expect("model");
    let windows = json["sections"].as_array().expect("sections").iter().find(|s| s["title"] == "Windows").expect("windows section").clone();
    let rows: Vec<serde_json::Value> = windows["details"].as_array().expect("details").iter().flat_map(|g| g["rows"].as_array().cloned().unwrap_or_default()).collect();
    rows.iter().find(|r| r["label"] == "TPM").map(|r| r["value"].as_str().unwrap_or_default().to_string()).unwrap_or_else(|| panic!("no TPM row: {rows:?}"))
}

#[test]
fn a_tpm_with_no_version_is_found_not_on() {
    assert_eq!(tpm_row(json!({ "present": true }), Locale::En), "found");
    assert_eq!(tpm_row(json!({ "present": true }), Locale::It), "presente");
    assert_eq!(tpm_row(json!({ "present": true, "spec_version": "2.0, 0, 1.59" }), Locale::En), "2.0");
    assert_eq!(tpm_row(json!({ "present": false }), Locale::En), "not found");
}
