//! Two checks of the same PC side by side, for a technician's before and after: only what changed,
//! each change marked better, worse or neither. Verdict lines come from each check's own receipt.

use crate::details::{binary_gb, disk_kind, disk_wear};
use crate::format::{Zone, bytes, int, iso_day, temp};
use crate::markdown::{esc, utf16_len};
use crate::{ModelLine, ReceiptModel, receipt_for, ui};
use fettle_facts::{Disk, EsuState, Facts, Report, Section};
use fettle_verdicts::{Level, Locale, SectionId, fmt};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Change {
    Better,
    Worse,
    Neutral,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompareRow {
    pub label: String,
    /// "—" when the thing wasn't there before.
    pub before: String,
    /// "—" when the thing is gone now.
    pub after: String,
    pub change: Change,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompareGroup {
    pub section: SectionId,
    pub title: String,
    pub rows: Vec<CompareRow>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompareLabels {
    /// "Before and after".
    pub title: String,
    pub before: String,
    pub after: String,
    /// "better" and "worse", as the text copy marks a change.
    pub better: String,
    pub worse: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub locale: Locale,
    /// The older check's stamp ("01/11/2026 09:00").
    pub before: String,
    /// The newer check's stamp.
    pub after: String,
    pub summary_before: String,
    pub summary_after: String,
    /// A calm warning when the two checks look like two PCs; the rows are still listed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub different_pc: Option<String>,
    /// Set when no row differs (and then `groups` is empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nothing_changed: Option<String>,
    pub groups: Vec<CompareGroup>,
    pub labels: CompareLabels,
}

const DASH: &str = "—";

/// The receipt's section order.
const SECTIONS: [SectionId; 7] = [SectionId::Storage, SectionId::Battery, SectionId::Memory, SectionId::Windows, SectionId::Startup, SectionId::Graphics, SectionId::System];

/// Compares two checks; whichever was collected first is "before", whatever order they come in.
/// Refuses a scan time that doesn't exist, as `receipt_for` does.
pub fn compare(a: &Report, b: &Report, locale: Locale, zone: Zone) -> Result<Comparison, String> {
    let time = |r: &Report| fmt::parse_rfc3339(&r.collected_at).ok_or_else(|| format!("not a valid scan time: {}", r.collected_at));
    let (ta, tb) = (time(a)?, time(b)?);
    // Same instant: the raw text decides, so the order of the arguments still doesn't matter.
    let (old, new) = match ta.cmp(&tb).then_with(|| a.collected_at.cmp(&b.collected_at)) {
        Ordering::Greater => (b, a),
        _ => (a, b),
    };
    let (_, before) = receipt_for(old, locale, zone)?;
    let (_, after) = receipt_for(new, locale, zone)?;
    let groups: Vec<CompareGroup> = SECTIONS
        .into_iter()
        .filter_map(|section| {
            let mut rows = verdict_rows(section, &before, &after);
            rows.extend(fact_rows(section, &old.facts, &new.facts, locale));
            (!rows.is_empty()).then(|| CompareGroup { section, title: ui(&format!("section.{}", section.key()), locale, &[]), rows })
        })
        .collect();
    Ok(Comparison {
        locale,
        before: before.meta.when.clone(),
        after: after.meta.when.clone(),
        summary_before: before.summary.clone(),
        summary_after: after.summary.clone(),
        different_pc: different_pc(&old.facts, &new.facts).then(|| ui("compare.different_pc", locale, &[])),
        nothing_changed: groups.is_empty().then(|| ui("compare.nothing_changed", locale, &[])),
        groups,
        labels: CompareLabels {
            title: ui("compare.title", locale, &[]),
            before: ui("compare.before", locale, &[]),
            after: ui("compare.after", locale, &[]),
            better: ui("compare.better", locale, &[]),
            worse: ui("compare.worse", locale, &[]),
        },
    })
}

/// Reports hold no system maker or model, only the board ("maker product"): two different known
/// boards mean two PCs.
fn different_pc(a: &Facts, b: &Facts) -> bool {
    let board = |f: &Facts| data(&f.system).and_then(|s| s.board.as_deref()).and_then(fettle_facts::placeholders::without_placeholders);
    matches!((board(a), board(b)), (Some(x), Some(y)) if x != y)
}

fn data<T>(s: &Option<Section<T>>) -> Option<&T> {
    s.as_ref().and_then(|s| s.data.as_ref())
}

/// A row only when the two values read differently.
fn row(rows: &mut Vec<CompareRow>, label: String, before: String, after: String, change: Change) {
    if before != after {
        rows.push(CompareRow { label, before, after, change });
    }
}

/// Better when the value moved the good way, worse the other way; a value that didn't move is neutral.
fn direction<T: PartialOrd>(before: T, after: T, higher_is_better: bool) -> Change {
    match before.partial_cmp(&after) {
        Some(Ordering::Less) if higher_is_better => Change::Better,
        Some(Ordering::Less) => Change::Worse,
        Some(Ordering::Greater) if higher_is_better => Change::Worse,
        Some(Ordering::Greater) => Change::Better,
        _ => Change::Neutral,
    }
}

/// How serious a level is; "couldn't check" says nothing about the PC, so it has no rank.
fn rank(level: Level) -> Option<u8> {
    match level {
        Level::Fine | Level::Info => Some(0),
        Level::KeepAnEye => Some(1),
        Level::ActSoon => Some(2),
        Level::CouldntCheck => None,
    }
}

fn verdict_rows(section: SectionId, before: &ReceiptModel, after: &ReceiptModel) -> Vec<CompareRow> {
    let lines = |m: &'_ ReceiptModel| m.sections.iter().filter(|s| s.id == section).flat_map(|s| s.lines.clone()).collect::<Vec<ModelLine>>();
    let (old, new) = (lines(before), lines(after));
    // A fine line or a note appearing or going away (a disk added, a restart nudge gone) is neither.
    let mild = |l: &ModelLine| matches!(l.level, Level::Fine | Level::Info);
    let mut rows = Vec::new();
    for b in &old {
        match new.iter().find(|a| a.id == b.id) {
            Some(a) if a.level != b.level => {
                let change = match (rank(b.level), rank(a.level)) {
                    (Some(x), Some(y)) => direction(x, y, false),
                    _ => Change::Neutral,
                };
                row(&mut rows, a.headline.clone(), b.level_text.clone(), a.level_text.clone(), change);
            }
            Some(_) => {}
            None => row(&mut rows, b.headline.clone(), b.level_text.clone(), DASH.into(), if mild(b) { Change::Neutral } else { Change::Better }),
        }
    }
    for a in new.iter().filter(|a| !old.iter().any(|b| b.id == a.id)) {
        row(&mut rows, a.headline.clone(), DASH.into(), a.level_text.clone(), if mild(a) { Change::Neutral } else { Change::Worse });
    }
    rows
}

fn fact_rows(section: SectionId, a: &Facts, b: &Facts, locale: Locale) -> Vec<CompareRow> {
    let mut rows = Vec::new();
    match section {
        SectionId::Storage => storage(&mut rows, a, b, locale),
        SectionId::Battery => battery(&mut rows, a, b, locale),
        SectionId::Memory => memory(&mut rows, a, b, locale),
        SectionId::Windows => windows(&mut rows, a, b, locale),
        SectionId::Startup => startup(&mut rows, a, b, locale),
        SectionId::Graphics => graphics(&mut rows, a, b, locale),
        SectionId::System => {}
    }
    rows
}

/// "Example SSD 1TB · Wear".
fn of(name: &str, key: &str, locale: Locale) -> String {
    format!("{name} · {}", ui(key, locale, &[]))
}

/// Pairs each item before with the first unpaired one after that has the same key; the rest are
/// only on one side.
fn pair<'a, T, K: PartialEq>(before: &'a [T], after: &'a [T], key: impl Fn(&T) -> K) -> (Vec<(&'a T, &'a T)>, Vec<&'a T>, Vec<&'a T>) {
    let mut used = vec![false; after.len()];
    let mut pairs = Vec::new();
    let mut gone = Vec::new();
    for x in before {
        match (0..after.len()).find(|&i| !used[i] && key(&after[i]) == key(x)) {
            Some(i) => {
                used[i] = true;
                pairs.push((x, &after[i]));
            }
            None => gone.push(x),
        }
    }
    let added = after.iter().zip(&used).filter(|(_, u)| !**u).map(|(y, _)| y).collect();
    (pairs, gone, added)
}

/// One of a disk's error counters.
type Counter = fn(&Disk) -> Option<u64>;

fn storage(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.storage), data(&b.storage)) else { return };
    let name = |d: &Disk| d.model.clone().filter(|m| !m.trim().is_empty()).unwrap_or_else(|| ui("compare.disk", locale, &[]));
    // Reports hold no serials: a disk is its model, size and bus.
    let (pairs, gone, added) = pair(&a.disks, &b.disks, |d| (d.model.clone(), d.size_bytes, d.bus));
    for (x, y) in pairs {
        let n = name(x);
        let both = |f: Counter| f(x).zip(f(y));
        if let (Some(p), Some(q)) = (disk_wear(x), disk_wear(y)) {
            row(rows, of(&n, "d.wear", locale), format!("{p}%"), format!("{q}%"), direction(p, q, false));
        }
        let t = |d: &Disk| d.nvme.as_ref().and_then(|n| n.temperature_c).or_else(|| d.smart.as_ref().and_then(|s| s.temperature_c));
        if let (Some(p), Some(q)) = (t(x), t(y)) {
            row(rows, of(&n, "d.temp", locale), temp(p.into()), temp(q.into()), Change::Neutral);
        }
        let counters: [(&str, Counter); 4] = [
            ("d.media_errors", |d| d.nvme.as_ref().and_then(|n| n.media_errors)),
            ("d.reallocated", |d| d.smart.as_ref().and_then(|s| s.reallocated_sectors)),
            ("d.pending", |d| d.smart.as_ref().and_then(|s| s.pending_sectors)),
            ("d.uncorrectable", |d| d.smart.as_ref().and_then(|s| s.offline_uncorrectable)),
        ];
        for (key, f) in counters {
            if let Some((p, q)) = both(f) {
                row(rows, of(&n, key, locale), int(p as f64, locale), int(q as f64, locale), direction(p, q, false));
            }
        }
    }
    let about = |d: &Disk| [d.size_bytes.filter(|s| *s > 0).map(|s| bytes(s as f64, locale)), Some(disk_kind(d)).filter(|k| !k.is_empty())].into_iter().flatten().collect::<Vec<_>>().join(" · ");
    let shown = |d: &Disk| Some(about(d)).filter(|s| !s.is_empty()).unwrap_or_else(|| name(d));
    for d in gone {
        row(rows, ui("compare.disk_removed", locale, &[("name", name(d).into())]), shown(d), DASH.into(), Change::Neutral);
    }
    for d in added {
        row(rows, ui("compare.disk_added", locale, &[("name", name(d).into())]), DASH.into(), shown(d), Change::Neutral);
    }
    // Volumes by letter; one on a single side (a USB stick plugged in) isn't a change to the PC.
    for x in &a.volumes {
        let Some(letter) = x.letter.as_deref().filter(|l| !l.is_empty()) else { continue };
        let Some(y) = b.volumes.iter().find(|v| v.letter.as_deref() == Some(letter)) else { continue };
        let free = |v: &fettle_facts::Volume| match (v.free_bytes, v.size_bytes.filter(|s| *s > 0)) {
            (Some(free), Some(size)) => Some((free, format!("{} ({}%)", ui("d.free_of", locale, &[("free", bytes(free as f64, locale).into()), ("size", bytes(size as f64, locale).into())]), ((free as f64 / size as f64) * 100.0).round()))),
            _ => None,
        };
        if let (Some((p, pt)), Some((q, qt))) = (free(x), free(y)) {
            let label = format!("{} · {}", ui("d.drive", locale, &[("letter", letter.into())]), ui("d.free", locale, &[]));
            row(rows, label, pt, qt, direction(p, q, true));
        }
    }
}

