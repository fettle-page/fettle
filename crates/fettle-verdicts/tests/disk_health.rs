//! Disk health from SMART: sectors the drive couldn't read or repair, and temperatures.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Level, Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

fn judge(facts: Value) -> Receipt {
    let facts: Facts = serde_json::from_value(facts).expect("facts");
    evaluate(&facts, fmt::parse_rfc3339("2026-10-11T10:00:00Z").expect("now"), true)
}
fn lines(r: &Receipt) -> Vec<&Line> {
    r.sections.iter().flat_map(|s| &s.lines).collect()
}
fn ids(r: &Receipt) -> Vec<&str> {
    lines(r).iter().map(|l| l.id.as_str()).collect()
}
fn find<'a>(r: &'a Receipt, id: &str) -> &'a Line {
    lines(r).into_iter().find(|l| l.id == id).unwrap_or_else(|| panic!("no line {id}: {:?}", ids(r)))
}
fn disk(media: &str, smart: Value) -> Value {
    json!({ "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": media, "model": "ST2000DM008", "smart": smart }] } } })
}

// SMART 198 counts sectors the drive found it can't read or repair: a drive on its way out.
#[test]
fn uncorrectable_sectors_mean_act_soon() {
    let r = judge(disk("hdd", json!({ "reallocated_sectors": 0, "pending_sectors": 0, "offline_uncorrectable": 24 })));
    let l = find(&r, "disk-0-uncorrectable");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::ActSoon, "storage.uncorrectable_sectors"));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("storage.pending_sectors.action"));
    assert!(!ids(&r).contains(&"disk-0"), "no healthy line: {:?}", ids(&r));

    let one = judge(disk("hdd", json!({ "offline_uncorrectable": 1 })));
    assert_eq!(find(&one, "disk-0-uncorrectable").headline.key, "storage.uncorrectable_sectors_one");
}

#[test]
fn no_uncorrectable_sectors_is_healthy() {
    let r = judge(disk("hdd", json!({ "reallocated_sectors": 0, "pending_sectors": 0, "offline_uncorrectable": 0 })));
    assert_eq!(find(&r, "disk-0").headline.key, "storage.disk_fine");
}

// Hard drives are rated to about 60 °C, SSDs to 70 °C or more.
#[test]
fn a_hard_drive_runs_hot_sooner_than_an_ssd() {
    let hot = |media: &str, t: i32| lines(&judge(disk(media, json!({ "reallocated_sectors": 0, "temperature_c": t })))).iter().any(|l| l.headline.key == "storage.disk_hot");
    assert!(!hot("hdd", 55));
    assert!(hot("hdd", 56));
    assert!(!hot("ssd", 65));
    assert!(!hot("ssd", 70));
    assert!(hot("ssd", 71));
    // Without a known type, the SSD limit: no false alarm.
    assert!(!hot("unknown", 65));
}
