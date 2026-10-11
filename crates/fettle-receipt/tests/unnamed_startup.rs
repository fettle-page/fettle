//! A startup item without a readable name shows in the receipt's language. New reports store an
//! empty name for it; reports from 1.0.2 and earlier stored the English words.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn startup_labels(locale: Locale) -> Vec<String> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.3", "collected_at": "2026-10-11T10:00:00Z", "elevated": false,
        "facts": {
            "startup": { "check": { "status": "ok" }, "data": { "items": [
                { "name": "", "enabled": true },
                { "name": "Unnamed startup item", "enabled": true },
                { "name": "Steam", "enabled": true },
                { "enabled": true }
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
fn an_unnamed_startup_item_shows_in_the_receipts_language() {
    assert_eq!(startup_labels(Locale::En), ["Steam", "Unnamed startup item", "Unnamed startup item"]);
    assert_eq!(startup_labels(Locale::It), ["App all'avvio senza nome", "App all'avvio senza nome", "Steam"]);
}
