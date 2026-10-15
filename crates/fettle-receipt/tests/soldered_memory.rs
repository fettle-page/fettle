//! Memory soldered to the board (LPDDR) has no slots to count, and its channels aren't the
//! sticks' channels: four LPDDR5 channels are as wide as an ordinary desktop's two.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::{Value, json};

fn memory_rows(modules: Value, slots_total: u32, locale: Locale) -> Vec<(String, String)> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.5", "collected_at": "2026-10-15T10:00:00Z", "elevated": true,
        "facts": { "memory": { "check": { "status": "ok" }, "data": { "modules": modules, "slots_total": slots_total } } }
    }))
    .expect("report");
    let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&model).expect("model");
    let memory = json["sections"].as_array().expect("sections").iter().find(|s| s["id"] == "memory").expect("memory section").clone();
    memory["details"]
        .as_array()
        .expect("details")
        .iter()
        .flat_map(|g| g["rows"].as_array().cloned().unwrap_or_default())
        .map(|r| (r["label"].as_str().unwrap_or_default().to_string(), r["value"].as_str().unwrap_or_default().to_string()))
        .collect()
}

fn values<'a>(rows: &'a [(String, String)], label: &str) -> Vec<&'a str> {
    rows.iter().filter(|(l, _)| l == label).map(|(_, v)| v.as_str()).collect()
}

fn chips(kind: &str) -> Value {
    let chip = |c: char| json!({ "kind": kind, "capacity_bytes": 4294967296u64, "speed_mts": 6400, "configured_speed_mts": 4266, "bank_label": "BANK 0", "device_locator": format!("Controller0-Channel{c}-DIMM0") });
    json!(['A', 'B', 'C', 'D'].map(chip))
}

#[test]
fn soldered_memory_says_so_instead_of_counting_slots_and_channels() {
    let en = memory_rows(chips("lpddr5"), 4, Locale::En);
    assert_eq!(values(&en, "Slots"), ["none, soldered to the board"]);
    assert_eq!(values(&en, "Slot"), ["Channel A", "Channel B", "Channel C", "Channel D"]);
    assert_eq!(values(&en, "Channels"), Vec::<&str>::new());
    assert_eq!(values(&en, "Speed"), ["4266 MT/s (chips rated 6400)"; 4]);

    let it = memory_rows(chips("lpddr5"), 4, Locale::It);
    // In Italian both the chips' row and the summary's are "Slot".
    assert_eq!(values(&it, "Slot"), ["Canale A", "Canale B", "Canale C", "Canale D", "nessuno, saldata sulla scheda madre"]);
    assert_eq!(values(&it, "Canali"), Vec::<&str>::new());
    assert_eq!(values(&it, "Velocità"), ["4266 MT/s (chip da 6400)"; 4]);
}

fn unchanneled(kind: &str) -> Value {
    let chip = |n: u32| json!({ "kind": kind, "capacity_bytes": 4294967296u64, "speed_mts": 6400, "bank_label": format!("BANK {n}"), "device_locator": format!("DIMM {n}") });
    json!([0, 1].map(chip))
}

#[test]
fn a_soldered_chip_without_a_channel_shows_no_slot() {
    // The firmware's "DIMM 0" is its numbering, not a slot, with or without a channel.
    let en = memory_rows(unchanneled("lpddr5"), 2, Locale::En);
    assert_eq!(values(&en, "Slot"), Vec::<&str>::new());
    assert_eq!(values(&en, "Size"), ["4 GB", "4 GB"]);
    let it = memory_rows(unchanneled("lpddr5"), 2, Locale::It);
    assert_eq!(values(&it, "Slot"), ["nessuno, saldata sulla scheda madre"]);
}

#[test]
fn a_stick_without_a_channel_keeps_the_firmware_slot_name() {
    let en = memory_rows(unchanneled("ddr4"), 2, Locale::En);
    assert_eq!(values(&en, "Slot"), ["DIMM 0", "DIMM 1"]);
}

#[test]
fn memory_sticks_keep_their_slots_and_channels() {
    let en = memory_rows(chips("ddr4"), 4, Locale::En);
    assert_eq!(values(&en, "Slots"), ["4 of 4 used"]);
    assert_eq!(values(&en, "Slot"), ["Channel A · DIMM 0", "Channel B · DIMM 0", "Channel C · DIMM 0", "Channel D · DIMM 0"]);
    assert_eq!(values(&en, "Channels"), ["quad channel"]);
    assert_eq!(values(&en, "Speed"), ["4266 MT/s (max 6400)"; 4]);
    let it = memory_rows(chips("ddr4"), 4, Locale::It);
    assert_eq!(values(&it, "Velocità"), ["4266 MT/s (massima 6400)"; 4]);
}
