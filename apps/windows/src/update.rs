//! The update check: with its switch on, once a day unless the user chose otherwise, Fettle asks its
//! website for the latest version number (GET /app/latest.json: no query, no cookies, no headers of
//! its own) and nothing else. A missing, bad or failed answer is no news, silently, and the next try
//! waits: an hour, then two, then four, never longer than the chosen interval.

use crate::share::Http;
use serde::{Deserialize, Serialize};

/// This app's version.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// The website's file with the latest version (on the share origin, the one configured domain).
pub const PATH: &str = "/app/latest.json";
/// A day: how often the website is asked unless the user chose otherwise.
pub const DAY_MS: i64 = 86_400_000;
/// The first wait after a failed check (it doubles with each failure in a row).
pub const HOUR_MS: i64 = 3_600_000;

/// The latest version, and where to download it in each language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Latest {
    pub version: String,
    pub page_en: String,
    pub page_it: String,
}

/// When the website last answered, and what it said, and when it was last asked (update.json, kept on
/// this PC and shared by every Fettle window on it).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateRecord {
    pub checked_at: i64,
    pub latest: Option<Latest>,
    /// The last time the website was asked, answered or not.
    pub tried_at: i64,
    /// Failed checks since the last answer.
    pub failures: u32,
}

/// Whether to ask the website now: a full `every` after the last try, or after failures an hour,
/// then two, four and so on, never longer than `every`.
pub fn due(record: &UpdateRecord, every_ms: i64, now_ms: i64) -> bool {
    let last = record.tried_at.max(record.checked_at);
    let wait = if record.failures == 0 { every_ms } else { every_ms.min(HOUR_MS << (record.failures - 1).min(16)) };
    let never_asked = last == 0 && record.failures == 0;
    // A last try "in the future" was stamped by a clock that was wrong and has been set right.
    never_asked || now_ms < last || now_ms - last >= wait
}

/// The more recent of two records (by their last try): the one on disk may be another window's
/// newer try, or older than this window's own if the file couldn't be written.
pub fn newest(a: UpdateRecord, b: UpdateRecord) -> UpdateRecord {
    let last = |r: &UpdateRecord| r.tried_at.max(r.checked_at);
    if last(&b) > last(&a) { b } else { a }
}

/// The record after asking at `at`: what the website said, or one more failure (keeping what the
/// last answer found).
pub fn answered(before: &UpdateRecord, latest: Option<Latest>, at: i64) -> UpdateRecord {
    match latest {
        Some(latest) => UpdateRecord { checked_at: at, latest: Some(latest), tried_at: at, failures: 0 },
        None => UpdateRecord { tried_at: at, failures: before.failures.saturating_add(1), ..before.clone() },
    }
}

/// "1.2.3" as numbers, and whether it's a pre-release ("1.2.3-beta.1").
fn semver(v: &str) -> Option<([u64; 3], bool)> {
    let (core, pre) = match v.split_once('-') {
        Some((core, pre)) => (core, !pre.is_empty()),
        None => (v, false),
    };
    let mut parts = core.split('.').map(|p| if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) { p.parse().ok() } else { None });
    let numbers = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some((numbers, pre))
}

/// Whether `candidate` is a release newer than `current`. Pre-releases are never offered.
pub fn newer(candidate: &str, current: &str) -> bool {
    match (semver(candidate), semver(current)) {
        (Some((c, false)), Some((now, now_pre))) => c > now || (c == now && now_pre),
        _ => false,
    }
}

/// The website's answer: {"version": "x.y.z", "page": {"en": https URL, "it": https URL}}.
pub fn parse(body: &[u8]) -> Option<Latest> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let version = v["version"].as_str().filter(|s| semver(s).is_some())?;
    let page = |lang: &str| v["page"][lang].as_str().filter(|u| u.starts_with("https://")).map(String::from);
    Some(Latest { version: version.into(), page_en: page("en")?, page_it: page("it")? })
}