fn battery(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.battery), data(&b.battery)) else { return };
    // Neutral: a technician can't fix a battery's health, and calibration moves it.
    let health = |d: &fettle_facts::Battery| match (d.full_charge_capacity_mwh.filter(|n| *n > 0), d.design_capacity_mwh.filter(|n| *n > 0)) {
        (Some(full), Some(design)) => Some(format!("{}%", (f64::from(full) / f64::from(design) * 100.0).round())),
        _ => None,
    };
    if let (Some(p), Some(q)) = (health(a), health(b)) {
        row(rows, ui("compare.battery_health", locale, &[]), p, q, Change::Neutral);
    }
    if let (Some(p), Some(q)) = (a.cycle_count, b.cycle_count) {
        row(rows, ui("d.cycles", locale, &[]), int(p.into(), locale), int(q.into(), locale), Change::Neutral);
    }
}

fn memory(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.memory), data(&b.memory)) else { return };
    if a.modules.is_empty() || b.modules.is_empty() {
        return;
    }
    let total = |m: &fettle_facts::Memory| m.modules.iter().filter_map(|s| s.capacity_bytes).sum::<u64>();
    if let (Some(p), Some(q)) = (binary_gb(Some(total(a) as f64)), binary_gb(Some(total(b) as f64))) {
        row(rows, ui("d.total", locale, &[]), p, q, direction(total(a), total(b), true));
    }
    row(rows, ui("compare.sticks", locale, &[]), a.modules.len().to_string(), b.modules.len().to_string(), Change::Neutral);
    // All sticks run at one speed; the slowest reading is the one in use.
    let speed = |m: &fettle_facts::Memory| m.modules.iter().filter_map(|s| s.configured_speed_mts).filter(|s| *s > 0).min();
    if let (Some(p), Some(q)) = (speed(a), speed(b)) {
        row(rows, ui("d.speed", locale, &[]), format!("{p} MT/s"), format!("{q} MT/s"), direction(p, q, true));
    }
}

