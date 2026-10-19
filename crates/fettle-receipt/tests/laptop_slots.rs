//! Laptop firmware often counts slots that aren't there (an ASUS X541UJ said 4 for soldered memory
//! plus one slot), so on a laptop the receipt says how many are in use, without a total.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::{Value, json};

fn slots(chassis: Option<&str>, modules: Value, locale: Locale) -> Vec<String> {
    let mut report = json!({
        "schema_version": 1, "app_version": "1.0.5", "collected_at": "2026-10-15T10:00:00Z", "elevated": true,
        "facts": { "memory": { "check": { "status": "ok" }, "data": { "modules": modules, "slots_total": 4 } } }
    });
    if let Some(c) = chassis {
        report["facts"]["system"] = json!({ "check": { "status": "ok" }, "data": { "chassis": c } });
    }
    let report: Report = serde_json::from_value(report).expect("report");
    let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&model).expect("model");
    let memory = json["sections"].as_array().expect("sections").iter().find(|s| s["id"] == "memory").expect("memory section").clone();
    // The summary group (Total, Slots, Channels) is the last one.
    let summary = memory["details"].as_array().expect("details").last().expect("summary").clone();
    summary["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|r| r["label"] == "Slots" || r["label"] == "Slot")
        .map(|r| r["value"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn sticks(kind: &str) -> Value {
    let stick = |n: u32| json!({ "kind": kind, "form_factor": "sodimm", "capacity_bytes": 4294967296u64, "device_locator": format!("ChannelA-DIMM{n}") });
    json!([0, 1].map(stick))
}

#[test]
fn a_laptop_shows_the_slots_in_use_without_a_total() {
    assert_eq!(slots(Some("laptop"), sticks("ddr4"), Locale::En), ["2 in use"]);
    assert_eq!(slots(Some("laptop"), sticks("ddr4"), Locale::It), ["2 in uso"]);
}

#[test]
fn other_pcs_keep_the_total() {
    for chassis in [Some("desktop"), Some("unknown"), None] {
        assert_eq!(slots(chassis, sticks("ddr4"), Locale::En), ["2 of 4 used"], "{chassis:?}");
    }
    assert_eq!(slots(Some("desktop"), sticks("ddr4"), Locale::It), ["2 su 4 occupati"]);
}

#[test]
fn soldered_memory_on_a_laptop_still_says_so() {
    assert_eq!(slots(Some("laptop"), sticks("lpddr4"), Locale::En), ["none, soldered to the board"]);
}
