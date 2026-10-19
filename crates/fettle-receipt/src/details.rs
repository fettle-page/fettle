//! The raw numbers under each section's "details" fold. Missing facts
//! simply leave no row.

use crate::format::{bytes, int, iso_day, temp};
use crate::memory::{channel_of, rated_from_part_number, slot_of};
use crate::ui;
use fettle_facts::{Battery, Bus, Chassis, Facts, FormFactor, Graphics, Media, Memory, MemoryKind, Startup, Storage, Windows};
use fettle_verdicts::{Locale, Param, SectionId, fmt};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DetailRow {
    pub label: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DetailGroup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub rows: Vec<DetailRow>,
}

/// Builds rows for one group, skipping any whose value is missing or empty.
fn rows_of(locale: Locale, entries: Vec<(&str, Option<String>)>) -> Vec<DetailRow> {
    entries
        .into_iter()
        .filter_map(|(key, value)| value.filter(|v| !v.is_empty()).map(|value| DetailRow { label: ui(key, locale, &[]), value }))
        .collect()
}

fn groups(list: Vec<DetailGroup>) -> Vec<DetailGroup> {
    list.into_iter().filter(|g| !g.rows.is_empty()).collect()
}

/// Zero means "unknown" in SMBIOS, so it never shows as a value.
fn positive<T: Into<f64> + Copy>(n: Option<T>) -> Option<f64> {
    n.map(Into::into).filter(|v| *v > 0.0)
}

fn pct<T: Into<f64>>(n: Option<T>) -> Option<String> {
    n.map(|v| format!("{}%", fmt::js_number(v.into())))
}

/// Memory and VRAM are sold in binary sizes: 16 GiB is "16 GB" on the box.
fn binary_gb(n: Option<f64>) -> Option<String> {
    let n = n.filter(|n| *n > 0.0)?;
    Some(if n < 2f64.powi(30) { format!("{} MB", (n / 2f64.powi(20)).round()) } else { format!("{} GB", (n / 2f64.powi(30)).round()) })
}

fn u64f(n: Option<u64>) -> Option<f64> {
    n.map(|v| v as f64)
}

fn storage(d: &Storage, locale: Locale) -> Vec<DetailGroup> {
    let mut list: Vec<DetailGroup> = d
        .disks
        .iter()
        .map(|disk| {
            let bus = match disk.bus {
                Some(Bus::Nvme) => Some("NVMe"),
                Some(Bus::Sata) => Some("SATA"),
                Some(Bus::Usb) => Some("USB"),
                _ => None,
            };
            let media = match disk.media {
                Some(Media::Ssd) => Some("SSD"),
                Some(Media::Hdd) => Some("HDD"),
                _ => None,
            };
            let kind = [bus, media].into_iter().flatten().collect::<Vec<_>>().join(" ");
            let n = disk.nvme.as_ref();
            let s = disk.smart.as_ref();
            // SMART's life-used value is wear only on an SSD (on some hard drives it means something else).
            let smart_wear = s.and_then(|s| s.life_used_pct).filter(|_| disk.media == Some(Media::Ssd));
            let wear = n.and_then(|n| n.percentage_used).or(smart_wear);
            let t = n.and_then(|n| n.temperature_c).or_else(|| s.and_then(|s| s.temperature_c));
            let hours = n.and_then(|n| n.power_on_hours).or_else(|| s.and_then(|s| s.power_on_hours))
                // Over 100 years: other counters packed into the value (seen on SandForce SSDs), not hours.
                .filter(|h| *h <= 876_000);
            DetailGroup {
                title: disk.model.clone().filter(|m| !m.is_empty()),
                rows: rows_of(
                    locale,
                    vec![
                        ("d.size", positive(u64f(disk.size_bytes)).map(|b| bytes(b, locale))),
                        ("d.type", Some(kind)),
                        ("d.wear", pct(wear)),
                        ("d.temp", t.map(|t| temp(t.into()))),
                        ("d.hours", hours.map(|h| int(h as f64, locale))),
                        ("d.media_errors", n.and_then(|n| n.media_errors).map(|e| int(e as f64, locale))),
                        ("d.spare", pct(n.and_then(|n| n.available_spare_pct))),
                        ("d.reallocated", s.and_then(|s| s.reallocated_sectors).map(|r| int(r as f64, locale))),
                        ("d.pending", s.and_then(|s| s.pending_sectors).map(|p| int(p as f64, locale))),
                        ("d.uncorrectable", s.and_then(|s| s.offline_uncorrectable).map(|u| int(u as f64, locale))),
                    ],
                ),
            }
        })
        .collect();
    for v in &d.volumes {
        // A size of 0 means Windows doesn't know it; 0 free on a real volume is a real (full) reading.
        let free = match (v.free_bytes, positive(u64f(v.size_bytes))) {
            (Some(free), Some(size)) => Some(ui("d.free_of", locale, &[("free", bytes(free as f64, locale).into()), ("size", bytes(size, locale).into())])),
            _ => None,
        };
        list.push(DetailGroup {
            title: Some(ui("d.drive", locale, &[("letter", v.letter.clone().unwrap_or_else(|| "?".into()).into())])),
            rows: rows_of(locale, vec![("d.free", free)]),
        });
    }
    groups(list)
}

