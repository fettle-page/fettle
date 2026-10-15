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
