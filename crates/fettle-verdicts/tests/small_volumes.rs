//! Free space on small volumes: a few-hundred-MB recovery or system partition with a drive letter
//! is meant to be nearly full, so it gets no "almost full" warning. The system drive (C:) is always
//! checked, and a volume whose size isn't known is never treated as small.

use fettle_facts::Facts;
use fettle_verdicts::{Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

const MB: u64 = 1_000_000;
const GB: u64 = 1_000_000_000;

fn judge(volumes: Value) -> Receipt {
    let facts: Facts = serde_json::from_value(json!({
        "storage": { "check": { "status": "ok" }, "data": { "disks": [], "volumes": volumes } },
    }))
    .expect("facts");
    evaluate(&facts, fmt::parse_rfc3339("2026-10-11T10:00:00Z").expect("now"), false)
}

fn low_space(r: &Receipt) -> Vec<String> {
    let lines: Vec<&Line> = r.sections.iter().flat_map(|s| &s.lines).filter(|l| l.headline.key == "storage.low_space").collect();
    lines.iter().map(|l| l.id.clone()).collect()
}

#[test]
fn a_small_partition_with_a_letter_gets_no_almost_full_warning() {
    // A friend's PC: "Drive D:" was a 367 MB partition with 4.2 MB free.
    let r = judge(json!([{ "letter": "D:", "size_bytes": 367 * MB, "free_bytes": 4_200_000 }]));
    assert!(low_space(&r).is_empty(), "{:?}", low_space(&r));
    // Not "needs a little fettling" because of it, and no "free up some space" next step.
    assert_ne!(r.summary.key, "summary.some");
    assert!(r.summary.actions.iter().all(|a| !a.key.contains("space")), "{:?}", r.summary.actions);
}

#[test]
fn a_full_volume_of_normal_size_still_gets_one() {
    let r = judge(json!([{ "letter": "E:", "size_bytes": 500 * GB, "free_bytes": 5 * GB }]));
    assert_eq!(low_space(&r), ["volume-0"]);
}

#[test]
fn the_limit_is_two_gigabytes() {
    let just_under = judge(json!([{ "letter": "D:", "size_bytes": 2 * 1024 * 1024 * 1024u64 - 1, "free_bytes": 1 }]));
    assert!(low_space(&just_under).is_empty());
    let at = judge(json!([{ "letter": "D:", "size_bytes": 2 * 1024 * 1024 * 1024u64, "free_bytes": 1 }]));
    assert_eq!(low_space(&at), ["volume-0"]);
}

#[test]
fn the_system_drive_is_checked_whatever_its_size() {
    for letter in ["C:", "c:", "C"] {
        let r = judge(json!([{ "letter": letter, "size_bytes": 900 * MB, "free_bytes": 10 * MB }]));
        assert_eq!(low_space(&r), ["volume-0"], "{letter}");
    }
}

#[test]
fn a_volume_of_unknown_size_is_never_taken_for_a_small_one() {
    // No size means no percentage to judge, as before; the small-volume rule doesn't change that,
    // and a known-size volume next to it is judged as usual.
    let r = judge(json!([{ "letter": "D:", "free_bytes": 10 * MB }, { "letter": "E:", "size_bytes": 500 * GB, "free_bytes": 5 * GB }]));
    assert_eq!(low_space(&r), ["volume-1"]);
}

#[test]
fn a_volume_on_a_usb_drive_gets_no_almost_full_warning() {
    // A backup drive kept full on purpose: Fettle only judges the drives inside the PC.
    let r = judge(json!([{ "letter": "E:", "size_bytes": 2000 * GB, "free_bytes": 20 * GB, "bus": "usb" }]));
    assert!(low_space(&r).is_empty(), "{:?}", low_space(&r));
    let inside = judge(json!([{ "letter": "E:", "size_bytes": 2000 * GB, "free_bytes": 20 * GB, "bus": "sata" }]));
    assert_eq!(low_space(&inside), ["volume-0"]);
}

#[test]
fn a_big_data_drive_with_plenty_left_is_not_almost_full() {
    // 8 TB with 700 GB free is under 10%, but nowhere near full. The system drive still goes by
    // percentage: Windows needs room there for its updates.
    let big = judge(json!([{ "letter": "D:", "size_bytes": 8000 * GB, "free_bytes": 700 * GB }]));
    assert!(low_space(&big).is_empty(), "{:?}", low_space(&big));
    let tight = judge(json!([{ "letter": "D:", "size_bytes": 8000 * GB, "free_bytes": 40 * GB }]));
    assert_eq!(low_space(&tight), ["volume-0"]);
    let system = judge(json!([{ "letter": "C:", "size_bytes": 8000 * GB, "free_bytes": 700 * GB }]));
    assert_eq!(low_space(&system), ["volume-0"]);
}