fn battery(d: &Battery, locale: Locale) -> Vec<DetailGroup> {
    let wh = |mwh: Option<u32>| positive(mwh).map(|m| format!("{} Wh", fmt::group(m / 1000.0, 1, locale)));
    groups(vec![DetailGroup {
        title: None,
        rows: rows_of(
            locale,
            vec![
                ("d.design", wh(d.design_capacity_mwh)),
                ("d.full", wh(d.full_charge_capacity_mwh)),
                ("d.cycles", d.cycle_count.map(|c| int(c.into(), locale))),
            ],
        ),
    }])
}

fn trimmed(s: Option<&String>) -> Option<&str> {
    s.map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn memory(d: &Memory, laptop: bool, locale: Locale) -> Vec<DetailGroup> {
    let modules = &d.modules;
    // Soldered memory (LPDDR) has no slots, and its chips' narrow channels don't mean what a
    // stick's do: four LPDDR5 channels are as wide as an ordinary desktop's two.
    let soldered = d.soldered();
    // Voltage is collected but not shown: firmware often reports the JEDEC 1.10 V even while an
    // XMP/EXPO profile runs the stick at ~1.35 V (seen on the dev workstation), so it would mislead.
    let sticks: Vec<DetailGroup> = modules
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let kind = match m.kind {
                Some(MemoryKind::Ddr3) => Some("DDR3".to_string()),
                Some(MemoryKind::Ddr4) => Some("DDR4".to_string()),
                Some(MemoryKind::Ddr5) => Some("DDR5".to_string()),
                Some(MemoryKind::Lpddr3) => Some("LPDDR3".to_string()),
                Some(MemoryKind::Lpddr4) => Some("LPDDR4".to_string()),
                Some(MemoryKind::Lpddr5) => Some("LPDDR5".to_string()),
                Some(MemoryKind::Other) | None => None,
            };
            let form = match m.form_factor {
                Some(FormFactor::Dimm) => Some("DIMM".to_string()),
                Some(FormFactor::Sodimm) => Some(ui("d.sodimm", locale, &[])),
                _ => None,
            };
            let now = positive(m.configured_speed_mts).or_else(|| positive(m.speed_mts));
            let max = positive(m.speed_mts);
            let speed = now.map(|now| match max {
                Some(max) if max > now => ui(if soldered { "d.speed_rated" } else { "d.speed_max" }, locale, &[("now", Param::Float(now)), ("max", Param::Float(max))]),
                _ => format!("{} MT/s", fmt::js_number(now)),
            });
            let rated = rated_from_part_number(m.part_number.as_deref()).map(|(speed, cl)| format!("{speed} MT/s · CL{cl}"));
            let (channel, dimm) = slot_of(m.bank_label.as_deref(), m.device_locator.as_deref());
            // A soldered chip's "DIMM 0" is the firmware's numbering, not a slot: it shows only its
            // channel, and no slot at all when it has none (the raw locator would be that "DIMM 0").
            let dimm = dimm.filter(|_| !soldered);
            let slot = match channel {
                Some(c) => Some([Some(ui("d.channel", locale, &[("c", c.to_string().into())])), dimm.map(|d| format!("DIMM {d}"))].into_iter().flatten().collect::<Vec<_>>().join(" · ")),
                None if soldered => None,
                None => trimmed(m.device_locator.as_ref()).map(str::to_string),
            };
            let title = [trimmed(m.manufacturer.as_ref()), trimmed(m.part_number.as_ref())].into_iter().flatten().collect::<Vec<_>>().join(" ");
            DetailGroup {
                title: Some(if title.is_empty() { ui("d.stick", locale, &[("n", Param::Int(i as i64 + 1))]) } else { title }),
                rows: rows_of(
                    locale,
                    vec![
                        ("d.size", binary_gb(u64f(m.capacity_bytes))),
                        ("d.type", Some([kind, form].into_iter().flatten().collect::<Vec<_>>().join(" "))),
                        ("d.speed", speed),
                        ("d.rated", rated),
                        ("d.slot", slot),
                    ],
                ),
            }
        })
        .filter(|g| !g.rows.is_empty())
        .collect();

    let total: f64 = modules.iter().map(|m| positive(u64f(m.capacity_bytes)).unwrap_or(0.0)).sum();
    let channels: Vec<Option<char>> = modules.iter().map(|m| channel_of(m.bank_label.as_deref(), m.device_locator.as_deref())).collect();
    let mut distinct = channels.clone();
    distinct.sort();
    distinct.dedup();
    let channel_text = if soldered || modules.is_empty() || channels.iter().any(Option::is_none) {
        None
    } else {
        Some(match distinct.len() {
            n @ (1 | 2 | 4) => ui(&format!("d.channels_{n}"), locale, &[]),
            n => ui("d.channels_n", locale, &[("n", Param::Int(n as i64))]),
        })
    };
    // Some firmware reports fewer slots than the sticks it lists; a contradiction is shown as unknown.
    // Laptop firmware often reports slots that aren't there, so a laptop shows only the ones in use,
    // whatever total the firmware gives.
    let used = ("used", Param::Int(modules.len() as i64));
    let slots = if soldered {
        Some(ui("d.soldered", locale, &[]))
    } else if laptop {
        (!modules.is_empty()).then(|| ui("d.slots_in_use", locale, &[used]))
    } else {
        d.slots_total.filter(|t| *t > 0 && *t as usize >= modules.len()).map(|t| ui("d.slots_used", locale, &[used, ("total", Param::Int(t.into()))]))
    };
    let summary = DetailGroup { title: None, rows: rows_of(locale, vec![("d.total", binary_gb(Some(total))), ("d.slots", slots), ("d.channels", channel_text)]) };
    let mut list = sticks;
    list.push(summary);
    groups(list)
}

