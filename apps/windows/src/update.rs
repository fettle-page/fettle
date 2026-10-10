//! The update check: with its switch on, at most once a day, Fettle asks its website for the latest
//! version number (GET /app/latest.json: no query, no cookies, no headers of its own) and nothing
//! else. A missing, bad or failed answer is no news, silently.

use crate::share::Http;
use serde::{Deserialize, Serialize};

/// This app's version.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// The website's file with the latest version (on the share origin, the one configured domain).
pub const PATH: &str = "/app/latest.json";
/// How often the website is asked, at most.
pub const DAY_MS: i64 = 86_400_000;

/// The latest version, and where to download it in each language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Latest {
    pub version: String,
    pub page_en: String,
    pub page_it: String,
}

/// When the website last answered, and what it said (update.json, kept on this PC).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateRecord {
    pub checked_at: i64,
    pub latest: Option<Latest>,
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
