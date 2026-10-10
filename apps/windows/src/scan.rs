//! The real scan and the real background: fettle-collect on a thread of its own.

use crate::app::{Background, Job, Scanner};
use fettle_facts::Report;

/// Scans this PC. The report is redacted before it leaves fettle-collect; the callback gets source
/// names only (raw results aren't redacted yet at that point).
pub struct RealScanner;

impl Scanner for RealScanner {
    fn scan(&self, on_source: &dyn Fn(&str)) -> Report {
        fettle_collect::collect_with(&options(), |name, _| on_source(name))
    }
}

/// The scan's options, naming this app's version in the report.
fn options() -> fettle_collect::Options {
    fettle_collect::Options { app_version: env!("CARGO_PKG_VERSION").to_string(), ..Default::default() }
}

/// Each job on a new thread: a scan or an upload at a time, so no pool is needed.
pub struct Threads;

impl Background for Threads {
    fn spawn(&self, job: Job) {
        std::thread::spawn(job);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_carry_the_app_s_version_not_the_scan_crate_s() {
        // The receipt's footer prints it ("Fettle 1.0.0").
        assert_eq!(super::options().app_version, env!("CARGO_PKG_VERSION"));
    }
}
