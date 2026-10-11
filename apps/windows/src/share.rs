//! Share links (packages/share-client, in Rust): upload the redacted report, delete it later.
//! Never panics; every outcome is a value the panel can word. The limits mirror the Worker's, so a
//! bad request isn't even sent. The password is sent once, over HTTPS, and kept nowhere.

use fettle_facts::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;
pub const MAX_UPLOAD_BYTES: usize = 65_536;

/// fettle.page. A development build can point at a local Worker with FETTLE_SHARE_ORIGIN; a
/// release build ignores it, so no setting outside the app can send shared reports elsewhere.
pub fn origin() -> String {
    origin_from(std::env::var("FETTLE_SHARE_ORIGIN").ok(), cfg!(debug_assertions))
}

fn origin_from(env: Option<String>, development: bool) -> String {
    env.filter(|_| development).unwrap_or_else(|| "https://fettle.page".into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiry {
    Week,
    Month,
    Year,
}

impl Expiry {
    pub fn code(&self) -> &'static str {
        match self {
            Expiry::Week => "7d",
            Expiry::Month => "30d",
            Expiry::Year => "1y",
        }
    }
}

/// What the Worker returns for a new link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    pub id: String,
    pub url: String,
    pub delete_url: String,
    pub delete_token: String,
    pub expires_at: String,
    pub locked: bool,
}

/// The Worker's error codes, plus what can go wrong before or around the request.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub error: &'static str,
    /// Seconds, after rate_limited.
    pub retry_after: Option<u64>,
    pub detail: Option<String>,
}

impl Failure {
    fn new(error: &'static str) -> Failure {
        Failure { error, retry_after: None, detail: None }
    }
}

const SERVER_ERRORS: [&str; 8] = ["invalid_request", "invalid_report", "too_large", "rate_limited", "unauthorized", "forbidden", "unsupported_media_type", "invalid_json"];

pub struct Response {
    pub status: u16,
    /// Lower-case names.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// One HTTPS request. Err means no answer at all (offline, DNS, TLS, timeout).
pub trait Http: Send + Sync {
    #[expect(clippy::result_unit_err, reason = "no answer is all the callers need to know: they say so the same way whatever the cause")]
    fn send(&self, method: &str, url: &str, headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()>;
}

fn endpoint(origin: &str, path: &str) -> String {
    format!("{}{path}", origin.trim_end_matches('/'))
}

fn read_json(r: &Response) -> Option<serde_json::Value> {
    serde_json::from_slice(&r.body).ok()
}

fn failure(r: &Response) -> Failure {
    let body = read_json(r);
    let code = body.as_ref().and_then(|b| b.get("error")).and_then(|e| e.as_str()).unwrap_or("");
    let Some(error) = SERVER_ERRORS.iter().find(|c| **c == code) else { return Failure::new("bad_response") };
    let retry_after = r.headers.get("retry-after").and_then(|v| v.trim().parse::<u64>().ok()).filter(|w| *w > 0 && *error == "rate_limited");
    let detail = body.as_ref().and_then(|b| b.get("detail")).and_then(|d| d.as_str()).map(str::to_string);
    Failure { error, retry_after, detail }
}

/// A QR code of a link, to open it on a phone: its side in modules (quiet zone included) and the
/// dark modules as a Slint path, one rectangle per run of dark modules in a row.
pub struct Qr {
    pub size: u32,
    pub path: String,
    runs: Vec<(u32, u32, u32)>,
}

impl Qr {
    /// The dark runs: (x, y, width) in modules.
    pub fn runs(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.runs.iter().copied()
    }
}

/// The QR code for `text` (medium error correction, the usual 4-module quiet zone around it).
pub fn qr_code(text: &str) -> Option<Qr> {
    use qrcodegen::{QrCode, QrCodeEcc};
    let code = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let (n, quiet) = (code.size(), 4);
    let mut runs = Vec::new();
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if code.get_module(x, y) {
                let start = x;
                while x < n && code.get_module(x, y) {
                    x += 1;
                }
                runs.push(((start + quiet) as u32, (y + quiet) as u32, (x - start) as u32));
            } else {
                x += 1;
            }
        }
    }
    let path = runs.iter().map(|&(x, y, w)| format!("M{x} {y}L{} {y}L{} {}L{x} {}Z", x + w, x + w, y + 1, y + 1)).collect::<String>();
    Some(Qr { size: (n + 2 * quiet) as u32, path, runs })
}

