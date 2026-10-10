use super::common::{Action, couldnt_check, int, line, plural, section_problem};
use crate::thresholds::{DISK_TEMP_KEEP_AN_EYE_C, FREE_SPACE_KEEP_AN_EYE, NVME_WEAR_ACT_SOON, NVME_WEAR_KEEP_AN_EYE};
use crate::{Level, Line, Param, SectionId, SectionVerdict};
use fettle_facts::{Bus, Disk, NvmeHealth, Section, SmartSummary, Storage};

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
        if size == 0 {
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

// "Looks healthy" needs evidence: at least one real health value, not just a temperature.
fn has_nvme_health(n: &NvmeHealth) -> bool {
    n.percentage_used.is_some() || n.critical_warning.is_some() || n.media_errors.is_some() || n.available_spare_pct.is_some()
}
fn has_smart_health(m: &SmartSummary) -> bool {
    m.reallocated_sectors.is_some() || m.pending_sectors.is_some() || m.offline_uncorrectable.is_some() || m.life_used_pct.is_some()
}

fn disk_lines(disk: &Disk, id: &str, elevated: bool) -> Vec<Line> {
    let model: Param = disk.model.clone().unwrap_or_else(|| "?".into()).into();
    let mut issues = Vec::new();
    let mut wear_pct: Option<u8> = None;

    if let Some(n) = disk.nvme.as_ref().filter(|n| has_nvme_health(n)) {
        // The drive's own verdict: any critical-warning bit, or spare capacity below its threshold.
        let spare_low = matches!((n.available_spare_pct, n.available_spare_threshold_pct), (Some(s), Some(t)) if s < t);
        if n.critical_warning.unwrap_or(0) != 0 || spare_low {
            issues.push(line(format!("{id}-critical"), Level::ActSoon, "storage.nvme_critical", &[("model", model.clone())], Action::Default));
        }
        if let Some(used) = n.percentage_used {
            let wear = used.min(100);
            wear_pct = Some(wear);
            if used >= NVME_WEAR_ACT_SOON {
                issues.push(line(format!("{id}-wear"), Level::ActSoon, "storage.nvme_worn", &[("model", model.clone()), ("pct", Param::Int(wear.into()))], Action::Default));
            } else if used >= NVME_WEAR_KEEP_AN_EYE {
                issues.push(line(format!("{id}-wear"), Level::KeepAnEye, "storage.nvme_wearing", &[("model", model.clone()), ("pct", Param::Int(wear.into()))], Action::Default));
            }
        }
        if let Some(errors) = n.media_errors.filter(|e| *e > 0) {
            issues.push(line(format!("{id}-media"), Level::KeepAnEye, &plural("storage.media_errors", errors), &[("model", model.clone()), ("count", int(errors))], Action::Key("storage.media_errors.action")));
        }
    } else if let Some(m) = disk.smart.as_ref().filter(|m| has_smart_health(m)) {
        if let Some(pending) = m.pending_sectors.filter(|p| *p > 0) {
            issues.push(line(format!("{id}-pending"), Level::ActSoon, &plural("storage.pending_sectors", pending), &[("model", model.clone()), ("count", int(pending))], Action::Key("storage.pending_sectors.action")));
        }
        if let Some(realloc) = m.reallocated_sectors.filter(|r| *r > 0) {
            issues.push(line(format!("{id}-realloc"), Level::KeepAnEye, &plural("storage.reallocated_sectors", realloc), &[("model", model.clone()), ("count", int(realloc))], Action::Key("storage.reallocated_sectors.action")));
        }
        if let Some(life) = m.life_used_pct {
            wear_pct = Some(life.min(100));
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
    if let Some(t) = temp.filter(|t| *t > DISK_TEMP_KEEP_AN_EYE_C) {
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
