use super::common::{Action, couldnt_check, int, line, plural, section_problem};
use crate::thresholds::{DISK_TEMP_KEEP_AN_EYE_C, FREE_SPACE_ENOUGH_BYTES, FREE_SPACE_KEEP_AN_EYE, HDD_TEMP_KEEP_AN_EYE_C, SMALL_VOLUME_BYTES, SSD_WEAR_ACT_SOON, SSD_WEAR_KEEP_AN_EYE};
use crate::{Level, Line, Param, SectionId, SectionVerdict};
use fettle_facts::{Bus, Disk, Media, NvmeHealth, Section, SmartSummary, Storage};

pub fn storage_rules(s: Option<&Section<Storage>>, elevated: bool) -> SectionVerdict {
    let problem = section_problem(SectionId::Storage, s.map(|s| &s.check));
    let data = s.and_then(|s| s.data.as_ref());
    let Some(data) = data.filter(|_| problem.is_none()) else {
        return SectionVerdict { section: SectionId::Storage, lines: couldnt_check(SectionId::Storage, problem) };
    };

    let mut lines = Vec::new();
    for (i, disk) in data.disks.iter().enumerate() {
        lines.extend(disk_lines(disk, &format!("disk-{i}"), elevated));
    }
    for (i, vol) in data.volumes.iter().enumerate() {
        let size = vol.size_bytes.unwrap_or(0);
        let Some(free) = vol.free_bytes else { continue };
        let system = is_system_drive(vol.letter.as_deref());
        // No size, no percentage to judge. A small partition with a letter (recovery, system) is
        // meant to be nearly full; the system drive is judged whatever its size. A USB drive isn't
        // judged at all (Fettle only checks the drives inside the PC), and a big data drive with
        // plenty left isn't almost full whatever the percentage.
        if size == 0 || (size < SMALL_VOLUME_BYTES && !system) || vol.bus == Some(Bus::Usb) || (!system && free >= FREE_SPACE_ENOUGH_BYTES) {
            continue;
        }
        let ratio = free as f64 / size as f64;
        // Round down, so "9.5% free" never displays as "10% free" next to a warning.
        if ratio < FREE_SPACE_KEEP_AN_EYE {
            let letter = vol.letter.clone().unwrap_or_else(|| "?".into());
            lines.push(line(format!("volume-{i}"), Level::KeepAnEye, "storage.low_space", &[("letter", letter.into()), ("pct", Param::Int((ratio * 100.0).floor() as i64))], Action::Default));
        }
    }
    SectionVerdict { section: SectionId::Storage, lines }
}

/// Whether a volume's letter is the system drive's: "C:", "c:" or "C".
fn is_system_drive(letter: Option<&str>) -> bool {
    letter.map(str::trim).is_some_and(|l| l.eq_ignore_ascii_case("C:") || l.eq_ignore_ascii_case("C"))
}

// "Looks healthy" needs evidence: at least one real health value, not just a temperature.
/// NVMe critical-warning bit 1: the drive is above its temperature threshold right now. That passes
/// once it cools, and the hot-disk line already says it, so it alone doesn't make a failing drive.
const NVME_WARNING_TEMPERATURE: u8 = 0x02;

fn has_nvme_health(n: &NvmeHealth) -> bool {
    n.percentage_used.is_some() || n.critical_warning.is_some() || n.media_errors.is_some() || n.available_spare_pct.is_some()
}
/// `ssd`: whether the life-used value counts. On some hard drives that SMART attribute means
/// something else (202 is data address mark errors there).
fn has_smart_health(m: &SmartSummary, ssd: bool) -> bool {
    m.reallocated_sectors.is_some() || m.pending_sectors.is_some() || m.offline_uncorrectable.is_some() || (ssd && m.life_used_pct.is_some())
}

/// A wear warning for an SSD that has used `used`% of its rated life (NVMe or SATA), or none yet.
fn wear_line(id: &str, model: &Param, used: u8) -> Option<Line> {
    let (level, key) = if used >= SSD_WEAR_ACT_SOON {
        (Level::ActSoon, "storage.ssd_worn")
    } else if used >= SSD_WEAR_KEEP_AN_EYE {
        (Level::KeepAnEye, "storage.ssd_wearing")
    } else {
        return None;
    };
    Some(line(format!("{id}-wear"), level, key, &[("model", model.clone()), ("pct", Param::Int(used.min(100).into()))], Action::Default))
}

/// Model names only hard drives use, for a drive whose type Windows doesn't know (Fettle doesn't
/// read the rotation speed): Seagate's "ST" + digits, WD's "WDC WD" + digits (its SSDs are "WDS…"),
/// HGST, Hitachi and Toshiba's hard-drive lines. Anything else keeps the SSD limit, so an unknown
/// drive is never warned about too soon.
fn hdd_model(model: &str) -> bool {
    let m = model.trim().to_ascii_uppercase();
    let digit_after = |prefix: &str| m.strip_prefix(prefix).is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()));
    digit_after("ST") || digit_after("WDC WD") || ["HGST ", "HITACHI HD", "HITACHI HT", "TOSHIBA DT", "TOSHIBA MQ", "TOSHIBA MG", "TOSHIBA HDW"].iter().any(|p| m.starts_with(p))
}

