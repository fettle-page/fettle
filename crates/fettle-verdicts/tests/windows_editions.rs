//! Windows editions with their own support dates: LTSC/LTSB (matched by build) and IoT Enterprise
//! (the Enterprise and Education dates). Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Level, Line, Receipt, evaluate, fmt};
use serde_json::json;

fn support(edition: &str, build: u32, version: &str, now: &str) -> Line {
    let facts: Facts = serde_json::from_value(json!({
        "windows": { "check": { "status": "ok" }, "data": { "product": "Windows", "edition_id": edition, "build": build, "display_version": version } }
    }))
    .expect("facts");
    let r: Receipt = evaluate(&facts, fmt::parse_rfc3339(now).expect("now"), false);
    r.sections.iter().flat_map(|s| &s.lines).find(|l| l.id == "windows-support").expect("support line").clone()
}
fn until(l: &Line) -> Option<String> {
    l.headline.params.as_ref().and_then(|p| p.get("until")).map(|v| format!("{v:?}"))
}

#[test]
fn windows_11_ltsc_2024_is_known_by_its_build() {
    let ltsc = support("EnterpriseS", 26100, "24H2", "2026-10-11T10:00:00Z");
    assert_eq!((ltsc.level, ltsc.headline.key.as_str()), (Level::Fine, "windows.supported"));
    assert_eq!(until(&ltsc), Some(r#"Str("2029-10-09")"#.into()));

    let iot = support("IoTEnterpriseS", 26100, "24H2", "2026-10-11T10:00:00Z");
    assert_eq!(until(&iot), Some(r#"Str("2034-10-10")"#.into()));
}

#[test]
fn ltsc_near_its_end_is_told_to_upgrade_not_to_take_a_feature_update() {
    let l = support("EnterpriseS", 26100, "24H2", "2029-09-01T10:00:00Z");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.ending"));
    let after = support("EnterpriseS", 26100, "24H2", "2029-11-01T10:00:00Z");
    assert_eq!((after.level, after.headline.key.as_str()), (Level::ActSoon, "windows.unsupported"));
}

#[test]
fn windows_10_ltsb_2016_ends_on_13_october_2026() {
    for edition in ["EnterpriseS", "IoTEnterpriseS"] {
        let soon = support(edition, 14393, "", "2026-10-11T10:00:00Z");
        assert_eq!((soon.level, soon.headline.key.as_str()), (Level::KeepAnEye, "windows.ending"), "{edition}");
        assert_eq!(until(&soon), Some(r#"Str("2026-10-13")"#.into()));
        assert_eq!(support(edition, 14393, "", "2026-10-20T10:00:00Z").level, Level::ActSoon, "{edition}");
    }
}

#[test]
fn windows_10_ltsb_2015_and_iot_ltsc_2019_are_known() {
    let old = support("EnterpriseS", 10240, "", "2026-10-11T10:00:00Z");
    assert_eq!((old.level, old.headline.key.as_str()), (Level::ActSoon, "windows.unsupported"));
    let iot_2019 = support("IoTEnterpriseS", 17763, "", "2026-10-11T10:00:00Z");
    assert_eq!(until(&iot_2019), Some(r#"Str("2029-01-09")"#.into()));
}

#[test]
fn iot_enterprise_follows_the_enterprise_dates() {
    let w11 = support("IoTEnterprise", 26100, "24H2", "2026-10-11T10:00:00Z");
    assert_eq!((w11.level, w11.headline.key.as_str()), (Level::Fine, "windows.supported"));
    assert_eq!(until(&w11), Some(r#"Str("2027-10-12")"#.into()));
    // Windows 10 22H2 IoT Enterprise is past its end, where commercial ESU can carry it.
    let w10 = support("IoTEnterpriseK", 19045, "22H2", "2026-10-11T10:00:00Z");
    assert_eq!(w10.headline.key, "windows.esu_unknown");
}
