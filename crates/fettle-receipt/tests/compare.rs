//! Two checks of the same PC side by side: only what changed, each change marked better, worse or
//! neither. The verdict lines come from the real engine (`receipt_for`), so they match the receipts.

use fettle_facts::Report;
use fettle_receipt::{Change, CompareRow, Comparison, Zone, compare, compare_markdown};
use fettle_verdicts::{Locale, SectionId};
use serde_json::{Value, json};

/// A made-up desktop: one NVMe SSD, two memory sticks, `startup` startup apps, one graphics card.
fn base(collected_at: &str, startup: usize) -> Value {
    let items: Vec<Value> = (0..startup).map(|i| json!({ "name": format!("Example App {i}"), "location": "run_key", "enabled": true })).collect();
    let stick = |locator: &str| json!({ "capacity_bytes": 17179869184u64, "speed_mts": 6000, "configured_speed_mts": 6000, "kind": "ddr5", "form_factor": "dimm", "bank_label": "P0 CHANNEL A", "device_locator": locator });
    json!({
        "schema_version": 1, "app_version": "1.1.0", "collected_at": collected_at, "elevated": true,
        "facts": {
            "system": { "check": { "status": "ok" }, "data": { "cpu_name": "Example CPU 1000", "board": "Example Boards Ltd. EX-B650 TEST", "bios_version": "TEST-1.00", "chassis": "desktop" } },
            "storage": { "check": { "status": "ok" }, "data": {
                "disks": [{ "model": "Example SSD 1TB", "bus": "nvme", "media": "ssd", "size_bytes": 1000204886016u64,
                    "nvme": { "percentage_used": 3, "temperature_c": 41, "power_on_hours": 1200, "media_errors": 0, "critical_warning": 0, "available_spare_pct": 100, "available_spare_threshold_pct": 10 } }],
                "volumes": [{ "letter": "C:", "size_bytes": 1000000000000u64, "free_bytes": 400000000000u64, "bus": "nvme" }]
            } },
            "battery": { "check": { "status": "not_present" } },
            "memory": { "check": { "status": "ok" }, "data": { "modules": [stick("DIMM 0"), stick("DIMM 1")], "slots_total": 4 } },
            "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 11 Pro", "edition_id": "Professional", "build": 26200, "ubr": 7000, "display_version": "25H2", "uptime_seconds": 3600, "last_update_installed": "2026-10-20", "secure_boot": true, "tpm": { "present": true, "spec_version": "2.0, 0, 1.59" } } },
            "startup": { "check": { "status": "ok" }, "data": { "items": items } },
            "graphics": { "check": { "status": "ok" }, "data": { "adapters": [{ "name": "Example Graphics 100", "vram_bytes": 8589934592u64, "driver_version": "31.0.101.1000", "driver_date": "2026-09-01" }] } }
        }
    })
}

fn report(v: Value) -> Report {
    serde_json::from_value(v).expect("report")
}

const BEFORE: &str = "2026-11-01T09:00:00Z";
const AFTER: &str = "2026-11-03T17:30:00Z";

fn group(c: &Comparison, section: SectionId) -> Vec<CompareRow> {
    c.groups.iter().find(|g| g.section == section).map(|g| g.rows.clone()).unwrap_or_default()
}

fn en(a: &Report, b: &Report) -> Comparison {
    compare(a, b, Locale::En, Zone::Utc).expect("comparison")
}

#[test]
fn fewer_startup_apps_is_better() {
    let c = en(&report(base(BEFORE, 41)), &report(base(AFTER, 9)));
    let rows = group(&c, SectionId::Startup);
    let count = rows.iter().find(|r| r.before == "41").unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!((count.after.as_str(), count.change), ("9", Change::Better));
    // The verdict line moved from "keep an eye" to fine too.
    let verdict = rows.iter().find(|r| r.before == "Keep an eye on it").unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!((verdict.after.as_str(), verdict.change), ("Fine", Change::Better));
    assert_eq!(c.nothing_changed, None);
    assert_eq!((c.before.as_str(), c.after.as_str()), ("01/11/2026 09:00 UTC", "03/11/2026 17:30 UTC"));
}

#[test]
fn a_disk_wearing_more_is_worse() {
    let before = base(BEFORE, 5);
    let mut after = base(AFTER, 5);
    after["facts"]["storage"]["data"]["disks"][0]["nvme"]["percentage_used"] = json!(12);
    let c = en(&report(before), &report(after));
    let rows = group(&c, SectionId::Storage);
    let wear = rows.iter().find(|r| r.label.contains("Wear")).unwrap_or_else(|| panic!("{rows:?}"));
    assert!(wear.label.contains("Example SSD 1TB"), "{wear:?}");
    assert_eq!((wear.before.as_str(), wear.after.as_str(), wear.change), ("3%", "12%", Change::Worse));
}

#[test]
fn a_new_act_soon_line_is_worse_and_a_resolved_one_better() {
    let fine = base(BEFORE, 5);
    let mut warned = base(AFTER, 5);
    warned["facts"]["storage"]["data"]["disks"][0]["nvme"]["critical_warning"] = json!(1);
    let c = en(&report(fine.clone()), &report(warned.clone()));
    let rows = group(&c, SectionId::Storage);
    let new = rows.iter().find(|r| r.after == "Act soon").unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!((new.before.as_str(), new.change), ("—", Change::Worse));

    // The other way round in time: the warning was there before and is gone now.
    warned["collected_at"] = json!(BEFORE);
    let mut fixed = fine;
    fixed["collected_at"] = json!(AFTER);
    let c = en(&report(warned), &report(fixed));
    let rows = group(&c, SectionId::Storage);
    let gone = rows.iter().find(|r| r.before == "Act soon").unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!((gone.after.as_str(), gone.change), ("—", Change::Better));
}

