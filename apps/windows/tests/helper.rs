//! fettle.exe started as the Deep check's helper with a file it won't write: it stops with exit
//! code 1 and never opens the app, which would run as administrator.

use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn a_refused_deep_scan_target_stops_without_opening_the_app() {
    for target in [r"\\nas\tools\Fettle data\fettle-deep-1-1.json", "fettle-deep-1-1.json", r"C:\Windows\win.ini"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fettle")).args(["--deep-scan", target]).spawn().expect("fettle.exe starts");
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().expect("waits") {
                break status;
            }
            if started.elapsed() > Duration::from_secs(20) {
                let _ = child.kill();
                panic!("{target}: still running after 20 s, so it opened the app");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(status.code(), Some(1), "{target}");
    }
}
