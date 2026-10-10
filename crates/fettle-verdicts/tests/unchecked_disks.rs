//! The summary when disks need a Deep check: not plain "in fine fettle", and the Deep check is a next step.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

fn judge(facts: Value, now: &str, elevated: bool) -> Receipt {
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