fn hours_text(hours: u64, locale: Locale) -> String {
    match hours {
        0 => ui("d.under_hour", locale, &[]),
        1 => ui("d.hours_one", locale, &[]),
        n => ui("d.hours_short", locale, &[("n", Param::Int(n as i64))]),
    }
}

/// The words a product name uses for each edition Windows reports ("Professional" is "Pro").
/// An N edition ("ProfessionalN") adds an "N".
const EDITION_WORDS: &[(&str, &[&str])] = &[
    ("Core", &["Home"]),
    ("CoreSingleLanguage", &["Home", "Single", "Language"]),
    ("CoreCountrySpecific", &["Home", "China"]),
    ("Professional", &["Pro"]),
    ("ProfessionalWorkstation", &["Pro", "Workstations"]),
    ("ProfessionalEducation", &["Pro", "Education"]),
    ("Enterprise", &["Enterprise"]),
    ("EnterpriseS", &["Enterprise", "LTSC"]),
    ("EnterpriseS", &["Enterprise", "LTSB"]),
    ("Education", &["Education"]),
    ("IoTEnterprise", &["IoT", "Enterprise"]),
    ("IoTEnterpriseS", &["IoT", "Enterprise", "LTSC"]),
    ("ServerRdsh", &["Enterprise", "multi-session"]),
    ("CloudEdition", &["SE"]),
];

/// Whether the product name ("Windows 11 Pro") already says the edition ("Professional").
fn names_edition(product: &str, edition: &str) -> bool {
    let words: Vec<&str> = product.split_whitespace().collect();
    let has = |w: &str| words.iter().any(|p| p.eq_ignore_ascii_case(w));
    let known = |id: &str| EDITION_WORDS.iter().any(|(e, _)| *e == id);
    let (base, n) = match edition.strip_suffix('N') {
        Some(base) if !known(edition) && known(base) => (base, true),
        _ => (edition, false),
    };
    (!n || has("N")) && EDITION_WORDS.iter().any(|(e, needed)| *e == base && needed.iter().all(|w| has(w)))
}

