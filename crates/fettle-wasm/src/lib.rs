//! The receipt for the website and the share Worker, as WebAssembly (wasm32-unknown-unknown).
//! A plain C interface with no imports: JSON text in, JSON text out. The caller (packages/brain)
//! writes the report into memory from `alloc`, calls `receipt`, reads the `[ptr, len]` pair it
//! points to, and frees both buffers with `dealloc`. The scan time comes from the report, so
//! nothing here reads a clock.

use fettle_facts::Report;
use fettle_receipt::{ReceiptModel, Zone, markdown, receipt_for};
use fettle_verdicts::{Locale, Receipt};
use serde::Serialize;

#[derive(Serialize)]
struct Out<'a> {
    receipt: &'a Receipt,
    model: &'a ReceiptModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    markdown: Option<String>,
}

fn error(message: impl Into<String>) -> String {
    serde_json::json!({ "error": message.into() }).to_string()
}

/// The receipt and model for a report, or `{"error": …}`. `locale`: 0 English, 1 Italian.
/// `zone_kind`: 0 UTC (server-drawn pages), 1 `offset_minutes` east of UTC.
pub fn run(input: &[u8], locale: u32, zone_kind: u32, offset_minutes: i32, with_markdown: bool) -> String {
    let locale = match locale {
        0 => Locale::En,
        1 => Locale::It,
        other => return error(format!("unknown locale {other}")),
    };
    let report: Report = match serde_json::from_slice(input) {
        Ok(r) => r,
        Err(e) => return error(format!("report: {e}")),
    };
    let zone = if zone_kind == 1 { Zone::Offset(offset_minutes) } else { Zone::Utc };
    match receipt_for(&report, locale, zone) {
        Ok((receipt, model)) => {
            let markdown = with_markdown.then(|| markdown(&model));
            serde_json::to_string(&Out { receipt: &receipt, model: &model, markdown }).unwrap_or_else(|e| error(format!("output: {e}")))
        }
        Err(e) => error(e),
    }
}

/// Room for `len` bytes, for the caller to write into.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees what `alloc` or `receipt` handed out.
///
/// # Safety
/// `ptr` and `cap` must come from `alloc(cap)` or from a `receipt` output pair, once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, cap: usize) {
    unsafe { drop(Vec::from_raw_parts(ptr, 0, cap)) };
}

static mut OUT: [u32; 2] = [0, 0];

/// Runs `run` on the report at `ptr..ptr+len`. Returns a pointer to `[out_ptr, out_len]`, valid
/// until the next call; the caller frees the output with `dealloc(out_ptr, out_len)`.
///
/// # Safety
/// `ptr..ptr+len` must be readable memory the caller wrote (from `alloc`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn receipt(ptr: *const u8, len: usize, locale: u32, zone_kind: u32, offset_minutes: i32, with_markdown: u32) -> *const u32 {
    let input = unsafe { std::slice::from_raw_parts(ptr, len) };
    let output = run(input, locale, zone_kind, offset_minutes, with_markdown != 0).into_bytes();
    // Exactly `len` capacity, so `dealloc(out_ptr, out_len)` frees it whole.
    let output = output.into_boxed_slice();
    let n = output.len();
    let out_ptr = Box::into_raw(output) as *mut u8;
    unsafe {
        OUT = [out_ptr as usize as u32, n as u32];
        &raw const OUT as *const u32
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use serde_json::Value;

    /// The first parity case's report, as the website would send it.
    fn report() -> String {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fettle-receipt/tests/parity/cases");
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        files.sort();
        let case: Value = serde_json::from_str(&std::fs::read_to_string(&files[0]).unwrap()).unwrap();
        case["report"].to_string()
    }
    fn out(s: String) -> Value {
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn a_report_gives_the_receipt_and_model() {
        let v = out(run(report().as_bytes(), 0, 0, 0, true));
        assert!(v["receipt"].is_object() && v["markdown"].is_string(), "{v}");
        assert_eq!(v["model"]["locale"], "en");
        assert!(v["model"]["meta"]["when"].as_str().unwrap().ends_with(" UTC"));
    }

    #[test]
    fn italian_and_an_offset_are_honoured() {
        let v = out(run(report().as_bytes(), 1, 1, 120, false));
        assert_eq!(v["model"]["locale"], "it");
        // The first case was checked at 21:26 UTC: 23:26 two hours east, with no "UTC".
        assert_eq!(v["model"]["meta"]["when"], "06/10/2026 23:26");
        assert!(v.get("markdown").is_none());
    }

    #[test]
    fn bad_json_is_an_error_not_a_panic() {
        let v = out(run(b"{", 0, 0, 0, false));
        assert!(v["error"].as_str().unwrap().starts_with("report: "), "{v}");
    }

    #[test]
    fn an_unknown_locale_is_an_error() {
        assert_eq!(out(run(report().as_bytes(), 7, 0, 0, false))["error"], "unknown locale 7");
    }

    #[test]
    fn an_impossible_scan_time_is_an_error() {
        let bad = report().replace("2026-10-06T21:26:18.000Z", "2026-13-40T99:00:00Z");
        assert!(out(run(bad.as_bytes(), 0, 0, 0, false))["error"].is_string());
    }
}
