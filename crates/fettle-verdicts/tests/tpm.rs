//! Windows 11 readiness on Windows 10: what the TPM line says.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Receipt, evaluate, fmt};
use serde_json::{Value, json};

fn tpm_key(tpm: Value) -> String {
    let facts: Facts = serde_json::from_value(json!({ "windows": { "check": { "status": "ok" }, "data": {
        "product": "Windows 10 Pro", "edition_id": "Professional", "build": 19045, "display_version": "22H2", "tpm": tpm
    } } }))
    .expect("facts");
    let r: Receipt = evaluate(&facts, fmt::parse_rfc3339("2026-10-11T10:00:00Z").expect("now"), true);
    r.sections.iter().flat_map(|s| &s.lines).find(|l| l.id == "windows-tpm").expect("tpm line").headline.key.clone()
}

#[test]
fn a_tpm_with_no_version_is_not_called_old() {
    // A TPM 2.0 that is off or not ready in the firmware answers with an empty version.
    assert_eq!(tpm_key(json!({ "present": true, "spec_version": "" })), "windows.tpm_unknown");
    assert_eq!(tpm_key(json!({ "present": true })), "windows.tpm_unknown");
    assert_eq!(tpm_key(json!({ "present": true, "spec_version": "1.2, 2, 3" })), "windows.tpm_old");
    assert_eq!(tpm_key(json!({ "present": true, "spec_version": "2.0, 0, 1.59" })), "windows.tpm_ready");
}
