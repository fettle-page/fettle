//! Prints the reports `fettle-collect` would build from saved scans (test dumps), as a JSON array in
//! the order given: `cargo run -q -p fettle-collect --example map_dump -- <dump.json>...`
//! `npm run parity:goldens` uses it for the receipt corpus's dump cases.

use fettle_collect::dump::Dump;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: map_dump <dump.json>...");
        std::process::exit(2);
    }
    let reports: Vec<_> = paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
            Dump::from_json(&text).unwrap_or_else(|e| panic!("{path}: not a saved scan: {e}")).report()
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&reports).expect("a report serializes"));
}