fn windows(d: &Windows, locale: Locale) -> Vec<DetailGroup> {
    let uptime = d.uptime_seconds.map(|up| match up / 86400 {
        0 => hours_text(up / 3600, locale),
        1 => ui("d.days_one", locale, &[]),
        days => ui("d.days", locale, &[("n", Param::Int(days as i64))]),
    });
    let tpm = d.tpm.as_ref().and_then(|t| t.present.map(|present| (present, t))).map(|(present, t)| {
        if !present {
            return ui("d.not_found", locale, &[]);
        }
        let first = t.spec_version.as_deref().unwrap_or("").split(',').next().unwrap_or("").trim();
        // No version: switched off or not ready in the firmware, or just not reported. Only "found".
        if first.is_empty() { ui("d.present", locale, &[]) } else { first.to_string() }
    });
    groups(vec![DetailGroup {
        title: None,
        rows: rows_of(
            locale,
            vec![
                ("d.version", d.display_version.clone()),
                ("d.build", d.build.map(|b| d.ubr.map(|u| format!("{b}.{u}")).unwrap_or_else(|| b.to_string()))),
                // "Professional" after "Windows 11 Pro" says nothing new (and in English on an Italian receipt).
                ("d.edition", d.edition_id.clone().filter(|e| !d.product.as_deref().is_some_and(|p| names_edition(p, e)))),
                ("d.last_update", d.last_update_installed.as_deref().filter(|s| !s.is_empty()).map(|s| iso_day(s, locale))),
                ("d.uptime", uptime),
                ("d.secure_boot", d.secure_boot.map(|on| ui(if on { "d.yes" } else { "d.no" }, locale, &[]))),
                ("d.tpm", tpm),
            ],
        ),
    }])
}

/// The order startup names are listed in, the same in TypeScript, Rust and Kotlin whatever the
/// viewer's language: case-insensitive, then exact, by UTF-16 code units (JavaScript's `<`).
pub fn name_order(a: &str, b: &str) -> Ordering {
    let units = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    units(&a.to_lowercase()).cmp(&units(&b.to_lowercase())).then_with(|| units(a).cmp(&units(b)))
}

fn startup(d: &Startup, locale: Locale) -> Vec<DetailGroup> {
    // An item without a readable name (stored empty, or in English before 1.0.3) shows in the
    // receipt's language, and so does one Windows stored under an internal name ("SecurityHealth");
    // one whose name wasn't read at all isn't listed.
    use fettle_facts::startup_names::{UNNAMED_BEFORE_1_0_3, shown_as};
    let unnamed = ui("d.unnamed_startup", locale, &[]);
    let label = |n: &str| match shown_as(n) {
        Some(key) => ui(&format!("d.app.{key}"), locale, &[]),
        None if n.is_empty() || n == UNNAMED_BEFORE_1_0_3 => unnamed.clone(),
        None => n.to_string(),
    };
    let named: Vec<_> = d.items.iter().filter_map(|i| i.name.as_deref().map(|n| (label(n), i.enabled != Some(false)))).collect();
    let mut on: Vec<&str> = named.iter().filter(|(_, e)| *e).map(|(n, _)| n.as_str()).collect();
    let mut off: Vec<&str> = named.iter().filter(|(_, e)| !*e).map(|(n, _)| n.as_str()).collect();
    on.sort_by(|a, b| name_order(a, b));
    off.sort_by(|a, b| name_order(a, b));
    let row = |enabled: bool| move |name: &str| DetailRow { label: name.to_string(), value: ui(if enabled { "d.enabled" } else { "d.disabled" }, locale, &[]) };
    let rows = on.into_iter().map(row(true)).chain(off.into_iter().map(row(false))).collect();
    groups(vec![DetailGroup { title: None, rows }])
}

fn graphics(d: &Graphics, locale: Locale) -> Vec<DetailGroup> {
    groups(
        d.adapters
            .iter()
            .filter(|a| a.software != Some(true))
            .map(|a| DetailGroup {
                title: a.name.clone().filter(|n| !n.is_empty()),
                rows: rows_of(
                    locale,
                    vec![
                        ("d.vram", binary_gb(u64f(a.vram_bytes))),
                        ("d.driver", a.driver_version.clone()),
                        ("d.driver_date", a.driver_date.as_deref().filter(|s| !s.is_empty()).map(|s| iso_day(s, locale))),
                        ("d.temp", a.temperature_c.map(|t| temp(t.into()))),
                    ],
                ),
            })
            .collect(),
    )
}

