//! A hard drive's details never show a "Wear" row: the SMART value read as wear means something
//! else on some hard drives. An SSD's still does.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn wear_rows(media: &str) -> Vec<String> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.1", "collected_at": "2026-10-11T10:00:00Z", "elevated": true,
        "facts": {
            "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": media, "model": "Disk", "smart": { "life_used_pct": 100, "reallocated_sectors": 0 } }] } }
        }
    }))
    .expect("report");
    let (_, en) = receipt_for(&report, Locale::En, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&en).expect("model");
    json["sections"][0]["details"][0]["rows"].as_array().expect("rows").iter().filter(|r| r["label"] == "Wear").map(|r| r["value"].as_str().unwrap_or_default().to_string()).collect()
}

#[test]
fn a_hard_drive_shows_no_wear_row() {
    assert_eq!(wear_rows("hdd"), Vec::<String>::new());
    assert_eq!(wear_rows("ssd"), ["100%"]);
}

#[test]
fn a_disk_shows_its_unrecoverable_sectors() {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.3", "collected_at": "2026-10-11T10:00:00Z", "elevated": true,
        "facts": {
            "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": "hdd", "model": "Disk", "smart": { "offline_uncorrectable": 24 } }] } }
        }
    }))
    .expect("report");
    for (locale, label) in [(Locale::En, "Unrecoverable sectors"), (Locale::It, "Settori irrecuperabili")] {
        let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
        let json = serde_json::to_value(&model).expect("model");
        let rows = json["sections"][0]["details"][0]["rows"].as_array().expect("rows").clone();
        assert!(rows.iter().any(|r| r["label"] == label && r["value"] == "24"), "{rows:?}");
    }
}
