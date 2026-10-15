//! The Edition row ("Professional") only shows when it says something the product name doesn't:
//! after "Windows 11 Pro" it repeats it, in English even on an Italian receipt.

use fettle_facts::Report;
use fettle_receipt::{Zone, receipt_for};
use fettle_verdicts::Locale;
use serde_json::json;

fn edition_row(product: Option<&str>, edition: &str, locale: Locale) -> Option<String> {
    let report: Report = serde_json::from_value(json!({
        "schema_version": 1, "app_version": "1.0.5", "collected_at": "2026-10-15T10:00:00Z", "elevated": true,
        "facts": {
            "windows": { "check": { "status": "ok" }, "data": { "product": product, "edition_id": edition, "build": 26100, "display_version": "24H2" } }
        }
    }))
    .expect("report");
    let (_, model) = receipt_for(&report, locale, Zone::Utc).expect("receipt");
    let json = serde_json::to_value(&model).expect("model");
    let windows = json["sections"].as_array().expect("sections").iter().find(|s| s["id"] == "windows").expect("windows section").clone();
    let label = if locale == Locale::It { "Edizione" } else { "Edition" };
    windows["details"].as_array().expect("details").iter().flat_map(|g| g["rows"].as_array().cloned().unwrap_or_default()).find(|r| r["label"] == label).map(|r| r["value"].as_str().unwrap_or_default().to_string())
}

#[test]
fn an_edition_the_product_name_already_says_is_not_repeated() {
    for (product, edition) in [
        ("Windows 11 Pro", "Professional"),
        ("Windows 11 Home", "Core"),
        ("Windows 11 Home Single Language", "CoreSingleLanguage"),
        ("Windows 11 Pro N", "ProfessionalN"),
        ("Windows 11 Pro for Workstations", "ProfessionalWorkstation"),
        ("Windows 11 Pro Education", "ProfessionalEducation"),
        ("Windows 11 Enterprise", "Enterprise"),
        ("Windows 11 Education", "Education"),
        ("Windows 10 Enterprise LTSC 2021", "EnterpriseS"),
        ("Windows 10 Enterprise 2016 LTSB", "EnterpriseS"),
        ("Windows 10 IoT Enterprise LTSC 2021", "IoTEnterpriseS"),
        ("Windows 11 IoT Enterprise", "IoTEnterprise"),
    ] {
        assert_eq!(edition_row(Some(product), edition, Locale::En), None, "{product} / {edition}");
        assert_eq!(edition_row(Some(product), edition, Locale::It), None, "{product} / {edition}");
    }
}

#[test]
fn an_edition_that_adds_something_is_kept() {
    // The product name says another edition, or none, or Fettle doesn't know the edition.
    assert_eq!(edition_row(Some("Windows 10 Pro"), "Enterprise", Locale::En), Some("Enterprise".into()));
    assert_eq!(edition_row(Some("Windows 11 Enterprise"), "EnterpriseS", Locale::En), Some("EnterpriseS".into()));
    assert_eq!(edition_row(None, "Professional", Locale::En), Some("Professional".into()));
    assert_eq!(edition_row(Some("Windows 11 Pro"), "ServerRdsh", Locale::It), Some("ServerRdsh".into()));
}