pub fn share_report(http: &dyn Http, origin: &str, report: &Report, expires: Option<Expiry>, password: Option<&str>) -> Result<Shared, Failure> {
    let password = password.filter(|p| !p.is_empty());
    if let Some(p) = password {
        // Counted as the Worker counts (JavaScript string length).
        let len = p.encode_utf16().count();
        if len < PASSWORD_MIN {
            return Err(Failure::new("password_too_short"));
        }
        if len > PASSWORD_MAX {
            return Err(Failure::new("password_too_long"));
        }
    }
    // A check remembered from an older version may hold text over today's upload limits.
    let mut report = report.clone();
    fettle_facts::fit::fit_limits(&mut report);
    let mut body = serde_json::json!({ "report": report });
    if let Some(e) = expires {
        body["expires"] = e.code().into();
    }
    if let Some(p) = password {
        body["password"] = p.into();
    }
    let bytes = body.to_string().into_bytes();
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(Failure::new("too_large"));
    }
    let r = http.send("POST", &endpoint(origin, "/api/reports"), &[("content-type", "application/json".into())], Some(bytes)).map_err(|()| Failure::new("network"))?;
    if r.status != 201 {
        return Err(failure(&r));
    }
    read_json(&r).and_then(|v| serde_json::from_value::<Shared>(v).ok()).ok_or(Failure::new("bad_response"))
}

/// How a Delete went: deleted now, or it was already gone (deleted on the website, say).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleted {
    Now,
    AlreadyGone,
}

/// The server's own report ids: letters and digits. Anything else is refused rather than sent.
fn sane_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Whether a shared link still exists: open or locked, the server knows it; deleted or expired, it
/// says "not found". A plain GET (no token, nothing sent); any other answer is an error, never "gone".
pub fn link_exists(http: &dyn Http, origin: &str, id: &str) -> Result<bool, Failure> {
    if !sane_id(id) {
        return Err(Failure::new("bad_response"));
    }
    let r = http.send("GET", &endpoint(origin, &format!("/api/reports/{id}")), &[], None).map_err(|()| Failure::new("network"))?;
    let error = read_json(&r).and_then(|b| b.get("error").and_then(|e| e.as_str()).map(String::from));
    match (r.status, error.as_deref()) {
        (200, _) | (401, Some("password_required")) => Ok(true),
        (404, Some("not_found")) => Ok(false),
        (404, _) => Err(Failure::new("bad_response")),
        _ => Err(failure(&r)),
    }
}

/// Deletes a shared link with its delete token. A link the Worker says is already gone (expired or
/// deleted elsewhere) counts as deleted. Any other 404, such as a proxy's page at a wrong address,
/// doesn't: the report may still be public.
pub fn delete_shared(http: &dyn Http, origin: &str, id: &str, token: &str) -> Result<Deleted, Failure> {
    // Ids are the Worker's own (letters and digits); anything else is refused rather than sent.
    if !sane_id(id) {
        return Err(Failure::new("bad_response"));
    }
    let r = http.send("DELETE", &endpoint(origin, &format!("/api/reports/{id}")), &[("authorization", format!("Bearer {token}"))], None).map_err(|()| Failure::new("network"))?;
    match r.status {
        204 => Ok(Deleted::Now),
        404 if read_json(&r).is_some_and(|b| b.get("error").and_then(|e| e.as_str()) == Some("not_found")) => Ok(Deleted::AlreadyGone),
        404 => Err(Failure::new("bad_response")),
        _ => Err(failure(&r)),
    }
}

/// A failure as words: an app string key and, for a rate limit, the minutes to wait.
pub fn problem(f: &Failure) -> (&'static str, Option<i64>) {
    match f.error {
        "network" => ("share.e.network", None),
        "rate_limited" => match f.retry_after {
            Some(s) => ("share.e.rate_limited_in", Some((s.div_ceil(60)).max(1) as i64)),
            None => ("share.e.rate_limited", None),
        },
        "too_large" => ("share.e.too_large", None),
        "password_too_short" => ("share.e.password_too_short", None),
        "password_too_long" => ("share.e.password_too_long", None),
        _ => ("share.e.other", None),
    }
}

