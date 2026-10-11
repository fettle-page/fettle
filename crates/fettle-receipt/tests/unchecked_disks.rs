//! A receipt where everything checked is fine but the disks' health is unknown: it says so in its top
//! line, keeps the fine look (tone and "Checked" stamp: what was checked is fine), and lists the Deep check.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

#[test]
fn a_fine_receipt_with_unchecked_disks_keeps_the_fine_look_and_names_the_deep_check() {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.0", "collected_at": "2026-10-11T10:00:00Z", "elevated": false,
        "facts": {
            "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": "ssd", "model": "Crucial MX500" }] } },
            "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 11 Pro", "edition_id": "Professional", "build": 26200, "display_version": "25H2" } },
            "memory": { "check": { "status": "ok" }, "data": { "modules": [{ "capacity_bytes": 17179869184u64 }] } },
            "startup": { "check": { "status": "ok" }, "data": { "items": [] } },
            "graphics": { "check": { "status": "ok" }, "data": { "adapters": [{ "name": "GPU", "driver_date": "2026-09-01" }] } }
        }
    }))
    .expect("report");
    let (_, en) = receipt_for(&report, Locale::En, Zone::Utc).expect("receipt");
    assert_eq!(en.tone, "fine");
    assert_eq!(en.labels.stamp, "Checked");
    assert_eq!(en.summary, "Fine so far, but Fettle couldn't read the disks' health.");
    assert_eq!(en.actions, ["Run a Deep check to read the disks' health."]);
    let (_, it) = receipt_for(&report, Locale::It, Zone::Utc).expect("receipt");
    assert_eq!(it.summary, "Tutto a posto finora, ma Fettle non è riuscito a leggere lo stato dei dischi.");
    assert_eq!(it.actions, ["Avvia un Controllo approfondito per leggere lo stato dei dischi."]);
}

#[test]
fn a_fine_receipt_with_a_section_unchecked_keeps_the_fine_look_and_says_so() {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.3", "collected_at": "2026-10-11T10:00:00Z", "elevated": true,
        "facts": {
            "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "nvme", "model": "X", "nvme": { "percentage_used": 3 } }] } },
            "windows": { "check": { "status": "failed" } },
            "memory": { "check": { "status": "ok" }, "data": { "modules": [{ "capacity_bytes": 17179869184u64 }] } },
            "startup": { "check": { "status": "ok" }, "data": { "items": [] } },
            "graphics": { "check": { "status": "ok" }, "data": { "adapters": [{ "name": "GPU", "driver_date": "2026-09-01" }] } }
        }
    }))
    .expect("report");
    let (_, en) = receipt_for(&report, Locale::En, Zone::Utc).expect("receipt");
    assert_eq!((en.tone.as_str(), en.labels.stamp.as_str()), ("fine", "Checked"));
    assert_eq!(en.summary, "Fine so far, but Fettle couldn't check everything.");
    let (_, it) = receipt_for(&report, Locale::It, Zone::Utc).expect("receipt");
    assert_eq!(it.summary, "Tutto a posto finora, ma Fettle non è riuscito a controllare tutto.");
}
