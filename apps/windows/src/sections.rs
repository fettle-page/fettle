//! Which scan sources make up each receipt section, so the progress list and the blank receipt
//! can tick a section off once all its sources have reported. Names match
//! crates/fettle-collect/src/sources/mod.rs. The TPM source only runs for a Deep check, so the
//! Windows section doesn't wait for it.

use crate::copy;
use crate::{BlankSection, Progress, Row};
use fettle_verdicts::{Locale, Param};
use slint::{ModelRc, SharedString, VecModel};

pub const SECTION_SOURCES: &[(&str, &[&str])] = &[
    ("system", &["cpu", "baseboard", "bios", "computer_system", "monitor_ids", "monitor_params"]),
    ("storage", &["physical_disks", "logical_disks", "smart_predict_data", "nvme_health"]),
    ("battery", &["battery_cimv2", "battery_static", "battery_full_capacity", "battery_cycle_count"]),
    ("memory", &["memory_modules", "memory_array"]),
    ("windows", &["os", "windows_registry", "secure_boot", "hotfixes"]),
    ("startup", &["startup_registry", "startup_folders", "startup_approved"]),
    ("graphics", &["dxgi_adapters", "video_controllers"]),
];

/// The sections the blank receipt lists (the machine block stands for "system").
const BLANK: [&str; 6] = ["storage", "battery", "memory", "windows", "startup", "graphics"];

pub fn section_done(id: &str, done: &[String]) -> bool {
    SECTION_SOURCES.iter().find(|(s, _)| *s == id).is_some_and(|(_, sources)| sources.iter().all(|s| done.iter().any(|d| d == s)))
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

/// The checking list, and what a screen reader says about it ("3 of 7 sections done").
pub fn progress(done: &[String], lang: Locale) -> (ModelRc<Progress>, String) {
    let items: Vec<Progress> = SECTION_SOURCES
        .iter()
        .map(|(id, _)| Progress { title: copy::any(&format!("ui.section.{id}"), lang).into(), done: section_done(id, done) })
        .collect();
    let finished = items.iter().filter(|p| p.done).count() as i64;
    (model(items.clone()), copy::t("scan.count", lang, &[("done", Param::Int(finished)), ("total", Param::Int(items.len() as i64))]))
}

/// The blank receipt: its machine labels and one line per section, ticked when done.
pub fn blank(done: &[String], lang: Locale) -> (ModelRc<Row>, ModelRc<BlankSection>) {
    let machine = ["d.cpu", "d.board", "d.bios"].iter().map(|k| Row { label: copy::any(&format!("ui.{k}"), lang).into(), value: SharedString::new() }).collect();
    let sections = BLANK
        .iter()
        .map(|id| BlankSection { title: copy::any(&format!("ui.section.{id}"), lang).into(), hint: copy::t(&format!("blank.{id}"), lang, &[]).into(), done: section_done(id, done) })
        .collect();
    (model(machine), model(sections))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_ticks_when_all_its_sources_are_done() {
        let done: Vec<String> = ["memory_modules", "memory_array", "cpu"].iter().map(|s| s.to_string()).collect();
        assert!(section_done("memory", &done));
        assert!(!section_done("system", &done));
    }
}
