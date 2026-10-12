//! Runs every source at once, each on its own thread with a timeout, so one stuck WMI provider
//! can't hold up the scan and one panicking source can't stop it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// One data source. Runs on its own thread, so it must own everything it uses.
pub type Source = Box<dyn FnOnce() -> Result<Value, String> + Send + 'static>;

/// What one source returned: the same shape as an entry in a saved scan (tests/dumps).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SourceResult {
    pub ok: bool,
    pub ms: u64,
    pub data: Option<Value>,
    pub error: Option<String>,
}

/// Every source's result, by source name.
pub type Raw = BTreeMap<String, SourceResult>;

/// Per source: how many of its threads are running, and whether a scan gave up on one of them.
/// A source is skipped only while an abandoned thread is still running; two scans at once both run.
#[derive(Default)]
struct Flight {
    running: usize,
    abandoned: bool,
}

static IN_FLIGHT: Mutex<BTreeMap<String, Flight>> = Mutex::new(BTreeMap::new());

fn in_flight() -> MutexGuard<'static, BTreeMap<String, Flight>> {
    IN_FLIGHT.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Registers a new thread for `name`, or None when an abandoned one is still stuck.
fn start_flight(name: &str) -> Option<()> {
    let mut flights = in_flight();
    let flight = flights.entry(name.to_string()).or_default();
    if flight.abandoned {
        return None;
    }
    flight.running += 1;
    Some(())
}

fn end_flight(name: &str) {
    let mut flights = in_flight();
    if let Some(flight) = flights.get_mut(name) {
        flight.running = flight.running.saturating_sub(1);
        if flight.running == 0 {
            flights.remove(name);
        }
    }
}

fn abandon_flight(name: &str) {
    if let Some(flight) = in_flight().get_mut(name) {
        flight.abandoned = true;
    }
}

/// Starts every source at once and waits at most `timeout` for all of them. A source still running
/// then is reported as timed out; its thread is left to finish on its own, and until it does, later
/// scans skip that source instead of stacking up another stuck thread (the app scans many times).
pub fn run_all(sources: Vec<(String, Source)>, timeout: Duration) -> Raw {
    run_all_with(sources, timeout, |_, _| {})
}