fn disk_lines(disk: &Disk, id: &str, elevated: bool) -> Vec<Line> {
    let model: Param = disk.model.clone().unwrap_or_else(|| "?".into()).into();
    let mut issues = Vec::new();
    let mut wear_pct: Option<u8> = None;
    let ssd = disk.media == Some(Media::Ssd);

    if let Some(n) = disk.nvme.as_ref().filter(|n| has_nvme_health(n)) {
        // The drive's own verdict: any critical-warning bit, or spare capacity below its threshold.
        let spare_low = matches!((n.available_spare_pct, n.available_spare_threshold_pct), (Some(s), Some(t)) if s < t);
        if n.critical_warning.unwrap_or(0) & !NVME_WARNING_TEMPERATURE != 0 || spare_low {
            issues.push(line(format!("{id}-critical"), Level::ActSoon, "storage.nvme_critical", &[("model", model.clone())], Action::Default));
        }
        if let Some(used) = n.percentage_used {
            wear_pct = Some(used.min(100));
            issues.extend(wear_line(id, &model, used));
        }
        if let Some(errors) = n.media_errors.filter(|e| *e > 0) {
            issues.push(line(format!("{id}-media"), Level::KeepAnEye, &plural("storage.media_errors", errors), &[("model", model.clone()), ("count", int(errors))], Action::Key("storage.media_errors.action")));
        }
    } else if let Some(m) = disk.smart.as_ref().filter(|m| has_smart_health(m, ssd)) {
        if let Some(pending) = m.pending_sectors.filter(|p| *p > 0) {
            // Crucial MX500 firmware is widely reported to set one pending sector and clear it again
            // by itself, so one on an SSD is worth watching; more, or any on a hard drive, is urgent.
            let (level, action) = if ssd && pending == 1 { (Level::KeepAnEye, "storage.media_errors.action") } else { (Level::ActSoon, "storage.pending_sectors.action") };
            issues.push(line(format!("{id}-pending"), level, &plural("storage.pending_sectors", pending), &[("model", model.clone()), ("count", int(pending))], Action::Key(action)));
        }
        // Sectors the drive's own scan couldn't read or repair (SMART 198): as serious as pending ones.
        if let Some(bad) = m.offline_uncorrectable.filter(|u| *u > 0) {
            issues.push(line(format!("{id}-uncorrectable"), Level::ActSoon, &plural("storage.uncorrectable_sectors", bad), &[("model", model.clone()), ("count", int(bad))], Action::Key("storage.pending_sectors.action")));
        }
        if let Some(realloc) = m.reallocated_sectors.filter(|r| *r > 0) {
            issues.push(line(format!("{id}-realloc"), Level::KeepAnEye, &plural("storage.reallocated_sectors", realloc), &[("model", model.clone()), ("count", int(realloc))], Action::Key("storage.reallocated_sectors.action")));
        }
        // SATA SSDs report wear through vendor SMART attributes: the same points as NVMe.
        if let Some(life) = m.life_used_pct.filter(|_| ssd) {
            wear_pct = Some(life.min(100));
            issues.extend(wear_line(id, &model, life));
        }
    } else if disk.bus == Some(Bus::Sata) && !elevated {
        // No "needs administrator access" detail: the headline already says so.
        return vec![line(format!("{id}-health"), Level::CouldntCheck, "storage.deep_check", &[("model", model)], Action::Key("storage.deep_check.action"))];
    } else if disk.bus == Some(Bus::Usb) {
        // USB sticks and most external enclosures don't pass health data through. Saying so beats a
        // "couldn't check" that reads like a fault (seen with a plugged-in stick on a laptop).
        return vec![line(format!("{id}-health"), Level::Info, "storage.usb_not_checked", &[("model", model)], Action::Default)];
    } else {
        return vec![line(format!("{id}-health"), Level::CouldntCheck, "storage.no_health", &[("model", model)], Action::Default)];
    }

    let temp = disk.nvme.as_ref().and_then(|n| n.temperature_c).or_else(|| disk.smart.as_ref().and_then(|m| m.temperature_c));
    let hdd = disk.media == Some(Media::Hdd) || (disk.media != Some(Media::Ssd) && disk.model.as_deref().is_some_and(hdd_model));
    let limit = if hdd { HDD_TEMP_KEEP_AN_EYE_C } else { DISK_TEMP_KEEP_AN_EYE_C };
    if let Some(t) = temp.filter(|t| *t > limit) {
        issues.push(line(format!("{id}-temp"), Level::KeepAnEye, "storage.disk_hot", &[("model", model.clone()), ("temp", Param::Int(t.into()))], Action::Default));
    }

    if !issues.is_empty() {
        return issues;
    }
    vec![match wear_pct {
        Some(pct) => line(id, Level::Fine, "storage.disk_fine_wear", &[("model", model), ("pct", Param::Int(pct.into()))], Action::Default),
        None => line(id, Level::Fine, "storage.disk_fine", &[("model", model)], Action::Default),
    }]
}
