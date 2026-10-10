//! A warning in the last 60 days before Windows support ends, with the way to keep getting updates.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Level, Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

fn judge(facts: Value, now: &str, elevated: bool) -> Receipt {
    let facts: Facts = serde_json::from_value(facts).expect("facts");
    evaluate(&facts, fmt::parse_rfc3339(now).expect("now"), elevated)
}
fn lines(r: &Receipt) -> Vec<&Line> {
    r.sections.iter().flat_map(|s| &s.lines).collect()
}
fn find<'a>(r: &'a Receipt, id: &str) -> &'a Line {
    lines(r).into_iter().find(|l| l.id == id).unwrap_or_else(|| panic!("no line {id}: {:?}", lines(r).iter().map(|l| &l.id).collect::<Vec<_>>()))
}
fn ok(data: Value) -> Value {
    json!({ "check": { "status": "ok" }, "data": data })
}
fn windows(product: &str, edition: &str, build: u32, version: &str) -> Value {
    ok(json!({ "product": product, "edition_id": edition, "build": build, "display_version": version }))
}

// 3. Before support ends, a warning with the way to keep getting updates.
#[test]
fn windows_11_warns_in_the_last_60_days_then_acts_soon() {
    // Windows 11 Home 24H2: support ends 14 October 2026.
    let w = json!({ "windows": windows("Windows 11 Home", "Core", 26100, "24H2") });
    let early = judge(w.clone(), "2026-08-01T10:00:00Z", false);
    assert_eq!(find(&early, "windows-support").headline.key, "windows.supported");
    assert_eq!(find(&early, "windows-support").level, Level::Fine);

    let soon = judge(w.clone(), "2026-10-01T10:00:00Z", false);
    let l = find(&soon, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.ending_feature_update"));
    assert_eq!(l.headline.params.as_ref().and_then(|p| p.get("until")).map(|v| format!("{v:?}")), Some(r#"Str("2026-10-13")"#.into()));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("windows.ending_feature_update.action"));

    let after = judge(w, "2026-10-20T10:00:00Z", false);
    assert_eq!(find(&after, "windows-support").level, Level::ActSoon);
}

#[test]
fn windows_10_22h2_points_at_esu_before_its_end() {
    // Windows 10 Home 22H2: support ended 14 October 2025; ESU carries on.
    let soon = judge(json!({ "windows": windows("Windows 10 Home", "Core", 19045, "22H2") }), "2025-09-01T10:00:00Z", false);
    let l = find(&soon, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.ending_esu"));
}

#[test]
fn other_windows_versions_are_told_to_upgrade_before_the_end() {
    // Windows 10 Home 21H2 ended 13 June 2023, with no ESU.
    let soon = judge(json!({ "windows": windows("Windows 10 Home", "Core", 19044, "21H2") }), "2023-05-01T10:00:00Z", false);
    let l = find(&soon, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.ending"));
}
