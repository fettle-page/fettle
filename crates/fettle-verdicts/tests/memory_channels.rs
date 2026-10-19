//! The single-channel tip suggests adding a stick, which soldered memory (LPDDR) can't take.

use fettle_facts::Facts;
use fettle_verdicts::{evaluate, fmt};
use serde_json::json;

fn has_tip(kind: &str) -> bool {
    let facts: Facts = serde_json::from_value(json!({
        "memory": { "check": { "status": "ok" }, "data": { "slots_total": 2, "modules": [{ "capacity_bytes": 17179869184u64, "kind": kind }] } }
    }))
    .expect("facts");
    let r = evaluate(&facts, fmt::parse_rfc3339("2026-10-15T10:00:00Z").unwrap(), false);
    r.sections.iter().flat_map(|s| &s.lines).any(|l| l.headline.key == "memory.single_channel")
}

#[test]
fn soldered_memory_gets_no_tip_to_add_a_stick() {
    assert!(has_tip("ddr4"));
    assert!(has_tip("ddr5"));
    for kind in ["lpddr3", "lpddr4", "lpddr5"] {
        assert!(!has_tip(kind), "{kind}");
    }
}

fn has_tip_on(chassis: Option<&str>) -> bool {
    let mut report = json!({
        "memory": { "check": { "status": "ok" }, "data": { "slots_total": 2, "modules": [{ "capacity_bytes": 17179869184u64, "kind": "ddr4", "form_factor": "sodimm" }] } }
    });
    if let Some(c) = chassis {
        report["system"] = json!({ "check": { "status": "ok" }, "data": { "chassis": c } });
    }
    let facts: Facts = serde_json::from_value(report).expect("facts");
    let r = evaluate(&facts, fmt::parse_rfc3339("2026-10-15T10:00:00Z").unwrap(), false);
    r.sections.iter().flat_map(|s| &s.lines).any(|l| l.headline.key == "memory.single_channel")
}

// Laptop firmware often counts slots that aren't there (an ASUS X541UJ said 4 for soldered memory
// plus one slot), so a laptop never gets the tip to add a second stick.
#[test]
fn a_laptop_gets_no_tip_to_add_a_stick() {
    assert!(!has_tip_on(Some("laptop")));
    assert!(has_tip_on(Some("desktop")));
    assert!(has_tip_on(Some("unknown")));
    assert!(has_tip_on(None));
}
