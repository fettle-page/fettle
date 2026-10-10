//! A real scan of this PC (Windows only, about a second).
#![cfg(windows)]

use fettle_collect::{Options, collect};
use fettle_facts::CheckStatus;

#[test]
fn a_real_scan_is_complete_and_leaks_no_names() {
    let report = collect(&Options::default());
    let json = serde_json::to_string(&report).unwrap();
    for var in ["COMPUTERNAME", "USERNAME"] {
        let name = std::env::var(var).unwrap();
        if name.len() >= 3 {
            assert!(!json.to_lowercase().contains(&name.to_lowercase()), "{var} leaked");
        }
    }
    assert_eq!(report.facts.storage.unwrap().check.status, CheckStatus::Ok);
    assert_eq!(report.facts.system.unwrap().check.status, CheckStatus::Ok);
    assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
}
