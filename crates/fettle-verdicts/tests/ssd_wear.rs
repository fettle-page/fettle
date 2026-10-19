//! SSD wear: a SATA SSD reports wear through SMART and warns at the same points as an NVMe drive.
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
fn sata_ssd(smart: Value) -> Value {
    json!({ "storage": ok(json!({ "disks": [{ "bus": "sata", "media": "ssd", "model": "Crucial MX500", "smart": smart }] })) })
}

// 1. A SATA SSD reports its wear through SMART; it warns at the same points as an NVMe drive.
#[test]
fn a_worn_sata_ssd_warns_like_an_nvme_one() {
    let wearing = judge(sata_ssd(json!({ "life_used_pct": 95 })), "2026-10-11T10:00:00Z", true);
    let l = find(&wearing, "disk-0-wear");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "storage.ssd_wearing"));
    assert_eq!(l.action.as_ref().map(|a| a.key.as_str()), Some("storage.ssd_wearing.action"));

    let worn = judge(sata_ssd(json!({ "life_used_pct": 100 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(find(&worn, "disk-0-wear").level, Level::ActSoon);
    assert_eq!(find(&worn, "disk-0-wear").headline.key, "storage.ssd_worn");

    let fine = judge(sata_ssd(json!({ "life_used_pct": 40 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(find(&fine, "disk-0").headline.key, "storage.disk_fine_wear");
}

#[test]
fn nvme_wear_keeps_its_points_under_the_shared_name() {
    let nvme = json!({ "storage": ok(json!({ "disks": [{ "bus": "nvme", "media": "ssd", "model": "Samsung 980", "nvme": { "percentage_used": 85 } }] })) });
    let r = judge(nvme, "2026-10-11T10:00:00Z", true);
    let l = find(&r, "disk-0-wear");
    assert_eq!((l.level, l.headline.key.as_str()), (Level::KeepAnEye, "storage.ssd_wearing"));
}

// On some hard drives the SMART attribute read as wear means something else (202: data address mark
// errors), so only an SSD's value counts as wear.
#[test]
fn a_hard_drive_smart_value_is_never_read_as_wear() {
    let hdd = |smart: Value| json!({ "storage": ok(json!({ "disks": [{ "bus": "sata", "media": "hdd", "model": "ST2000DM008", "smart": smart }] })) });
    let r = judge(hdd(json!({ "life_used_pct": 100, "reallocated_sectors": 0, "pending_sectors": 0 })), "2026-10-11T10:00:00Z", true);
    assert!(lines(&r).iter().all(|l| l.id != "disk-0-wear"), "no wear line for a hard drive");
    assert_eq!(find(&r, "disk-0").headline.key, "storage.disk_fine");

    // With nothing else read, the drive's health is still unknown, not fine.
    let only = judge(hdd(json!({ "life_used_pct": 100 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(find(&only, "disk-0-health").level, Level::CouldntCheck);
}

// Some SSDs report health but no wear (a Verbatim Vi550 S3 gave only reallocated sectors and its
// temperature): the line says the wear is missing instead of a bare "looks healthy". Wear means
// nothing on a hard drive, and a drive of unknown type isn't called an SSD.
#[test]
fn an_ssd_with_no_wear_value_says_so() {
    let disk = |media: &str, smart: Value| json!({ "storage": ok(json!({ "disks": [{ "bus": "sata", "media": media, "model": "Vi550 S3", "smart": smart }] })) });
    let key = |r: &Receipt| {
        let l = find(r, "disk-0");
        assert_eq!(l.level, Level::Fine);
        l.headline.key.clone()
    };

    let no_wear = judge(disk("ssd", json!({ "reallocated_sectors": 0, "temperature_c": 35 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(key(&no_wear), "storage.disk_fine_no_wear");

    let wear = judge(disk("ssd", json!({ "reallocated_sectors": 0, "temperature_c": 35, "life_used_pct": 3 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(key(&wear), "storage.disk_fine_wear");

    let hdd = judge(disk("hdd", json!({ "reallocated_sectors": 0 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(key(&hdd), "storage.disk_fine");

    let unknown = judge(disk("unknown", json!({ "reallocated_sectors": 0 })), "2026-10-11T10:00:00Z", true);
    assert_eq!(key(&unknown), "storage.disk_fine");
}

// NVMe critical-warning bit 1 only says the drive is hot right now; the hot-disk line covers that.
#[test]
fn a_hot_nvme_drive_is_not_called_failing() {
    let nvme = |warning: u8| json!({ "storage": ok(json!({ "disks": [{ "bus": "nvme", "media": "ssd", "model": "Samsung 980", "nvme": { "critical_warning": warning, "percentage_used": 10, "temperature_c": 75 } }] })) });
    let hot = judge(nvme(0x02), "2026-10-11T10:00:00Z", true);
    assert!(lines(&hot).iter().all(|l| l.id != "disk-0-critical"), "temperature alone isn't a failing drive");
    assert_eq!(find(&hot, "disk-0-temp").level, Level::KeepAnEye);

    let failing = judge(nvme(0x02 | 0x04), "2026-10-11T10:00:00Z", true);
    assert_eq!(find(&failing, "disk-0-critical").level, Level::ActSoon);
}
