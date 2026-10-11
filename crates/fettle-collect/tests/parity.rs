//! The Rust mapping must turn each tracked spike dump into the same facts as the JS converter the
//! verdict tests trust (crates/fettle-receipt/parity/from-spike.mjs). The fixtures are its output.

use fettle_collect::{Raw, map};
use fettle_facts::Facts;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

#[derive(Deserialize)]
struct Dump {
    collected_at_unix: u64,
    sources: Raw,
}

#[test]
fn rust_mapping_matches_the_js_converter_on_every_spike_dump() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    // The real dumps, and made-up scans with odd values they don't have.
    let entries = fs::read_dir(root.join("../../spike/dumps")).unwrap().chain(fs::read_dir(root.join("tests/twin-cases")).unwrap());
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let dump: Dump = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let want: Facts = serde_json::from_str(&fs::read_to_string(root.join(format!("tests/fixtures/{name}.facts.json"))).unwrap()).unwrap();
        let mut got = map::facts(&dump.sources, UNIX_EPOCH + Duration::from_secs(dump.collected_at_unix));
        // The JS converter has no system section; it's tested on its own in map.rs.
        got.system = None;
        for (section, got, want) in [
            ("storage", serde_json::to_value(&got.storage), serde_json::to_value(&want.storage)),
            ("battery", serde_json::to_value(&got.battery), serde_json::to_value(&want.battery)),
            ("memory", serde_json::to_value(&got.memory), serde_json::to_value(&want.memory)),
            ("windows", serde_json::to_value(&got.windows), serde_json::to_value(&want.windows)),
            ("startup", serde_json::to_value(&got.startup), serde_json::to_value(&want.startup)),
            ("graphics", serde_json::to_value(&got.graphics), serde_json::to_value(&want.graphics)),
        ] {
            assert_eq!(got.unwrap(), want.unwrap(), "{name}: {section}");
        }
        assert_eq!(got, want, "{name}");
        checked += 1;
    }
    assert_eq!(checked, 6);
}