/// `run_all`, calling `on_done` once per source as its result is settled (finished, failed,
/// skipped or timed out), so the app can print the receipt while the scan runs.
pub fn run_all_with(sources: Vec<(String, Source)>, timeout: Duration, on_done: impl Fn(&str, &SourceResult)) -> Raw {
    let start = Instant::now();
    let mut raw = Raw::new();
    let mut names: Vec<String> = Vec::new();
    let (tx, rx) = mpsc::channel();
    for (name, source) in sources {
        if start_flight(&name).is_none() {
            let result = SourceResult { ok: false, ms: 0, data: None, error: Some("still running from the last scan".into()) };
            on_done(&name, &result);
            raw.insert(name, result);
            continue;
        }
        names.push(name.clone());
        let tx = tx.clone();
        thread::spawn(move || {
            let begun = Instant::now();
            let outcome = panic::catch_unwind(AssertUnwindSafe(source)).unwrap_or_else(|payload| Err(format!("panicked: {}", panic_message(&*payload))));
            end_flight(&name);
            let _ = tx.send((name, outcome, begun.elapsed()));
        });
    }
    drop(tx);

    let deadline = start + timeout;
    let expected = raw.len() + names.len();
    while raw.len() < expected {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else { break };
        let Ok((name, outcome, took)) = rx.recv_timeout(left) else { break };
        let ms = took.as_millis() as u64;
        let result = match outcome {
            Ok(data) => SourceResult { ok: true, ms, data: Some(data), error: None },
            Err(error) => SourceResult { ok: false, ms, data: None, error: Some(error) },
        };
        on_done(&name, &result);
        raw.insert(name, result);
    }
    let waited = timeout.as_millis() as u64;
    for name in names {
        if let std::collections::btree_map::Entry::Vacant(slot) = raw.entry(name) {
            abandon_flight(slot.key());
            let result = SourceResult { ok: false, ms: waited, data: None, error: Some(format!("timed out after {waited} ms")) };
            on_done(slot.key(), &result);
            slot.insert(result);
        }
    }
    raw
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Instant;

    fn sleeper(ms: u64) -> Source {
        Box::new(move || {
            sleep(Duration::from_millis(ms));
            Ok(Value::from(ms))
        })
    }

    #[test]
    fn run_all_runs_sources_in_parallel() {
        let start = Instant::now();
        let raw = run_all(vec![("a".into(), sleeper(300)), ("b".into(), sleeper(300)), ("c".into(), sleeper(300))], Duration::from_secs(2));
        assert!(start.elapsed() < Duration::from_millis(600), "took {:?}", start.elapsed());
        assert_eq!(raw.len(), 3);
        assert!(raw.values().all(|r| r.ok && r.data == Some(Value::from(300))));
    }

    #[test]
    fn a_panic_becomes_an_error() {
        let raw = run_all(vec![("boom".into(), Box::new(|| panic!("kaboom")))], Duration::from_secs(1));
        let r = &raw["boom"];
        assert!(!r.ok && r.data.is_none());
        assert_eq!(r.error.as_deref(), Some("panicked: kaboom"));
    }

    #[test]
    fn a_hang_becomes_a_timeout() {
        let start = Instant::now();
        let raw = run_all(vec![("stuck".into(), sleeper(5000)), ("quick".into(), sleeper(10))], Duration::from_millis(200));
        assert!(start.elapsed() < Duration::from_secs(1), "took {:?}", start.elapsed());
        assert_eq!(raw["stuck"].error.as_deref(), Some("timed out after 200 ms"));
        assert!(raw["quick"].ok);
    }

    #[test]
    fn a_source_still_stuck_from_the_last_scan_is_not_started_again() {
        let stuck = || -> Source {
            Box::new(|| {
                sleep(Duration::from_millis(800));
                Ok(Value::Null)
            })
        };
        let first = run_all(vec![("stuck-test".into(), stuck())], Duration::from_millis(50));
        assert_eq!(first["stuck-test"].error.as_deref(), Some("timed out after 50 ms"));
        let start = Instant::now();
        let second = run_all(vec![("stuck-test".into(), stuck())], Duration::from_millis(400));
        assert!(start.elapsed() < Duration::from_millis(100), "waited {:?}", start.elapsed());
        assert_eq!(second["stuck-test"].error.as_deref(), Some("still running from the last scan"));
        // Once it finishes, it runs again.
        sleep(Duration::from_millis(1200));
        assert!(run_all(vec![("stuck-test".into(), sleeper(1))], Duration::from_secs(1))["stuck-test"].ok);
    }

    #[test]
    fn run_all_with_reports_each_source_once_as_it_finishes() {
        let seen = std::sync::Mutex::new(Vec::<(String, bool)>::new());
        let raw = run_all_with(
            vec![
                ("slow-cb".into(), sleeper(150)),
                ("fast-cb".into(), sleeper(10)),
                ("fail-cb".into(), Box::new(|| Err("no".to_string()))),
                ("hang-cb".into(), sleeper(3000)),
            ],
            Duration::from_millis(400),
            |name, r| seen.lock().unwrap().push((name.to_string(), r.ok)),
        );
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 4, "{seen:?}");
        assert!(seen.iter().position(|s| s.0 == "fast-cb").unwrap() < seen.iter().position(|s| s.0 == "slow-cb").unwrap());
        // The timed-out one is reported last, when the scan gives up on it.
        assert_eq!(seen.last().unwrap(), &("hang-cb".to_string(), false));
        assert_eq!(raw.len(), 4);
    }

    #[test]
    fn two_scans_at_once_both_run_their_sources() {
        let scan = || std::thread::spawn(|| run_all(vec![("concurrent-test".into(), sleeper(200))], Duration::from_secs(2)));
        let (a, b) = (scan(), scan());
        assert!(a.join().unwrap()["concurrent-test"].ok);
        assert!(b.join().unwrap()["concurrent-test"].ok);
    }

    #[test]
    fn an_error_is_kept() {
        let raw = run_all(vec![("denied".into(), Box::new(|| Err("0x80041003".to_string())))], Duration::from_secs(1));
        assert_eq!(raw["denied"], SourceResult { ok: false, ms: raw["denied"].ms, data: None, error: Some("0x80041003".into()) });
    }
}
