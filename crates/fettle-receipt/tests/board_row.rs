//! The Board row leaves out what firmware writes when the maker never filled the name in
//! ("Default string", "To Be Filled By O.E.M."). Reports from 1.0.4 and earlier still carry it.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn board_row(board: &str) -> Option<String> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.4", "collected_at": "2026-10-15T10:00:00Z", "elevated": true,
        "facts": {
            "system": { "check": { "status": "ok" }, "data": { "cpu_name": "Example CPU 1000", "board": board, "bios_version": "TEST-1.00" } }
        }
    }))
    .expect("report");
    let (_, model) = receipt_for(&report, Locale::En, Zone::Utc).expect("receipt");
    model.machine.iter().find(|r| r.label == "Board").map(|r| r.value.clone())
}

#[test]
fn a_board_name_the_maker_never_filled_in_is_not_shown() {
    assert_eq!(board_row("Default string Default string"), None);
    assert_eq!(board_row("To Be Filled By O.E.M. To Be Filled By O.E.M."), None);
    assert_eq!(board_row("System manufacturer System Product Name"), None);
    assert_eq!(board_row("Example Boards Ltd. Default string"), Some("Example Boards Ltd.".into()));
    assert_eq!(board_row("Default string EX-B650 TEST"), Some("EX-B650 TEST".into()));
    assert_eq!(board_row("Example Boards Ltd. EX-B650 TEST"), Some("Example Boards Ltd. EX-B650 TEST".into()));
}

#[test]
fn a_generic_word_inside_a_real_board_name_stays() {
    assert_eq!(board_row("Example Boards Ltd. None Edition"), Some("Example Boards Ltd. None Edition".into()));
    assert_eq!(board_row("N/A"), None);
    assert_eq!(board_row("Default string N/A"), None);
}
