use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// One data source. Runs on its own thread, so it must own everything it uses.
pub type Collector = Box<dyn FnOnce() -> Result<Value, String> + Send + 'static>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SourceResult {
    pub ok: bool,
    pub ms: u64,
    pub data: Option<Value>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Dump {
    pub spike_version: String,
    pub label: String,
    pub collected_at_unix: u64,
    pub elevated: bool,
    pub private_included: bool,
    pub total_ms: u64,
    pub sources: BTreeMap<String, SourceResult>,
}

/// Runs one collector on its own thread. A panic or a timeout becomes an
/// error result, so one bad source can never stop the whole run.
pub fn run_source(f: Collector, timeout: Duration) -> SourceResult {
    let start = Instant::now();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|payload| {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            Err(format!("panicked: {msg}"))
        });
        let _ = tx.send(outcome);
    });
    let outcome = match rx.recv_timeout(timeout) {
        Ok(outcome) => outcome,
        Err(_) => Err(format!("timed out after {} ms", timeout.as_millis())),
    };
    let ms = start.elapsed().as_millis() as u64;
    match outcome {
        Ok(data) => SourceResult { ok: true, ms, data: Some(data), error: None },
        Err(error) => SourceResult { ok: false, ms, data: None, error: Some(error) },
    }
}

/// Turns a user-typed machine label into something safe for a file name.
pub fn sanitize_label(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.to_lowercase().chars() {
        let c = if c == ' ' || c == '_' { '-' } else { c };
        if c.is_ascii_lowercase() || c.is_ascii_digit() || (c == '-' && !out.ends_with('-')) {
            out.push(c);
        }
    }
    let out: String = out.trim_matches('-').chars().take(32).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "unlabelled".into() } else { out }
}

/// Writes the dump as `<label>-<admin|user>-<unix>.json`, creating `dir` if needed.
pub fn write_dump(dir: &Path, dump: &Dump) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    let mode = if dump.elevated { "admin" } else { "user" };
    let path = dir.join(format!("{}-{}-{}.json", dump.label, mode, dump.collected_at_unix));
    let json = serde_json::to_string_pretty(dump).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("can't write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn boxed(f: impl FnOnce() -> Result<Value, String> + Send + 'static) -> Collector {
        Box::new(f)
    }

    fn dump(elevated: bool) -> Dump {
        Dump {
            spike_version: "0.1.0".into(),
            label: "desk".into(),
            collected_at_unix: 1_700_000_000,
            elevated,
            private_included: false,
            total_ms: 0,
            sources: BTreeMap::new(),
        }
    }

    #[test]
    fn run_source_ok_keeps_empty_data() {
        let r = run_source(boxed(|| Ok(json!([]))), Duration::from_secs(1));
        assert!(r.ok);
        assert_eq!(r.data, Some(json!([])));
        assert_eq!(r.error, None);
    }

    #[test]
    fn run_source_err_keeps_message_verbatim() {
        let r = run_source(boxed(|| Err("0x80041003 access denied".into())), Duration::from_secs(1));
        assert!(!r.ok);
        assert_eq!(r.data, None);
        assert_eq!(r.error.as_deref(), Some("0x80041003 access denied"));
    }

    #[test]
    fn run_source_catches_panic() {
        let r = run_source(boxed(|| panic!("boom")), Duration::from_secs(1));
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("panicked: boom"));
    }

    #[test]
    fn run_source_times_out() {
        let r = run_source(
            boxed(|| {
                std::thread::sleep(Duration::from_secs(5));
                Ok(json!(null))
            }),
            Duration::from_millis(100),
        );
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("timed out after 100 ms"));
        assert!(r.ms < 1000);
    }

    #[test]
    fn sanitize_label_cases() {
        assert_eq!(sanitize_label("Work Laptop!"), "work-laptop");
        assert_eq!(sanitize_label("../../x"), "x");
        assert_eq!(sanitize_label("old_pc  2"), "old-pc-2");
        assert_eq!(sanitize_label(""), "unlabelled");
    }

    #[test]
    fn write_dump_creates_missing_dir_and_names_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_dump(&tmp.path().join("a/b"), &dump(false)).unwrap();
        assert_eq!(path.file_name().unwrap(), "desk-user-1700000000.json");
        let back: Dump = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back.label, "desk");
    }

    #[test]
    fn write_dump_names_admin_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_dump(tmp.path(), &dump(true)).unwrap();
        assert_eq!(path.file_name().unwrap(), "desk-admin-1700000000.json");
    }

    #[test]
    fn write_dump_errors_when_dir_is_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("not-a-dir");
        std::fs::write(&file, "x").unwrap();
        assert!(write_dump(&file, &dump(false)).is_err());
    }
}
