mod elevation;
mod nvme;
mod redact;
mod report;
mod sources;
mod wmi_util;

use redact::{redact_source, redact_str};
use report::{Dump, run_source, sanitize_label, write_dump};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage: fettle-spike [--label <name>] [--out <dir>] [--include-private] [--timeout-secs <n>]";

struct Args {
    label: String,
    out: PathBuf,
    include_private: bool,
    timeout: Duration,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        label: "unlabelled".into(),
        out: PathBuf::from("dumps"),
        include_private: false,
        timeout: Duration::from_secs(15),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--label" => args.label = it.next().ok_or("--label needs a value")?,
            "--out" => args.out = PathBuf::from(it.next().ok_or("--out needs a value")?),
            "--include-private" => args.include_private = true,
            "--timeout-secs" => {
                let n = it.next().ok_or("--timeout-secs needs a value")?;
                let secs: u64 = n.parse().map_err(|_| format!("bad --timeout-secs: {n}"))?;
                args.timeout = Duration::from_secs(secs);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let names = redact::this_pc();
    let start = Instant::now();
    let mut results = BTreeMap::new();
    for (name, collector) in sources::all() {
        let mut r = run_source(collector, args.timeout);
        if let Some(data) = r.data.as_mut() {
            redact_source(&name, data, &names, args.include_private);
        }
        r.error = r.error.map(|e| redact_str(&e, &names, args.include_private));
        let status = if r.ok { "ok" } else { "FAIL" };
        let err: String = r.error.as_deref().unwrap_or("").chars().take(80).collect();
        eprintln!("{name:<24} {status:<4} {:>6} ms  {err}", r.ms);
        results.insert(name, r);
    }

    let dump = Dump {
        spike_version: env!("CARGO_PKG_VERSION").into(),
        label: sanitize_label(&args.label),
        collected_at_unix: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        elevated: elevation::is_elevated(),
        private_included: args.include_private,
        total_ms: start.elapsed().as_millis() as u64,
        sources: results,
    };
    eprintln!("total {} ms (elevated: {})", dump.total_ms, dump.elevated);

    match write_dump(&args.out, &dump) {
        Ok(path) => {
            eprintln!("wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("couldn't save the file ({e}), printing the JSON instead:");
            println!("{}", serde_json::to_string_pretty(&dump).unwrap_or_default());
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod kit_tests {
    /// The test kit's batch file runs in cmd.exe on other PCs: Windows line endings, Windows
    /// device names (a Unix shell once turned `>nul` into `>/dev/null`, so every run said "user").
    #[test]
    fn the_kit_batch_file_is_plain_cmd() {
        let bat = include_bytes!("../kit/run-fettle-test.bat");
        let text = std::str::from_utf8(bat).unwrap();
        assert!(text.is_ascii(), "no accented letters: older consoles garble them");
        assert_eq!(text.matches('\n').count(), text.matches("\r\n").count(), "every line ends with CRLF");
        assert!(!text.contains("/dev/"), "no Unix paths");
        assert!(text.contains("fltmc >nul 2>&1 && (set \"MODE=admin\")"), "admin check that needs no service");
    }
}