fn windows(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.windows), data(&b.windows)) else { return };
    // Version and build are neutral; time since restart is never compared (it's noise).
    if let (Some(p), Some(q)) = (a.display_version.clone(), b.display_version.clone()) {
        row(rows, ui("d.version", locale, &[]), p, q, Change::Neutral);
    }
    let build = |w: &fettle_facts::Windows| w.build.map(|b| w.ubr.map(|u| format!("{b}.{u}")).unwrap_or_else(|| b.to_string()));
    if let (Some(p), Some(q)) = (build(a), build(b)) {
        row(rows, ui("d.build", locale, &[]), p, q, Change::Neutral);
    }
    let esu = |s: EsuState| {
        ui(
            match s {
                EsuState::Enrolled => "compare.esu.enrolled",
                EsuState::NotEnrolled => "compare.esu.not_enrolled",
                EsuState::Unknown => "compare.esu.unknown",
            },
            locale,
            &[],
        )
    };
    if let (Some(p), Some(q)) = (a.esu, b.esu) {
        let change = match (p, q) {
            (x, EsuState::Enrolled) if x != EsuState::Enrolled => Change::Better,
            (EsuState::Enrolled, y) if y != EsuState::Enrolled => Change::Worse,
            _ => Change::Neutral,
        };
        row(rows, ui("compare.esu", locale, &[]), esu(p), esu(q), change);
    }
}

