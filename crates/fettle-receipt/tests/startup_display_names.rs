//! Startup entries Windows and drivers register under an internal name show under the product's
//! name, in the receipt's language. The report keeps the name as Windows stored it.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn startup_labels(locale: Locale) -> Vec<String> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.4", "collected_at": "2026-10-15T10:00:00Z", "elevated": false,
        "facts": {
            "startup": { "check": { "status": "ok" }, "data": { "items": [
                { "name": "SecurityHealth", "enabled": true },
                { "name": "RtkAudUService", "enabled": true },
                { "name": "Steam", "enabled": true }
            ] } }
        }
    }))
    .expect("report");
    let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&model).expect("model");
    let startup = json["sections"].as_array().expect("sections").iter().find(|s| s["id"] == "startup").expect("startup section");
    startup["details"][0]["rows"].as_array().expect("rows").iter().map(|r| r["label"].as_str().unwrap_or_default().to_string()).collect()
}

#[test]
fn windows_security_and_realtek_audio_show_by_their_names() {
    assert_eq!(startup_labels(Locale::En), ["Realtek Audio", "Steam", "Windows Security"]);
    assert_eq!(startup_labels(Locale::It), ["Realtek Audio", "Sicurezza di Windows", "Steam"]);
}