/// Detail groups for one section; empty when the section wasn't read or has nothing to show.
pub fn section_details(id: SectionId, facts: &Facts, locale: Locale) -> Vec<DetailGroup> {
    fn data<T>(s: &Option<fettle_facts::Section<T>>) -> Option<&T> {
        s.as_ref().and_then(|s| s.data.as_ref())
    }
    match id {
        SectionId::Storage => data(&facts.storage).map(|d| storage(d, locale)),
        SectionId::Battery => data(&facts.battery).map(|d| battery(d, locale)),
        SectionId::Memory => data(&facts.memory).map(|d| memory(d, data(&facts.system).is_some_and(|s| s.chassis == Some(Chassis::Laptop)), locale)),
        SectionId::Windows => data(&facts.windows).map(|d| windows(d, locale)),
        SectionId::Startup => data(&facts.startup).map(|d| startup(d, locale)),
        SectionId::Graphics => data(&facts.graphics).map(|d| graphics(d, locale)),
        SectionId::System => None,
    }
    .unwrap_or_default()
}

/// The "THIS PC" block under the receipt header.
pub fn machine_rows(facts: &Facts, locale: Locale) -> Vec<DetailRow> {
    let Some(s) = facts.system.as_ref().and_then(|s| s.data.as_ref()) else { return Vec::new() };
    let bios = [s.bios_version.clone().filter(|v| !v.is_empty()), s.bios_date.as_deref().filter(|d| !d.is_empty()).map(|d| iso_day(d, locale))]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    // Reports from 1.0.4 and earlier can carry a name the maker never filled in ("Default string").
    let board = s.board.as_deref().and_then(fettle_facts::placeholders::without_placeholders);
    rows_of(locale, vec![("d.cpu", s.cpu_name.clone()), ("d.board", board), ("d.bios", Some(bios))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_sort_the_same_everywhere() {
        // Case-insensitive, then exact, by UTF-16 units (the order the Android app must keep too).
        let mut names = vec!["éclair", "Zoom", "apple", "Łódź", "Apple", "Banana", "Intel® Graphics", "IntelX"];
        names.sort_by(|a, b| name_order(a, b));
        assert_eq!(names, ["Apple", "apple", "Banana", "IntelX", "Intel® Graphics", "Zoom", "éclair", "Łódź"]);
        // UTF-16, not code points: an emoji's high surrogate sorts before U+FF21 (fullwidth A).
        assert_eq!(name_order("\u{1F680}", "\u{FF21}"), Ordering::Less);
    }

    // Reports saved by 1.0.1 from a SandForce SSD carry hours with other counters packed in
    // (79,869,211,876,947): a value no drive can have isn't shown.
    #[test]
    fn impossible_power_on_hours_are_not_shown() {
        use fettle_facts::{Disk, SmartSummary};
        let rows = |hours: u64| {
            let disk = Disk { smart: Some(SmartSummary { power_on_hours: Some(hours), ..Default::default() }), ..Default::default() };
            storage(&Storage { disks: vec![disk], ..Default::default() }, Locale::En).into_iter().flat_map(|g| g.rows).collect::<Vec<_>>()
        };
        assert!(rows(40_531).iter().any(|r| r.value == "40,531"));
        assert!(!rows(79_869_211_876_947).iter().any(|r| r.value.contains("79,869")));
    }

    #[test]
    fn every_known_memory_type_is_named() {
        use fettle_facts::MemoryModule;
        for (kind, name) in [(MemoryKind::Ddr3, "DDR3"), (MemoryKind::Lpddr3, "LPDDR3"), (MemoryKind::Lpddr4, "LPDDR4"), (MemoryKind::Lpddr5, "LPDDR5"), (MemoryKind::Ddr4, "DDR4"), (MemoryKind::Ddr5, "DDR5")] {
            let d = Memory { modules: vec![MemoryModule { kind: Some(kind), ..Default::default() }], ..Default::default() };
            let rows: Vec<_> = memory(&d, false, Locale::En).into_iter().flat_map(|g| g.rows).collect();
            assert!(rows.iter().any(|r| r.value == name), "{name}: {rows:?}");
        }
    }
}
