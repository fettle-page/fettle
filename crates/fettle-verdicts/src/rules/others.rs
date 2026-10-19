// The short rule files: battery, memory, startup and graphics, side by side.

use super::common::{Action, couldnt_check, line, plural, section_problem};
use crate::thresholds::{BATTERY_ACT_SOON, BATTERY_KEEP_AN_EYE, DRIVER_AGE_INFO_MONTHS, STARTUP_ITEMS_KEEP_AN_EYE};
use crate::{Level, Param, SectionId, SectionVerdict, fmt};
use fettle_facts::{Battery, CheckStatus, Graphics, Memory, Section, Startup};

/// `None` when there's no battery (desktops), so the receipt has no battery section at all.
pub fn battery_rules(s: Option<&Section<Battery>>) -> Option<SectionVerdict> {
    let s = s.filter(|s| s.check.status != CheckStatus::NotPresent)?;
    if let Some(problem) = section_problem(SectionId::Battery, Some(&s.check)) {
        return Some(SectionVerdict { section: SectionId::Battery, lines: vec![problem] });
    }
    let design = s.data.as_ref().and_then(|d| d.design_capacity_mwh).unwrap_or(0);
    let full = s.data.as_ref().and_then(|d| d.full_charge_capacity_mwh);
    // A full-charge reading of 0 is broken firmware, not a dead battery.
    let Some(full) = full.filter(|f| *f > 0 && design > 0) else {
        return Some(SectionVerdict { section: SectionId::Battery, lines: vec![line("battery", Level::CouldntCheck, "battery.unknown", &[], Action::Default)] });
    };
    let ratio = f64::from(full) / f64::from(design);
    // New batteries can report more than their design capacity; never show more than 100%.
    // Round down, so 79.5% never displays as "80%" next to a keep-an-eye verdict.
    let pct = ((ratio * 100.0).floor() as i64).min(100);
    let level = if ratio < BATTERY_ACT_SOON {
        Level::ActSoon
    } else if ratio < BATTERY_KEEP_AN_EYE {
        Level::KeepAnEye
    } else {
        Level::Fine
    };
    let key = if level == Level::Fine { "battery.fine" } else { "battery.worn" };
    Some(SectionVerdict { section: SectionId::Battery, lines: vec![line("battery", level, key, &[("pct", Param::Int(pct))], Action::Default)] })
}

// The "probably running below its rated speed" guess waits for a test on a machine with XMP/EXPO
// off: WMI's Speed field didn't behave as assumed.
pub fn memory_rules(s: Option<&Section<Memory>>, laptop: bool) -> SectionVerdict {
    let problem = section_problem(SectionId::Memory, s.map(|s| &s.check));
    let data = s.and_then(|s| s.data.as_ref());
    let modules = data.map(|d| d.modules.as_slice()).unwrap_or(&[]);
    if problem.is_some() || modules.is_empty() {
        return SectionVerdict { section: SectionId::Memory, lines: couldnt_check(SectionId::Memory, problem) };
    }
    let bytes: f64 = modules.iter().map(|m| m.capacity_bytes.unwrap_or(0) as f64).sum();
    let gb = (bytes / 2f64.powi(30)).round() as i64;
    let mut lines = vec![line("memory", Level::Fine, "memory.fine", &[("gb", Param::Int(gb)), ("sticks", Param::Int(modules.len() as i64))], Action::Default)];
    // LPDDR is soldered to the board: there's no second stick to add. Nor is there on many laptops
    // whose firmware counts slots that aren't there (an ASUS X541UJ said 4 for soldered memory plus
    // one slot, and its soldered memory reads as an ordinary stick), so laptops get no tip.
    let soldered = data.is_some_and(Memory::soldered);
    if modules.len() == 1 && !soldered && !laptop && data.and_then(|d| d.slots_total).unwrap_or(0) >= 2 {
        lines.push(line("memory-channels", Level::Info, "memory.single_channel", &[], Action::Default));
    }
    SectionVerdict { section: SectionId::Memory, lines }
}

pub fn startup_rules(s: Option<&Section<Startup>>) -> SectionVerdict {
    let problem = section_problem(SectionId::Startup, s.map(|s| &s.check));
    let Some(data) = s.and_then(|s| s.data.as_ref()).filter(|_| problem.is_none()) else {
        return SectionVerdict { section: SectionId::Startup, lines: couldnt_check(SectionId::Startup, problem) };
    };
    // Items disabled in Task Manager don't start, so they don't count.
    let count = data.items.iter().filter(|i| i.enabled != Some(false)).count();
    let l = if count == 0 {
        line("startup", Level::Fine, "startup.none", &[], Action::Default)
    } else if count > STARTUP_ITEMS_KEEP_AN_EYE {
        line("startup", Level::KeepAnEye, "startup.many", &[("count", Param::Int(count as i64))], Action::Default)
    } else {
        line("startup", Level::Fine, &plural("startup.fine", count as u64), &[("count", Param::Int(count as i64))], Action::Default)
    };
    SectionVerdict { section: SectionId::Startup, lines: vec![l] }
}

pub fn graphics_rules(s: Option<&Section<Graphics>>, now_ms: i64) -> SectionVerdict {
    let problem = section_problem(SectionId::Graphics, s.map(|s| &s.check));
    let Some(data) = s.and_then(|s| s.data.as_ref()).filter(|_| problem.is_none()) else {
        return SectionVerdict { section: SectionId::Graphics, lines: couldnt_check(SectionId::Graphics, problem) };
    };
    let cutoff_day = fmt::utc_day(fmt::months_earlier(now_ms, DRIVER_AGE_INFO_MONTHS));
    let mut lines = Vec::new();
    for (i, gpu) in data.adapters.iter().enumerate() {
        if gpu.software == Some(true) {
            continue; // e.g. Microsoft Basic Render Driver
        }
        let name: Param = gpu.name.clone().unwrap_or_else(|| "?".into()).into();
        // Dates are YYYY-MM-DD, so comparing strings compares days.
        match gpu.driver_date.as_deref().filter(|d| !d.is_empty() && *d < cutoff_day.as_str()) {
            Some(date) => lines.push(line(format!("gpu-{i}"), Level::Info, "graphics.old_driver", &[("name", name), ("date", date.into())], Action::Default)),
            None => lines.push(line(format!("gpu-{i}"), Level::Fine, "graphics.fine", &[("name", name)], Action::Default)),
        }
    }
    SectionVerdict { section: SectionId::Graphics, lines }
}