/// The real transport: HTTPS with Windows' own TLS (SChannel) and certificate store, Windows'
/// proxy settings, and a 20-second limit.
pub struct RealHttp(ureq::Agent);

impl Default for RealHttp {
    fn default() -> Self {
        use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
        let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
        let config = ureq::Agent::config_builder().tls_config(tls).timeout_global(Some(std::time::Duration::from_secs(20))).http_status_as_error(false).build();
        RealHttp(config.into())
    }
}

impl Http for RealHttp {
    fn send(&self, method: &str, url: &str, headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()> {
        let mut request = ureq::http::Request::builder().method(method).uri(url);
        for (k, v) in headers {
            request = request.header(*k, v);
        }
        let request = request.body(body.unwrap_or_default()).map_err(|_| ())?;
        let mut response = self.0.run(request).map_err(|_| ())?;
        let status = response.status().as_u16();
        let headers = response.headers().iter().filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_lowercase(), v.to_string()))).collect();
        // The Worker's answers are small; a runaway body is cut at 1 MB.
        let body = response.body_mut().with_config().limit(1_048_576).read_to_vec().map_err(|_| ())?;
        Ok(Response { status, headers, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_qr_code_reads_back_as_the_link() {
        let url = "https://fettle.page/r/wpgckxjm0upqsgxrp";
        let qr = qr_code(url).expect("a QR code");
        // Draw the path's squares into a picture, 8 px a module, and read it as a phone would.
        let scale = 8usize;
        let side = qr.size as usize * scale;
        let mut px = vec![255u8; side * side];
        for (x, y, w) in qr.runs() {
            for yy in y as usize * scale..(y as usize + 1) * scale {
                for xx in x as usize * scale..(x as usize + w as usize) * scale {
                    px[yy * side + xx] = 0;
                }
            }
        }
        let mut img = rqrr::PreparedImage::prepare_from_greyscale(side, side, |x, y| px[y * side + x]);
        let grids = img.detect_grids();
        assert_eq!(grids.len(), 1);
        let (_, content) = grids[0].decode().expect("it decodes");
        assert_eq!(content, url);
        assert!(qr.path.starts_with('M') && qr.path.contains('Z'), "a Slint path");
    }
    use fettle_facts::Facts;
    use std::sync::Mutex;

    const ORIGIN: &str = "https://fettle.page";

    fn report() -> Report {
        Report { schema_version: 1, app_version: "0.1.0".into(), engine_version: None, collected_at: "2026-10-07T21:00:00Z".into(), elevated: false, facts: Facts::default() }
    }

    fn created() -> serde_json::Value {
        serde_json::json!({ "id": "m0123456789abcdefg", "url": "https://fettle.page/r/m0123456789abcdefg", "deleteUrl": "https://fettle.page/r/m0123456789abcdefg/delete#tok", "deleteToken": "tok", "expiresAt": "2026-11-06T21:00:00.000Z", "locked": false })
    }

    #[derive(Debug, Clone)]
    struct Call {
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<serde_json::Value>,
    }

    /// Answers every request the same way and records what was asked.
    struct Fake {
        answer: Box<dyn Fn() -> Result<Response, ()> + Send + Sync>,
        calls: Mutex<Vec<Call>>,
    }

    impl Fake {
        fn new(answer: impl Fn() -> Result<Response, ()> + Send + Sync + 'static) -> Fake {
            Fake { answer: Box::new(answer), calls: Mutex::new(Vec::new()) }
        }
        fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl Http for Fake {
        fn send(&self, method: &str, url: &str, headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()> {
            self.calls.lock().unwrap().push(Call {
                method: method.into(),
                url: url.into(),
                headers: headers.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
                body: body.map(|b| serde_json::from_slice(&b).unwrap()),
            });
            (self.answer)()
        }
    }

    fn json(status: u16, body: serde_json::Value) -> Result<Response, ()> {
        Ok(Response { status, headers: HashMap::from([("content-type".into(), "application/json".into())]), body: body.to_string().into_bytes() })
    }

    #[test]
    fn posts_the_envelope_as_json_and_returns_the_links() {
        let f = Fake::new(|| json(201, created()));
        let shared = share_report(&f, ORIGIN, &report(), Some(Expiry::Month), None).expect("shared");
        assert_eq!(serde_json::to_value(&shared).unwrap(), created());
        let call = &f.calls()[0];
        assert_eq!((call.method.as_str(), call.url.as_str()), ("POST", "https://fettle.page/api/reports"));
        assert!(call.headers.contains(&("content-type".into(), "application/json".into())));
        assert_eq!(call.body, Some(serde_json::json!({ "report": serde_json::to_value(report()).unwrap(), "expires": "30d" })));
    }

    // A check remembered from an older version could hold text over today's limits: it's shortened
    // before upload instead of being refused by the server.
    #[test]
    fn shortens_overlong_text_to_the_upload_limits() {
        let f = Fake::new(|| json(201, created()));
        let long = Report { app_version: "9".repeat(500), ..report() };
        let _ = share_report(&f, ORIGIN, &long, None, None);
        let sent = f.calls()[0].body.as_ref().unwrap()["report"]["app_version"].as_str().unwrap().to_string();
        assert_eq!(sent.len(), fettle_facts::limits::SHORT);
    }

    #[test]
    fn sends_the_password_only_when_there_is_one() {
        let f = Fake::new(|| json(201, serde_json::json!({ "locked": true }).as_object().map(|_| created()).unwrap()));
        let _ = share_report(&f, ORIGIN, &report(), None, Some("hunter22"));
        let _ = share_report(&f, ORIGIN, &report(), None, Some(""));
        assert_eq!(f.calls()[0].body.as_ref().unwrap()["password"], "hunter22");
        assert!(f.calls()[1].body.as_ref().unwrap().get("password").is_none());
    }

    #[test]
    fn refuses_a_short_or_long_password_before_sending_anything() {
        let f = Fake::new(|| json(201, created()));
        assert_eq!(share_report(&f, ORIGIN, &report(), None, Some("short")).unwrap_err().error, "password_too_short");
        assert_eq!(share_report(&f, ORIGIN, &report(), None, Some(&"x".repeat(129))).unwrap_err().error, "password_too_long");
        // Counted as JavaScript does: an emoji is two.
        assert_eq!(share_report(&f, ORIGIN, &report(), None, Some("🔒🔒🔒")).unwrap_err().error, "password_too_short");
        assert!(f.calls().is_empty());
    }

    /// A report with `n` startup apps of 100 characters and `n` disks with 128-character models, and
    /// the size of its upload once fitted to the limits (each list keeps at most 256 entries).
    fn with_apps_and_disks(n: usize) -> (Report, usize) {
        let ok = || fettle_facts::Check { status: fettle_facts::CheckStatus::Ok, detail: None };
        let item = fettle_facts::StartupItem { name: Some("x".repeat(100)), location: None, enabled: Some(true) };
        let startup = fettle_facts::Section { check: ok(), data: Some(fettle_facts::Startup { items: vec![item; n] }) };
        let disk = fettle_facts::Disk { model: Some("y".repeat(128)), ..Default::default() };
        let storage = fettle_facts::Section { check: ok(), data: Some(fettle_facts::Storage { disks: vec![disk; n], volumes: vec![] }) };
        let r = Report { facts: Facts { startup: Some(startup), storage: Some(storage), ..Facts::default() }, ..report() };
        let mut sent = r.clone();
        fettle_facts::fit::fit_limits(&mut sent);
        let size = serde_json::json!({ "report": sent }).to_string().len();
        (r, size)
    }

    #[test]
    fn refuses_a_report_over_64_kib_before_sending_it() {
        let f = Fake::new(|| json(201, created()));
        // Long text and long lists are cut to the limits first, so it takes two full lists to pass 64 KiB.
        let (big, size) = with_apps_and_disks(256);
        assert!(size > 65_536 && size < 262_144, "over the limit, under the old one: {size}");
        assert_eq!(share_report(&f, ORIGIN, &big, None, None).unwrap_err().error, "too_large");
        assert!(f.calls().is_empty());
    }

    #[test]
    fn sends_a_report_just_under_64_kib() {
        let f = Fake::new(|| json(201, created()));
        let (report, size) = with_apps_and_disks(170);
        assert!(size > 50_000 && size < 65_536, "{size}");
        assert!(share_report(&f, ORIGIN, &report, None, None).is_ok());
        assert_eq!(f.calls().len(), 1);
    }

    #[test]
    fn passes_on_the_servers_error_code_and_the_wait_after_a_rate_limit() {
        let limited = Fake::new(|| {
            let mut r = json(429, serde_json::json!({ "error": "rate_limited" }))?;
            r.headers.insert("retry-after".into(), "60".into());
            Ok(r)
        });
        assert_eq!(share_report(&limited, ORIGIN, &report(), None, None).unwrap_err(), Failure { error: "rate_limited", retry_after: Some(60), detail: None });
        let invalid = Fake::new(|| json(400, serde_json::json!({ "error": "invalid_report", "detail": "facts: bad" })));
        assert_eq!(share_report(&invalid, ORIGIN, &report(), None, None).unwrap_err(), Failure { error: "invalid_report", retry_after: None, detail: Some("facts: bad".into()) });
    }

    #[test]
    fn reports_no_connection_and_odd_answers_without_panicking() {
        let offline = Fake::new(|| Err(()));
        assert_eq!(share_report(&offline, ORIGIN, &report(), None, None).unwrap_err().error, "network");
        let html = Fake::new(|| Ok(Response { status: 502, headers: HashMap::new(), body: b"<h1>Bad gateway</h1>".to_vec() }));
        assert_eq!(share_report(&html, ORIGIN, &report(), None, None).unwrap_err().error, "bad_response");
        let partial = Fake::new(|| json(201, serde_json::json!({ "id": "x" })));
        assert_eq!(share_report(&partial, ORIGIN, &report(), None, None).unwrap_err().error, "bad_response");
    }

    #[test]
    fn accepts_an_origin_with_a_trailing_slash() {
        let f = Fake::new(|| json(201, created()));
        let _ = share_report(&f, "https://fettle.page/", &report(), None, None);
        assert_eq!(f.calls()[0].url, "https://fettle.page/api/reports");
    }

    #[test]
    fn sends_the_delete_token_as_a_bearer_token() {
        let f = Fake::new(|| Ok(Response { status: 204, headers: HashMap::new(), body: vec![] }));
        assert_eq!(delete_shared(&f, ORIGIN, "m0123456789abcdefg", "tok"), Ok(Deleted::Now));
        let call = &f.calls()[0];
        assert_eq!((call.method.as_str(), call.url.as_str()), ("DELETE", "https://fettle.page/api/reports/m0123456789abcdefg"));
        assert!(call.headers.contains(&("authorization".into(), "Bearer tok".into())));
    }

    #[test]
    fn treats_a_link_that_is_already_gone_as_deleted_and_says_so() {
        let f = Fake::new(|| json(404, serde_json::json!({ "error": "not_found" })));
        assert_eq!(delete_shared(&f, ORIGIN, "id", "tok"), Ok(Deleted::AlreadyGone));
    }

    #[test]
    fn asks_the_server_whether_a_link_still_exists() {
        let open = Fake::new(|| json(200, serde_json::json!({ "report": {}, "expiresAt": "x" })));
        assert_eq!(link_exists(&open, ORIGIN, "m0123456789abcdefg"), Ok(true));
        let call = &open.calls()[0];
        assert_eq!((call.method.as_str(), call.url.as_str()), ("GET", "https://fettle.page/api/reports/m0123456789abcdefg"));
        assert!(call.headers.is_empty() && call.body.is_none(), "a plain GET: no token, nothing sent");
        // Locked links exist too: the server only wants the password to show them.
        let locked = Fake::new(|| json(401, serde_json::json!({ "error": "password_required" })));
        assert_eq!(link_exists(&locked, ORIGIN, "id"), Ok(true));
        let gone = Fake::new(|| json(404, serde_json::json!({ "error": "not_found" })));
        assert_eq!(link_exists(&gone, ORIGIN, "id"), Ok(false));
        // Anything else is no answer, never "gone".
        let html = Fake::new(|| Ok(Response { status: 404, headers: HashMap::from([("content-type".into(), "text/html".into())]), body: b"<h1>Not Found</h1>".to_vec() }));
        assert!(link_exists(&html, ORIGIN, "id").is_err());
        assert_eq!(link_exists(&Fake::new(|| Err(())), ORIGIN, "id").unwrap_err().error, "network");
        assert!(link_exists(&open, ORIGIN, "../etc").is_err(), "only the server's own ids are asked about");
    }

    #[test]
    fn does_not_take_someone_elses_404_page_for_a_deleted_link() {
        let f = Fake::new(|| Ok(Response { status: 404, headers: HashMap::from([("content-type".into(), "text/html".into())]), body: b"<h1>Not Found</h1>".to_vec() }));
        assert_eq!(delete_shared(&f, ORIGIN, "id", "tok").unwrap_err().error, "bad_response");
    }

    #[test]
    fn reports_a_wrong_token_and_a_lost_connection() {
        let f = Fake::new(|| json(403, serde_json::json!({ "error": "forbidden" })));
        assert_eq!(delete_shared(&f, ORIGIN, "id", "tok").unwrap_err().error, "forbidden");
        let offline = Fake::new(|| Err(()));
        assert_eq!(delete_shared(&offline, ORIGIN, "id", "tok").unwrap_err().error, "network");
    }

    #[test]
    fn failures_read_as_words() {
        assert_eq!(problem(&Failure::new("network")), ("share.e.network", None));
        assert_eq!(problem(&Failure { error: "rate_limited", retry_after: Some(61), detail: None }), ("share.e.rate_limited_in", Some(2)));
        assert_eq!(problem(&Failure { error: "rate_limited", retry_after: Some(5), detail: None }), ("share.e.rate_limited_in", Some(1)));
        assert_eq!(problem(&Failure::new("rate_limited")), ("share.e.rate_limited", None));
        assert_eq!(problem(&Failure::new("too_large")), ("share.e.too_large", None));
        assert_eq!(problem(&Failure::new("password_too_short")), ("share.e.password_too_short", None));
        assert_eq!(problem(&Failure::new("password_too_long")), ("share.e.password_too_long", None));
        assert_eq!(problem(&Failure::new("forbidden")), ("share.e.other", None));
    }

    #[test]
    fn only_a_development_build_can_send_shares_elsewhere() {
        let local = Some("http://localhost:8788".to_string());
        assert_eq!(origin_from(local.clone(), false), "https://fettle.page");
        assert_eq!(origin_from(local, true), "http://localhost:8788");
        assert_eq!(origin_from(None, true), "https://fettle.page");
        assert_eq!(origin_from(None, false), "https://fettle.page");
    }
}

/// Live checks of the real transport, run by hand:
/// `FETTLE_SHARE_ORIGIN=http://localhost:8788 cargo test -p fettle-windows --lib live_ -- --ignored`
#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore]
    fn live_share_and_delete_against_a_local_worker() {
        let origin = std::env::var("FETTLE_SHARE_ORIGIN").expect("FETTLE_SHARE_ORIGIN");
        let report: Report = crate::store::tests::dump_report();
        let report = Report { engine_version: Some(fettle_verdicts::ENGINE_VERSION.into()), ..report };
        let http = RealHttp::default();
        let shared = share_report(&http, &origin, &report, Some(Expiry::Week), Some("hunter2222")).expect("shared");
        assert!(shared.locked);
        assert!(shared.url.contains("/r/"));
        assert!(delete_shared(&http, &origin, &shared.id, &shared.delete_token).is_ok());
        // Deleting again: already gone counts as deleted.
        assert!(delete_shared(&http, &origin, &shared.id, &shared.delete_token).is_ok());
    }

    #[test]
    #[ignore]
    fn live_https_through_windows_tls() {
        let r = RealHttp::default().send("GET", "https://example.com/", &[], None).expect("an HTTPS answer");
        assert_eq!(r.status, 200);
    }
}
