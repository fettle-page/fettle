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

// The end of Extended Security Updates gets the same warning window as the end of support.
#[test]
fn enrolled_windows_10_warns_before_esu_itself_ends() {
    let home = json!({ "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 10 Home", "edition_id": "Core", "build": 19045, "display_version": "22H2", "esu": "enrolled" } } });
    let early = judge(home.clone(), "2027-07-01T10:00:00Z", false);
    let fine = find(&early, "windows-support");
    assert_eq!((fine.level, fine.headline.key.as_str()), (Level::Fine, "windows.esu_enrolled_until"));

    let soon = judge(home, "2027-09-01T10:00:00Z", false);
    let l = find(&soon, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.esu_ending"));
    assert_eq!(l.headline.params.as_ref().and_then(|p| p.get("until")), fine.headline.params.as_ref().and_then(|p| p.get("until")));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("windows.esu_ending.action"));

    // Commercial ESU's last possible day is known even when the paid-for year isn't.
    let edu = json!({ "windows": { "check": { "status": "ok" }, "data": { "product": "Windows 10 Education", "edition_id": "Education", "build": 19045, "display_version": "22H2", "esu": "enrolled" } } });
    assert_eq!(find(&judge(edu.clone(), "2028-01-01T10:00:00Z", false), "windows-support").headline.key, "windows.esu_enrolled");
    assert_eq!(find(&judge(edu, "2028-09-01T10:00:00Z", false), "windows-support").headline.key, "windows.esu_ending");
}

// Enrolment in ESU shows in the build: 19045.6466 (KB5071959, November 2025) was the last update
// Windows 10 22H2 got without ESU; every later one comes only through ESU. When an update was
// installed proves nothing, because PCs install old updates late.
fn win10_home(ubr: Option<u32>, last_update: Option<&str>) -> Value {
    ok(json!({ "product": "Windows 10 Home", "edition_id": "Core", "build": 19045, "display_version": "22H2", "ubr": ubr, "last_update_installed": last_update }))
}

#[test]
fn a_late_installed_october_2025_update_does_not_mean_esu() {
    // The October 2025 update (19045.6456) installed three days after it came out, never enrolled.
    let r = judge(json!({ "windows": win10_home(Some(6456), Some("2025-10-17")) }), "2025-12-01T10:00:00Z", false);
    let l = find(&r, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "windows.esu_unknown"));
}

#[test]
fn the_update_for_pcs_without_esu_does_not_mean_esu() {
    // KB5071959 was offered only to PCs not enrolled, to fix the enrolment wizard.
    let r = judge(json!({ "windows": win10_home(Some(6466), Some("2025-11-20")) }), "2025-12-20T10:00:00Z", false);
    assert_eq!(find(&r, "windows-support").headline.key, "windows.esu_unknown");
}

// A PC still on the last build without ESU, two Patch Tuesdays (December, January) after it, has
// missed security updates whether it's enrolled or not.
#[test]
fn no_security_update_since_november_2025_means_act_soon() {
    let stale = judge(json!({ "windows": win10_home(Some(6466), Some("2026-09-25")) }), "2026-10-01T10:00:00Z", false);
    let l = find(&stale, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::ActSoon, "windows.esu_stale"));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("windows.esu_stale.action"));
    let older = judge(json!({ "windows": win10_home(Some(6456), None) }), "2026-03-01T10:00:00Z", false);
    assert_eq!(find(&older, "windows-support").headline.key, "windows.esu_stale");
    // Up to the January 2026 Patch Tuesday a late install is still likely.
    let early = judge(json!({ "windows": win10_home(Some(6466), None) }), "2026-01-13T10:00:00Z", false);
    assert_eq!(find(&early, "windows-support").headline.key, "windows.esu_unknown");
    // Without the build revision, nothing to go on.
    let unknown = judge(json!({ "windows": win10_home(None, None) }), "2026-10-01T10:00:00Z", false);
    assert_eq!(find(&unknown, "windows-support").headline.key, "windows.esu_unknown");
}

#[test]
fn a_build_only_esu_delivers_means_enrolled() {
    // 19045.6575 is the November 2025 ESU update.
    let r = judge(json!({ "windows": win10_home(Some(6575), Some("2025-11-12")) }), "2026-03-01T10:00:00Z", false);
    let l = find(&r, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::Fine, "windows.esu_enrolled_until"));
}

#[test]
fn without_the_build_revision_an_update_date_proves_nothing() {
    let r = judge(json!({ "windows": win10_home(None, Some("2026-02-11")) }), "2026-03-01T10:00:00Z", false);
    assert_eq!(find(&r, "windows-support").headline.key, "windows.esu_unknown");
}

#[test]
fn windows_own_answer_about_esu_wins_over_the_build() {
    let mut w = win10_home(Some(6575), None);
    w["data"]["esu"] = json!("not_enrolled");
    let r = judge(json!({ "windows": w }), "2026-03-01T10:00:00Z", false);
    assert_eq!(find(&r, "windows-support").headline.key, "windows.unsupported_esu");
}

#[test]
fn an_enrolled_pc_stuck_on_the_november_2025_build_still_acts_soon() {
    let mut w = win10_home(Some(6466), None);
    w["data"]["esu"] = json!("enrolled");
    let r = judge(json!({ "windows": w }), "2026-10-01T10:00:00Z", false);
    let l = find(&r, "windows-support");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::ActSoon, "windows.esu_stale"));
    // Already enrolled: the updates just need installing.
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("windows.esu_stale_enrolled.action"));
}

// Organisations buy Extended Security Updates through volume licensing: Windows Update doesn't offer
// them to Enterprise or Education, so the way out is whoever manages the PC.
#[test]
fn enterprise_and_education_are_sent_to_whoever_manages_the_pc() {
    let w = |esu: &str, ubr: Option<u32>| ok(json!({ "product": "Windows 10 Enterprise", "edition_id": "Enterprise", "build": 19045, "display_version": "22H2", "ubr": ubr, "esu": esu }));
    let stale = find(&judge(json!({ "windows": w("unknown", Some(6466)) }), "2026-10-01T10:00:00Z", false), "windows-support").clone();
    assert_eq!(stale.headline.key, "windows.esu_stale");
    assert_eq!(stale.action.as_ref().map(|a| a.key.as_str()), Some("windows.esu_commercial.action"));
    let not = find(&judge(json!({ "windows": w("not_enrolled", None) }), "2026-10-01T10:00:00Z", false), "windows-support").clone();
    assert_eq!(not.headline.key, "windows.unsupported_esu");
    assert_eq!(not.action.as_ref().map(|a| a.key.as_str()), Some("windows.esu_commercial.action"));
    // Home and Pro keep Windows Update's own enrolment.
    let mut home = win10_home(None, None);
    home["data"]["esu"] = json!("not_enrolled");
    let home = find(&judge(json!({ "windows": home }), "2026-10-01T10:00:00Z", false), "windows-support").clone();
    assert_eq!(home.action.as_ref().map(|a| a.key.as_str()), Some("windows.unsupported_esu.action"));
}
