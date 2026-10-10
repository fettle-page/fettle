//! A receipt where everything checked is fine but the disks need a Deep check: it says so in its top
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
            "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 11 Pro", "edition_id": "Professional", "build": 26200, "display_version": "25H2" } }
        }
    }))
    .expect("report");
    let (_, en) = receipt_for(&report, Locale::En, Zone::Utc).expect("receipt");
    assert_eq!(en.tone, "fine");
    assert_eq!(en.labels.stamp, "Checked");
    assert_eq!(en.summary, "Fine so far, but the disks still need a Deep check.");
    assert_eq!(en.actions, ["Run a Deep check to read the disks' health."]);
    let (_, it) = receipt_for(&report, Locale::It, Zone::Utc).expect("receipt");
    assert_eq!(it.summary, "Tutto a posto finora, ma per i dischi serve un Controllo approfondito.");
    assert_eq!(it.actions, ["Avvia un Controllo approfondito per leggere lo stato dei dischi."]);
}
