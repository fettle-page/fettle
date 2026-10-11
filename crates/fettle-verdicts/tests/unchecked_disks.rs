//! The summary when the disks' health is unknown: not plain "in fine fettle", and the Deep check is a
//! next step where it would help.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

/// Judges `facts`, with the sections a real scan always has (memory, startup apps, graphics) read
/// and fine unless the test sets them: a section that's missing reads as one Fettle couldn't check.
fn judge(mut facts: Value, now: &str, elevated: bool) -> Receipt {
    let rest = json!({
        "memory": ok(json!({ "modules": [{ "capacity_bytes": 17179869184u64, "speed_mts": 3200, "configured_speed_mts": 3200 }] })),
        "startup": ok(json!({ "items": [] })),
        "graphics": ok(json!({ "adapters": [{ "name": "GPU", "driver_date": "2026-09-01" }] })),
    });
    for (k, v) in rest.as_object().unwrap() {
        facts.as_object_mut().unwrap().entry(k.clone()).or_insert(v.clone());
    }
    let facts: Facts = serde_json::from_value(facts).expect("facts");
    evaluate(&facts, fmt::parse_rfc3339(now).expect("now"), elevated)
}
fn lines(r: &Receipt) -> Vec<&Line> {
    r.sections.iter().flat_map(|s| &s.lines).collect()
}
fn find<'a>(r: &'a Receipt, id: &str) -> &'a Line {
    lines(r).into_iter().find(|l| l.id == id).unwrap_or_else(|| panic!("no line {id}: {:?}", lines(r).iter().map(|l| &l.id).collect::<Vec<_>>()))
}
fn ok(data: Value) -> Value {
    json!({ "check": { "status": "ok" }, "data": data })
}
fn windows(product: &str, edition: &str, build: u32, version: &str) -> Value {
    ok(json!({ "product": product, "edition_id": edition, "build": build, "display_version": version }))
}

// 2. Disks that need a Deep check aren't "in fine fettle", and the Deep check is a next step.
fn unchecked_sata_and(extra: Value) -> Value {
    let mut facts = json!({
        "storage": ok(json!({ "disks": [{ "bus": "sata", "media": "ssd", "model": "Crucial MX500" }] })),
        "windows": windows("Windows 11 Pro", "Professional", 26200, "25H2"),
    });
    facts.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    facts
}

#[test]
fn fine_except_unchecked_disks_says_so_and_offers_the_deep_check() {
    let r = judge(unchecked_sata_and(json!({})), "2026-10-11T10:00:00Z", false);
    assert_eq!(find(&r, "disk-0-health").headline.key, "storage.deep_check");
    assert_eq!(r.summary.key, "summary.fine_unchecked");
    assert_eq!(r.summary.actions.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(), ["summary.action.deep_check"]);
}

#[test]
fn the_deep_check_comes_after_the_warnings() {
    let low = json!({ "storage": ok(json!({
        "disks": [{ "bus": "sata", "media": "ssd", "model": "Crucial MX500" }],
        "volumes": [{ "letter": "C:", "size_bytes": 1000, "free_bytes": 20 }],
    })) });
    let r = judge(unchecked_sata_and(low), "2026-10-11T10:00:00Z", false);
    assert_eq!(r.summary.key, "summary.some");
    assert_eq!(r.summary.actions.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(), ["storage.low_space.action", "summary.action.deep_check"]);
}

#[test]
fn a_deep_check_that_ran_is_plain_fine() {
    let r = judge(json!({ "windows": windows("Windows 11 Pro", "Professional", 26200, "25H2"), "storage": ok(json!({ "disks": [{ "bus": "nvme", "model": "X", "nvme": { "percentage_used": 3 } }] })) }), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine");
    assert!(r.summary.actions.is_empty());
}


// "In fine fettle" needs a disk that was found healthy. A disk that shares no health, or a disks
// check that failed, leaves the summary at "fine so far"; a Deep check is offered only where it helps.
fn fine_windows_and(storage: Value) -> Value {
    json!({ "windows": windows("Windows 11 Pro", "Professional", 26200, "25H2"), "storage": storage })
}
fn action_keys(r: &Receipt) -> Vec<&str> {
    r.summary.actions.iter().map(|a| a.key.as_str()).collect()
}

#[test]
fn a_disk_without_health_after_a_deep_check_is_not_plain_fine() {
    let r = judge(fine_windows_and(ok(json!({ "disks": [{ "bus": "nvme", "model": "X" }] }))), "2026-10-11T10:00:00Z", true);
    assert_eq!(find(&r, "disk-0-health").headline.key, "storage.no_health");
    assert_eq!(r.summary.key, "summary.fine_unchecked");
    assert_eq!(action_keys(&r), Vec::<&str>::new());
}

#[test]
fn one_healthy_disk_and_one_without_health_is_not_plain_fine() {
    let disks = json!({ "disks": [{ "bus": "nvme", "model": "X", "nvme": { "percentage_used": 3 } }, { "bus": "nvme", "model": "Y" }] });
    let r = judge(fine_windows_and(ok(disks)), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine_unchecked");
}

#[test]
fn a_failed_disks_check_is_not_plain_fine() {
    let r = judge(fine_windows_and(json!({ "check": { "status": "failed" } })), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine_unchecked");
    assert_eq!(action_keys(&r), Vec::<&str>::new());
}

#[test]
fn a_disks_check_that_needs_admin_offers_the_deep_check() {
    let r = judge(fine_windows_and(json!({ "check": { "status": "needs_admin" } })), "2026-10-11T10:00:00Z", false);
    assert_eq!(r.summary.key, "summary.fine_unchecked");
    assert_eq!(action_keys(&r), ["summary.action.deep_check"]);
}

#[test]
fn no_disks_at_all_is_not_plain_fine() {
    let r = judge(fine_windows_and(ok(json!({ "disks": [] }))), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine_unchecked");
}

// "In fine fettle" also needs the rest checked: Windows' support known and fine, and no section that
// couldn't be read. Otherwise the summary says Fettle couldn't check everything.
fn healthy_disk() -> Value {
    ok(json!({ "disks": [{ "bus": "nvme", "model": "X", "nvme": { "percentage_used": 3 } }] }))
}

#[test]
fn a_section_that_couldnt_be_read_is_not_plain_fine() {
    let failed = json!({ "check": { "status": "failed" } });
    let windows_failed = judge(json!({ "storage": healthy_disk(), "windows": failed }), "2026-10-11T10:00:00Z", true);
    assert_eq!(windows_failed.summary.key, "summary.fine_partial");
    let startup_failed = judge(json!({ "storage": healthy_disk(), "windows": windows("Windows 11 Pro", "Professional", 26200, "25H2"), "startup": failed }), "2026-10-11T10:00:00Z", true);
    assert_eq!(startup_failed.summary.key, "summary.fine_partial");
}

#[test]
fn a_windows_version_fettle_doesnt_know_is_not_plain_fine() {
    let r = judge(json!({ "storage": healthy_disk(), "windows": windows("Windows Server 2025", "ServerStandard", 26100, "24H2") }), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine_partial");
}

#[test]
fn everything_read_and_fine_is_plain_fine() {
    let r = judge(json!({ "storage": healthy_disk(), "windows": windows("Windows 11 Pro", "Professional", 26200, "25H2") }), "2026-10-11T10:00:00Z", true);
    assert_eq!(r.summary.key, "summary.fine");
}