#[test]
fn more_free_space_and_memory_are_better() {
    let before = base(BEFORE, 5);
    let mut after = base(AFTER, 5);
    after["facts"]["storage"]["data"]["volumes"][0]["free_bytes"] = json!(600000000000u64);
    let mut sticks = after["facts"]["memory"]["data"]["modules"].as_array().unwrap().clone();
    sticks.extend(sticks.clone());
    after["facts"]["memory"]["data"]["modules"] = json!(sticks);
    let c = en(&report(before), &report(after));
    let free = group(&c, SectionId::Storage).into_iter().find(|r| r.label.contains("C:")).expect("free space row");
    assert_eq!((free.before.as_str(), free.after.as_str(), free.change), ("400 GB of 1 TB (40%)", "600 GB of 1 TB (60%)", Change::Better));
    let memory = group(&c, SectionId::Memory);
    let total = memory.iter().find(|r| r.before == "32 GB").unwrap_or_else(|| panic!("{memory:?}"));
    assert_eq!((total.after.as_str(), total.change), ("64 GB", Change::Better));
    let sticks = memory.iter().find(|r| r.before == "2").unwrap_or_else(|| panic!("{memory:?}"));
    assert_eq!((sticks.after.as_str(), sticks.change), ("4", Change::Neutral));
}

#[test]
fn a_newer_graphics_driver_is_better_and_a_new_windows_build_neutral() {
    let before = base(BEFORE, 5);
    let mut after = base(AFTER, 5);
    after["facts"]["graphics"]["data"]["adapters"][0]["driver_version"] = json!("31.0.101.2000");
    after["facts"]["graphics"]["data"]["adapters"][0]["driver_date"] = json!("2026-10-15");
    after["facts"]["windows"]["data"]["ubr"] = json!(7100);
    after["facts"]["windows"]["data"]["uptime_seconds"] = json!(3 * 86400);
    let c = en(&report(before), &report(after));
    let graphics = group(&c, SectionId::Graphics);
    let date = graphics.iter().find(|r| r.after == "15 Oct 2026").unwrap_or_else(|| panic!("{graphics:?}"));
    assert_eq!((date.before.as_str(), date.change), ("1 Sept 2026", Change::Better));
    assert!(graphics.iter().any(|r| r.after == "31.0.101.2000" && r.label.contains("Example Graphics 100")), "{graphics:?}");
    let windows = group(&c, SectionId::Windows);
    // The build changed; time since restart is never compared.
    assert_eq!(windows.len(), 1, "{windows:?}");
    assert_eq!((windows[0].before.as_str(), windows[0].after.as_str(), windows[0].change), ("26200.7000", "26200.7100", Change::Neutral));
}

#[test]
fn identical_checks_say_nothing_changed() {
    let c = en(&report(base(BEFORE, 5)), &report(base(AFTER, 5)));
    assert_eq!(c.groups, vec![]);
    assert_eq!(c.nothing_changed.as_deref(), Some("Nothing changed between these two checks."));
    assert_eq!(c.different_pc, None);
}

#[test]
fn a_different_board_looks_like_a_different_pc() {
    let before = base(BEFORE, 5);
    let mut after = base(AFTER, 5);
    after["facts"]["system"]["data"]["board"] = json!("Example Boards Ltd. EX-Z790 OTHER");
    let c = en(&report(before), &report(after));
    assert_eq!(c.different_pc.as_deref(), Some("These two checks look like they're from different PCs."));
}

#[test]
fn the_order_of_the_arguments_does_not_matter() {
    let a = report(base(BEFORE, 41));
    let b = report(base(AFTER, 9));
    assert_eq!(en(&a, &b), en(&b, &a));
}

#[test]
fn a_bad_scan_time_is_refused() {
    let mut bad = base(BEFORE, 5);
    bad["collected_at"] = json!("garbage");
    assert!(compare(&report(bad), &report(base(AFTER, 5)), Locale::En, Zone::Utc).is_err());
}

#[test]
fn italian_uses_italian_words() {
    let c = compare(&report(base(BEFORE, 41)), &report(base(AFTER, 9)), Locale::It, Zone::Utc).expect("comparison");
    assert_eq!((c.labels.title.as_str(), c.labels.before.as_str(), c.labels.after.as_str()), ("Prima e dopo", "Prima", "Dopo"));
    let startup = c.groups.iter().find(|g| g.section == SectionId::Startup).expect("startup");
    assert_eq!(startup.title, "Avvio");
    assert!(startup.rows.iter().any(|r| r.before == "Da tenere d'occhio" && r.after == "A posto"), "{:?}", startup.rows);
    let same = compare(&report(base(BEFORE, 5)), &report(base(AFTER, 5)), Locale::It, Zone::Utc).expect("comparison");
    assert_eq!(same.nothing_changed.as_deref(), Some("Non è cambiato niente tra questi due controlli."));
}

#[test]
fn changes_serialize_in_lowercase() {
    assert_eq!(serde_json::to_value([Change::Better, Change::Worse, Change::Neutral]).unwrap(), json!(["better", "worse", "neutral"]));
}

#[test]
fn the_text_copy_lists_each_change() {
    let c = en(&report(base(BEFORE, 41)), &report(base(AFTER, 9)));
    let md = compare_markdown(&c);
    assert!(md.starts_with("**Fettle** · Before and after"), "{md}");
    assert!(md.contains("01/11/2026 09:00 UTC") && md.contains("03/11/2026 17:30 UTC"), "{md}");
    assert!(md.contains("41 → 9 (better)"), "{md}");
    let same = compare_markdown(&en(&report(base(BEFORE, 5)), &report(base(AFTER, 5))));
    assert!(same.contains("Nothing changed between these two checks."), "{same}");
}
