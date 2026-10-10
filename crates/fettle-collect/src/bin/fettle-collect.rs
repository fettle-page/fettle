//! Scans this PC and prints the report as JSON: `fettle-collect [--out <file>]`.

use std::process::ExitCode;

#[cfg(windows)]
fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let out = match (args.next().as_deref(), args.next()) {
        (None, _) => None,
        (Some("--out"), Some(path)) => Some(path),
        _ => {
            eprintln!("usage: fettle-collect [--out <file>]");
            return ExitCode::from(2);
        }
    };
    let start = std::time::Instant::now();
    // Names and timings only: raw error text isn't redacted yet at this point.
    let report = fettle_collect::collect_with(&fettle_collect::Options::default(), |name, r| {
        eprintln!("{name:<22} {:<4} {:>5} ms", if r.ok { "ok" } else { "FAIL" }, r.ms);
    });
    let json = serde_json::to_string_pretty(&report).expect("a report always serializes");
    eprintln!("scanned in {} ms", start.elapsed().as_millis());
    match out {
        Some(path) => match std::fs::write(&path, json + "\n") {
            Ok(()) => eprintln!("wrote {path}"),
            Err(e) => {
                eprintln!("can't write {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => println!("{json}"),
    }
    ExitCode::SUCCESS
}

#[cfg(not(windows))]
fn main() -> ExitCode {
    eprintln!("fettle-collect scans Windows PCs only");
    ExitCode::FAILURE
}
