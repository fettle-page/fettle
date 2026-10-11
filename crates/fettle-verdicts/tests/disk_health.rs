//! Disk health from SMART: sectors the drive couldn't read or repair, and temperatures.
//! Small reports built from JSON, judged at a fixed time.

use fettle_facts::Facts;
use fettle_verdicts::{Level, Line, Receipt, evaluate, fmt};
use serde_json::{Value, json};

fn judge(facts: Value) -> Receipt {
    let facts: Facts = serde_json::from_value(facts).expect("facts");
    evaluate(&facts, fmt::parse_rfc3339("2026-10-11T10:00:00Z").expect("now"), true)
}
fn lines(r: &Receipt) -> Vec<&Line> {
    r.sections.iter().flat_map(|s| &s.lines).collect()
}
fn ids(r: &Receipt) -> Vec<&str> {
    lines(r).iter().map(|l| l.id.as_str()).collect()
}
fn find<'a>(r: &'a Receipt, id: &str) -> &'a Line {
    lines(r).into_iter().find(|l| l.id == id).unwrap_or_else(|| panic!("no line {id}: {:?}", ids(r)))
}
fn disk(media: &str, smart: Value) -> Value {
    json!({ "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": media, "model": "ST2000DM008", "smart": smart }] } } })
}

// SMART 198 counts sectors the drive found it can't read or repair: a drive on its way out.
#[test]
fn uncorrectable_sectors_mean_act_soon() {
    let r = judge(disk("hdd", json!({ "reallocated_sectors": 0, "pending_sectors": 0, "offline_uncorrectable": 24 })));
    let l = find(&r, "disk-0-uncorrectable");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::ActSoon, "storage.uncorrectable_sectors"));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("storage.pending_sectors.action"));
    assert!(!ids(&r).contains(&"disk-0"), "no healthy line: {:?}", ids(&r));

    let one = judge(disk("hdd", json!({ "offline_uncorrectable": 1 })));
    assert_eq!(find(&one, "disk-0-uncorrectable").headline.key, "storage.uncorrectable_sectors_one");
}

#[test]
fn no_uncorrectable_sectors_is_healthy() {
    let r = judge(disk("hdd", json!({ "reallocated_sectors": 0, "pending_sectors": 0, "offline_uncorrectable": 0 })));
    assert_eq!(find(&r, "disk-0").headline.key, "storage.disk_fine");
}

// Hard drives are rated to about 60 °C, SSDs to 70 °C or more.
#[test]
fn a_hard_drive_runs_hot_sooner_than_an_ssd() {
    let hot = |media: &str, t: i32| lines(&judge(disk(media, json!({ "reallocated_sectors": 0, "temperature_c": t })))).iter().any(|l| l.headline.key == "storage.disk_hot");
    assert!(!hot("hdd", 55));
    assert!(hot("hdd", 56));
    assert!(!hot("ssd", 65));
    assert!(!hot("ssd", 70));
    assert!(hot("ssd", 71));
    // Without a known type, the model decides: this one (ST2000DM008) is a Seagate hard drive.
    assert!(hot("unknown", 60));
}

// Two warnings whose next steps read the same, word for word, list that step once.
#[test]
fn a_next_step_with_the_same_words_is_listed_once() {
    let r = judge(json!({ "storage": { "check": { "status": "ok" }, "data": { "disks": [
        { "bus": "nvme", "media": "ssd", "model": "WD SN770", "nvme": { "percentage_used": 3, "media_errors": 2 } },
        { "bus": "sata", "media": "hdd", "model": "ST2000DM008", "smart": { "reallocated_sectors": 8 } }
    ] } } }));
    let keys: Vec<&str> = r.summary.actions.iter().map(|a| a.key.as_str()).collect();
    assert_eq!(keys, ["storage.media_errors.action"]);
}

// Windows sometimes reports a drive's type as unknown. A model name that only hard drives use
// gets the hard-drive limit; anything else keeps the SSD one, so nothing warns wrongly.
#[test]
fn a_hard_drive_of_unknown_type_is_known_by_its_model() {
    let hot = |model: &str, t: i32| {
        let facts = json!({ "storage": { "check": { "status": "ok" }, "data": { "disks": [{ "bus": "sata", "media": "unknown", "model": model, "smart": { "reallocated_sectors": 0, "temperature_c": t } }] } } });
        lines(&judge(facts)).iter().any(|l| l.headline.key == "storage.disk_hot")
    };
    for hdd in ["ST2000DM008-2FR102", "WDC WD10EZEX-08WN4A0", "HGST HTS721010A9E630", "TOSHIBA DT01ACA100", "Hitachi HDS721010CLA332"] {
        assert!(hot(hdd, 60), "{hdd}");
    }
    for ssd in ["WDC WDS500G2B0A", "Samsung SSD 860 EVO 1TB", "CT1000MX500SSD1", "Some Drive"] {
        assert!(!hot(ssd, 60), "{ssd}");
    }
}

// Crucial MX500 firmware is widely reported to set one pending sector and clear it again on its
// own, so one on an SSD is worth watching rather than an alarm; more, or any on a hard drive, is.
#[test]
fn one_pending_sector_on_an_ssd_is_worth_watching() {
    let pending = |media: &str, n: u64| {
        let r = judge(disk(media, json!({ "pending_sectors": n })));
        let l = find(&r, "disk-0-pending");
        (l.level, l.action.as_ref().map(|a| a.key.clone()).unwrap_or_default())
    };
    assert_eq!(pending("ssd", 1), (Level::KeepAnEye, "storage.media_errors.action".to_string()));
    assert_eq!(pending("ssd", 2).0, Level::ActSoon);
    assert_eq!(pending("hdd", 1).0, Level::ActSoon);
    assert_eq!(pending("unknown", 1).0, Level::ActSoon);
}