fn startup(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.startup), data(&b.startup)) else { return };
    // As the verdict counts: items switched off in Task Manager don't start.
    let count = |s: &fettle_facts::Startup| s.items.iter().filter(|i| i.enabled != Some(false)).count();
    let (p, q) = (count(a), count(b));
    row(rows, ui("compare.startup_apps", locale, &[]), int(p as f64, locale), int(q as f64, locale), direction(p, q, false));
}

fn graphics(rows: &mut Vec<CompareRow>, a: &Facts, b: &Facts, locale: Locale) {
    let (Some(a), Some(b)) = (data(&a.graphics), data(&b.graphics)) else { return };
    let real = |g: &fettle_facts::Graphics| g.adapters.iter().filter(|x| x.software != Some(true) && x.name.as_deref().is_some_and(|n| !n.is_empty())).cloned().collect::<Vec<_>>();
    let (old, new) = (real(a), real(b));
    let (pairs, _, _) = pair(&old, &new, |g| g.name.clone());
    for (x, y) in pairs {
        let n = x.name.clone().unwrap_or_default();
        let day = |g: &fettle_facts::Gpu| g.driver_date.clone().filter(|d| !d.is_empty());
        // Dates are YYYY-MM-DD, so comparing strings compares days. A new driver version is as good
        // as its date says.
        let change = match (day(x), day(y)) {
            (Some(p), Some(q)) => direction(p, q, true),
            _ => Change::Neutral,
        };
        if let (Some(p), Some(q)) = (x.driver_version.clone(), y.driver_version.clone()) {
            row(rows, of(&n, "d.driver", locale), p, q, change);
        }
        if let (Some(p), Some(q)) = (day(x), day(y)) {
            row(rows, of(&n, "d.driver_date", locale), iso_day(&p, locale), iso_day(&q, locale), change);
        }
    }
}

