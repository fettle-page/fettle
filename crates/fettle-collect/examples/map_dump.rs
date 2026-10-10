//! Turns a spike dump into the report `fettle-collect` would build from the same raw data.
//! For checking dumps from other PCs: `cargo run -p fettle-collect --example map_dump -- dump.json`

use fettle_collect::{Raw, build_report};
use fettle_facts::redact::Redactor;
use serde::Deserialize;
use std::time::{Duration, UNIX_EPOCH};

#[derive(Deserialize)]
struct Dump {
    collected_at_unix: u64,
    elevated: bool,
    sources: Raw,
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: map_dump <dump.json>");
    let dump: Dump = serde_json::from_str(&std::fs::read_to_string(&path).expect("readable dump")).expect("a spike dump");
    let at = UNIX_EPOCH + Duration::from_secs(dump.collected_at_unix);
    // The dump is already redacted by the spike; no names to add here.
    let report = build_report(&dump.sources, at, dump.elevated, &Redactor::default(), "map-dump");
    println!("{}", serde_json::to_string_pretty(&report).expect("a report serializes"));
}