/// One plain GET to the website. None for anything but a good answer.
pub fn fetch(http: &dyn Http, origin: &str) -> Option<Latest> {
    let response = http.send("GET", &format!("{}{PATH}", origin.trim_end_matches('/')), &[], None).ok()?;
    (response.status == 200).then(|| parse(&response.body)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::share::{Http, Response};
    use std::sync::Mutex;

    fn found(version: &str) -> Option<Latest> {
        Some(Latest { version: version.into(), page_en: "https://fettle.page/download".into(), page_it: "https://fettle.page/it/download".into() })
    }

    #[test]
    fn after_an_answer_the_website_waits_the_chosen_interval() {
        assert!(due(&UpdateRecord::default(), DAY_MS, 0), "never asked");
        let r = answered(&UpdateRecord::default(), found("1.0.0"), 1_000);
        assert_eq!((r.checked_at, r.tried_at, r.failures), (1_000, 1_000, 0));
        assert!(!due(&r, DAY_MS, 1_000 + DAY_MS - 1));
        assert!(due(&r, DAY_MS, 1_000 + DAY_MS));
    }

    #[test]
    fn after_a_failure_it_waits_an_hour_then_two_then_four_never_longer_than_the_interval() {
        let once = answered(&UpdateRecord::default(), None, 0);
        assert_eq!(once.failures, 1);
        assert!(!due(&once, DAY_MS, HOUR_MS - 1), "not every ten minutes");
        assert!(due(&once, DAY_MS, HOUR_MS));
        let twice = answered(&once, None, HOUR_MS);
        assert!(!due(&twice, DAY_MS, 3 * HOUR_MS - 1));
        assert!(due(&twice, DAY_MS, 3 * HOUR_MS));
        let thrice = answered(&twice, None, 3 * HOUR_MS);
        assert!(!due(&thrice, DAY_MS, 7 * HOUR_MS - 1));
        assert!(due(&thrice, DAY_MS, 7 * HOUR_MS));
        let many = (0..40).fold(UpdateRecord::default(), |r, i| answered(&r, None, i));
        assert!(!due(&many, DAY_MS, 39 + DAY_MS - 1), "at most once a day, as promised");
        assert!(due(&many, DAY_MS, 39 + DAY_MS));
        assert!(due(&many, HOUR_MS, 39 + HOUR_MS), "never longer than the chosen interval");
    }

    #[test]
    fn a_failure_keeps_what_the_last_answer_found() {
        let good = answered(&UpdateRecord::default(), found("999.0.0"), 5);
        let failed = answered(&good, None, 10);
        assert_eq!((failed.latest.clone(), failed.checked_at, failed.tried_at), (good.latest.clone(), 5, 10));
        assert_eq!(answered(&failed, found("999.0.1"), 20).failures, 0, "an answer starts over");
    }

    #[test]
    fn the_newer_of_two_records_counts() {
        let old = answered(&UpdateRecord::default(), found("1.0.0"), 1_000);
        let new = answered(&old, None, 2_000);
        assert_eq!(newest(old.clone(), new.clone()), new);
        assert_eq!(newest(new.clone(), old.clone()), new);
        assert_eq!(newest(UpdateRecord::default(), old.clone()), old, "a file that couldn't be written counts as never asked");
    }

    #[test]
    fn a_clock_set_back_doesnt_stop_the_checks() {
        // The last try was stamped while the clock was a year ahead; now it's right again.
        let r = answered(&UpdateRecord::default(), found("1.0.0"), 365 * DAY_MS);
        assert!(due(&r, DAY_MS, 10 * DAY_MS));
    }

    #[test]
    fn an_update_file_from_before_attempts_were_kept_still_reads() {
        let old: UpdateRecord = serde_json::from_str(r#"{"checked_at": 5, "latest": null}"#).unwrap();
        assert_eq!((old.checked_at, old.tried_at, old.failures), (5, 0, 0));
        assert!(!due(&old, DAY_MS, 5 + DAY_MS - 1));
    }

    #[test]
    fn the_website_is_asked_every_hour_day_or_week() {
        use crate::state::UpdateEvery;
        assert_eq!(UpdateEvery::Hourly.ms(), 3_600_000);
        assert_eq!(UpdateEvery::Daily.ms(), DAY_MS);
        assert_eq!(UpdateEvery::Weekly.ms(), 7 * DAY_MS);
    }

    #[test]
    fn only_a_higher_release_is_newer() {
        assert!(newer("0.2.0", "0.1.0"));
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("1.0.0", "0.9.9"));
        assert!(newer("0.2.0", "0.2.0-beta.1"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.2.0"));
        // Pre-releases are never offered, and nonsense is no news.
        assert!(!newer("0.2.0-beta.1", "0.1.0"));
        assert!(!newer("garbage", "0.1.0"));
        assert!(!newer("0.2", "0.1.0"));
        assert!(!newer("", "0.1.0"));
    }

    const GOOD: &str = r#"{"version":"0.2.0","page":{"en":"https://fettle.page/download","it":"https://fettle.page/it/download"}}"#;

    #[test]
    fn the_websites_answer_is_read_strictly() {
        let latest = parse(GOOD.as_bytes()).expect("the site's answer");
        assert_eq!(latest, Latest { version: "0.2.0".into(), page_en: "https://fettle.page/download".into(), page_it: "https://fettle.page/it/download".into() });
        for bad in [
            "",
            "not json",
            r#"{"version":"0.2.0"}"#,
            r#"{"version":"0.2","page":{"en":"https://fettle.page/download","it":"https://fettle.page/it/download"}}"#,
            r#"{"version":"0.2.0","page":{"en":"http://fettle.page/download","it":"https://fettle.page/it/download"}}"#,
            r#"{"version":"0.2.0","page":{"en":"https://fettle.page/download","it":"javascript:alert(1)"}}"#,
        ] {
            assert_eq!(parse(bad.as_bytes()), None, "{bad}");
        }
    }

    struct Site(u16, &'static str, Mutex<Vec<(String, String, usize, bool)>>);

    impl Http for Site {
        fn send(&self, method: &str, url: &str, headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()> {
            self.2.lock().unwrap().push((method.into(), url.into(), headers.len(), body.is_some()));
            Ok(Response { status: self.0, headers: Default::default(), body: self.1.as_bytes().to_vec() })
        }
    }

    #[test]
    fn the_check_is_one_plain_get() {
        let site = Site(200, GOOD, Mutex::default());
        assert_eq!(fetch(&site, "https://fettle.page").map(|l| l.version), Some("0.2.0".into()));
        assert_eq!(*site.2.lock().unwrap(), vec![("GET".to_string(), "https://fettle.page/app/latest.json".to_string(), 0, false)]);
        assert_eq!(fetch(&Site(404, GOOD, Mutex::default()), "https://fettle.page"), None);
        assert_eq!(fetch(&Site(500, "", Mutex::default()), "https://fettle.page"), None);
    }
}