/// Discord's message limit, as for the receipt's markdown.
const MAX: usize = 2000;

/// The comparison as compact markdown for Copy, in the receipt markdown's style, at most 2000
/// UTF-16 units.
pub fn compare_markdown(c: &Comparison) -> String {
    let mut out = vec![
        format!("**Fettle** · {}", esc(&c.labels.title)),
        format!("- {}: {} · {}", esc(&c.labels.before), esc(&c.before), esc(&c.summary_before)),
        format!("- {}: {} · {}", esc(&c.labels.after), esc(&c.after), esc(&c.summary_after)),
    ];
    if let Some(warning) = &c.different_pc {
        out.push(String::new());
        out.push(format!("_{}_", esc(warning)));
    }
    if let Some(same) = &c.nothing_changed {
        out.push(String::new());
        out.push(esc(same));
    }
    for g in &c.groups {
        out.push(String::new());
        out.push(format!("**{}**", esc(&g.title)));
        for r in &g.rows {
            let mark = match r.change {
                Change::Better => format!(" ({})", esc(&c.labels.better)),
                Change::Worse => format!(" ({})", esc(&c.labels.worse)),
                Change::Neutral => String::new(),
            };
            out.push(format!("- {}: {} → {}{mark}", esc(&r.label), esc(&r.before), esc(&r.after)));
        }
    }
    let fits = |lines: &[String]| utf16_len(&lines.join("\n")) <= MAX;
    if !fits(&out) {
        // Rows go from the end, never the header, and the cut says so.
        while out.len() > 3 && !fits(&[out.as_slice(), &["…".to_string()]].concat()) {
            out.pop();
        }
        out.push("…".into());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use fettle_verdicts::Param;

    #[test]
    fn a_long_comparison_is_cut_to_2000_utf16_units() {
        let rows = (0..300).map(|i| CompareRow { label: format!("Example App {i} 🚀"), before: "on".into(), after: "off".into(), change: Change::Neutral }).collect();
        let labels = CompareLabels { title: "Before and after".into(), before: "Before".into(), after: "After".into(), better: "better".into(), worse: "worse".into() };
        let c = Comparison {
            locale: Locale::En,
            before: "01/11/2026 09:00 UTC".into(),
            after: "03/11/2026 17:30 UTC".into(),
            summary_before: "Needs a little fettling.".into(),
            summary_after: "In fine fettle.".into(),
            different_pc: None,
            nothing_changed: None,
            groups: vec![CompareGroup { section: SectionId::Startup, title: "Startup".into(), rows }],
            labels,
        };
        let md = compare_markdown(&c);
        assert!(utf16_len(&md) <= MAX, "{}", utf16_len(&md));
        assert!(md.ends_with("\n…"), "{md}");
        assert!(md.starts_with("**Fettle** · Before and after\n- Before: 01/11/2026 09:00 UTC"), "{md}");
    }

    #[test]
    fn moving_the_good_way_is_better() {
        assert_eq!(direction(41, 9, false), Change::Better);
        assert_eq!(direction(3, 12, false), Change::Worse);
        assert_eq!(direction(16, 32, true), Change::Better);
        assert_eq!(direction(5, 5, true), Change::Neutral);
    }

    #[test]
    fn param_names_match_the_copy() {
        // Unfilled slots would show as "{name}".
        for locale in [Locale::En, Locale::It] {
            for key in ["compare.disk_added", "compare.disk_removed"] {
                assert!(!ui(key, locale, &[("name", Param::Str("Example SSD".into()))]).contains('{'), "{key}");
            }
        }
    }
}
