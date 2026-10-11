//! Everything the receipt shows, already in one language (receipt-ui's model.ts). The platforms
//! (Slint, Compose, React) only lay it out.

use crate::details::{DetailGroup, DetailRow, machine_rows, section_details};
use crate::format::{Zone, stamp};
use crate::ui;
use fettle_facts::Report;
use fettle_verdicts::{Level, Locale, MessageRef, Receipt, SectionId, render};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ModelLine {
    pub id: String,
    pub level: Level,
    pub tag: String,
    pub level_text: String,
    pub headline: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelSection {
    pub id: SectionId,
    pub title: String,
    pub lines: Vec<ModelLine>,
    pub details: Vec<DetailGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub when: String,
    /// The raw timestamp: it identifies the report whatever the language.
    pub collected_at: String,
    pub deep: bool,
    pub footer: String,
    pub scan: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Labels {
    pub details: String,
    pub next_step: String,
    pub machine: String,
    pub stamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReceiptModel {
    pub locale: Locale,
    /// fine, some, attention or partial.
    #[cfg_attr(feature = "schema", schemars(extend("enum" = ["fine", "some", "attention", "partial"])))]
    pub tone: String,
    pub summary: String,
    pub actions: Vec<String>,
    pub machine: Vec<DetailRow>,
    pub sections: Vec<ModelSection>,
    pub meta: Meta,
    pub labels: Labels,
}

/// The receipt for a report and its verdicts, in one language, with the scan time in `zone`.
pub fn build(report: &Report, receipt: &Receipt, locale: Locale, zone: Zone) -> ReceiptModel {
    let level_key = |l: Level| serde_json::to_value(l).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    let sections = receipt
        .sections
        .iter()
        .filter(|s| !s.lines.is_empty())
        .map(|s| ModelSection {
            id: s.section,
            title: ui(&format!("section.{}", s.section.key()), locale, &[]),
            lines: s
                .lines
                .iter()
                .map(|l| ModelLine {
                    id: l.id.clone(),
                    level: l.level,
                    tag: ui(&format!("tag.{}", level_key(l.level)), locale, &[]),
                    level_text: render(&MessageRef::key(format!("level.{}", level_key(l.level))), locale),
                    headline: render(&l.headline, locale),
                    detail: l.detail.as_ref().map(|d| render(d, locale)),
                    action: l.action.as_ref().map(|a| render(a, locale)),
                })
                .collect(),
            details: section_details(s.section, &report.facts, locale),
        })
        .collect();
    let key = receipt.summary.key.as_str();
    ReceiptModel {
        locale,
        // "Fine so far" (disks or something else unchecked) still looks fine: what was checked is fine,
        // and the top line says the rest.
        tone: if matches!(key, "summary.fine_unchecked" | "summary.fine_partial") { "fine" } else { key.strip_prefix("summary.").unwrap_or("partial") }.to_string(),
        summary: render(&MessageRef::key(key), locale),
        actions: receipt.summary.actions.iter().map(|a| render(a, locale)).collect(),
        machine: machine_rows(&report.facts, locale),
        sections,
        meta: Meta {
            when: stamp(&report.collected_at, zone),
            collected_at: report.collected_at.clone(),
            deep: report.elevated,
            scan: ui(if report.elevated { "footer.deep" } else { "footer.quick" }, locale, &[]),
            footer: ui("footer.versions", locale, &[("app", report.app_version.as_str().into()), ("engine", receipt.engine_version.as_str().into())]),
        },
        labels: Labels {
            details: ui("details", locale, &[]),
            next_step: ui("next_step", locale, &[]),
            machine: ui("machine", locale, &[]),
            // A mostly unchecked PC isn't "checked": the stamp stays honest.
            stamp: ui(if key == "summary.partial" { "stamp.partial" } else { "stamp.checked" }, locale, &[]),
        },
    }
}
