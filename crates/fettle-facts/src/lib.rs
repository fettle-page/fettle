//! The facts a Fettle scan collects: raw values only, no verdicts (those are fettle-verdicts').
//! These structs are the single source of truth for the report format. The JSON
//! Schema, the zod schema and the TypeScript types are all generated from them.
//!
//! Data structs use `#[serde(default)]`: any single source can fail, so every
//! fact is optional. The report envelope and each section's `check` are required.

pub mod fit;
pub mod placeholders;
pub mod redact;
pub mod startup_names;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The longest each kind of string in a report may be, in UTF-16 code units (how the upload check
/// counts). Shared reports are checked against these, so the collector clamps to them too.
pub mod limits {
    /// Names and models: products, disks, memory, startup apps, graphics cards, processors, boards, displays.
    pub const NAME: usize = 128;
    /// Versions, dates, editions and drive letters.
    pub const SHORT: usize = 64;
    /// Error text, in reports from Fettle 1.0.3 and earlier (newer ones have a code instead).
    pub const DETAIL: usize = 256;
    /// Entries in any list (disks, startup apps…), as the upload check counts them.
    pub const ITEMS: usize = 256;
}

/// Bumped whenever the shape changes in a way old readers can't handle.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Report {
    pub schema_version: u32,
    #[schemars(length(max = limits::SHORT))]
    pub app_version: String,
    /// Version of the verdict engine the app used, so a shared report can show the same verdicts.
    #[serde(default)]
    #[schemars(length(max = limits::SHORT))]
    pub engine_version: Option<String>,
    /// RFC 3339 timestamp.
    #[schemars(regex(pattern = r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"))]
    #[schemars(length(max = limits::SHORT))]
    pub collected_at: String,
    pub elevated: bool,
    pub facts: Facts,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Facts {
    pub system: Option<Section<System>>,
    pub storage: Option<Section<Storage>>,
    pub battery: Option<Section<Battery>>,
    pub memory: Option<Section<Memory>>,
    pub windows: Option<Section<Windows>>,
    pub startup: Option<Section<Startup>>,
    pub graphics: Option<Section<Graphics>>,
}

/// One report section: whether it could be read, and what was found.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[schemars(rename = "{T}Section")]
pub struct Section<T> {
    pub check: Check,
    #[serde(default)]
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Check {
    pub status: CheckStatus,
    /// Why the section couldn't be read (since 1.0.4).
    #[serde(default)]
    pub code: Option<CheckCode>,
    /// Windows' own error text, as reports from Fettle 1.0.3 and earlier carried it. Newer reports
    /// leave it out (they have `code`); older ones keep it, cleaned by the privacy pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = limits::DETAIL))]
    pub detail: Option<String>,
}

/// Why a section couldn't be read: a code, so no text from Windows travels in a report.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckCode {
    /// Windows didn't answer in time.
    TimedOut,
    /// Windows refused to tell this account.
    AccessDenied,
    /// This PC or this version of Windows doesn't offer it.
    NotAvailable,
    /// Fettle's own code failed while reading it.
    Crashed,
    /// Any other error.
    Other,
}

/// "Not present" (a desktop has no battery) and "needs admin" must stay different.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    NeedsAdmin,
    NotPresent,
    Failed,
}

// ---------- storage ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Storage {
    pub disks: Vec<Disk>,
    pub volumes: Vec<Volume>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Disk {
    #[schemars(length(max = limits::NAME))]
    pub model: Option<String>,
    pub bus: Option<Bus>,
    pub media: Option<Media>,
    pub size_bytes: Option<u64>,
    pub nvme: Option<NvmeHealth>,
    pub smart: Option<SmartSummary>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bus {
    Nvme,
    Sata,
    Usb,
    Other,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Media {
    Ssd,
    Hdd,
    Unknown,
}

/// From the NVMe SMART / Health Information log (page 0x02).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct NvmeHealth {
    /// Can exceed 100 (the spec allows up to 255).
    pub percentage_used: Option<u8>,
    pub temperature_c: Option<i32>,
    pub power_on_hours: Option<u64>,
    pub media_errors: Option<u64>,
    pub critical_warning: Option<u8>,
    pub available_spare_pct: Option<u8>,
    pub available_spare_threshold_pct: Option<u8>,
}

/// Summary of ATA SMART attributes (provisional: confirm on the HDD test PC).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct SmartSummary {
    pub reallocated_sectors: Option<u64>,
    pub pending_sectors: Option<u64>,
    pub offline_uncorrectable: Option<u64>,
    pub power_on_hours: Option<u64>,
    pub temperature_c: Option<i32>,
    /// From vendor attributes (e.g. Crucial 202, Samsung 177), when known.
    pub life_used_pct: Option<u8>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Volume {
    #[schemars(length(max = limits::SHORT))]
    pub letter: Option<String>,
    pub size_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    /// How the disk this volume is on is connected, so a USB drive's volumes can be told apart.
    /// Missing in reports from before 1.0.5.
    pub bus: Option<Bus>,
}

// ---------- battery (provisional: confirm on the laptop test PC) ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Battery {
    pub design_capacity_mwh: Option<u32>,
    pub full_charge_capacity_mwh: Option<u32>,
    pub cycle_count: Option<u32>,
}

// ---------- memory ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Memory {
    pub modules: Vec<MemoryModule>,
    pub slots_total: Option<u32>,
}

/// Speed fields are provisional until a machine with XMP/EXPO off has been seen.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct MemoryModule {
    pub capacity_bytes: Option<u64>,
    pub speed_mts: Option<u32>,
    pub configured_speed_mts: Option<u32>,
    #[schemars(length(max = limits::NAME))]
    pub part_number: Option<String>,
    pub kind: Option<MemoryKind>,
    #[schemars(length(max = limits::NAME))]
    pub manufacturer: Option<String>,
    /// Firmware slot labels, e.g. "P0 CHANNEL A" + "DIMM 1" (AMD) or "BANK 0" + "ChannelA-DIMM0" (Intel).
    #[schemars(length(max = limits::NAME))]
    pub bank_label: Option<String>,
    #[schemars(length(max = limits::NAME))]
    pub device_locator: Option<String>,
    pub configured_voltage_mv: Option<u32>,
    pub form_factor: Option<FormFactor>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Ddr3,
    Ddr4,
    Ddr5,
    /// Soldered laptop memory.
    Lpddr3,
    Lpddr4,
    Lpddr5,
    Other,
}

/// Desktop (DIMM) or laptop (SODIMM) stick.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FormFactor {
    Dimm,
    Sodimm,
    Other,
}

// ---------- windows ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Windows {
    #[schemars(length(max = limits::NAME))]
    pub product: Option<String>,
    #[schemars(length(max = limits::SHORT))]
    pub edition_id: Option<String>,
    pub build: Option<u32>,
    pub ubr: Option<u32>,
    #[schemars(length(max = limits::SHORT))]
    pub display_version: Option<String>,
    #[schemars(length(max = limits::SHORT))]
    pub installation_type: Option<String>,
    pub uptime_seconds: Option<u64>,
    /// Date (YYYY-MM-DD) of the most recent installed update.
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
    #[schemars(length(max = limits::SHORT))]
    pub last_update_installed: Option<String>,
    pub secure_boot: Option<bool>,
    pub tpm: Option<Tpm>,
    /// Provisional: confirm the signal on the Windows 10 work laptop.
    pub esu: Option<EsuState>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Tpm {
    pub present: Option<bool>,
    #[schemars(length(max = limits::SHORT))]
    pub spec_version: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EsuState {
    Enrolled,
    NotEnrolled,
    Unknown,
}

// ---------- startup ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Startup {
    pub items: Vec<StartupItem>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct StartupItem {
    #[schemars(length(max = limits::NAME))]
    pub name: Option<String>,
    pub location: Option<StartupLocation>,
    /// Task Manager's enabled flag; `None` when Windows has no record of it.
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartupLocation {
    RunKey,
    RunOnce,
    Folder,
}

// ---------- graphics ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Graphics {
    pub adapters: Vec<Gpu>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Gpu {
    #[schemars(length(max = limits::NAME))]
    pub name: Option<String>,
    pub vram_bytes: Option<u64>,
    #[schemars(length(max = limits::SHORT))]
    pub driver_version: Option<String>,
    /// YYYY-MM-DD.
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
    #[schemars(length(max = limits::SHORT))]
    pub driver_date: Option<String>,
    pub temperature_c: Option<i32>,
    pub software: Option<bool>,
}

// ---------- system ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct System {
    #[schemars(length(max = limits::NAME))]
    pub cpu_name: Option<String>,
    pub cores: Option<u32>,
    pub threads: Option<u32>,
    #[schemars(length(max = limits::NAME))]
    pub board: Option<String>,
    #[schemars(length(max = limits::SHORT))]
    pub bios_version: Option<String>,
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
    #[schemars(length(max = limits::SHORT))]
    pub bios_date: Option<String>,
    pub chassis: Option<Chassis>,
    pub displays: Vec<Display>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Chassis {
    Desktop,
    Laptop,
    Unknown,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Display {
    #[schemars(length(max = limits::NAME))]
    pub name: Option<String>,
    pub width_cm: Option<u32>,
    pub height_cm: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_every_section() {
        let schema = serde_json::to_string(&schemars::schema_for!(Report)).unwrap();
        for name in [
            "\"schema_version\"",
            "\"facts\"",
            "\"storage\"",
            "\"battery\"",
            "\"memory\"",
            "\"windows\"",
            "\"startup\"",
            "\"graphics\"",
            "\"system\"",
            "\"needs_admin\"",
        ] {
            assert!(schema.contains(name), "schema is missing {name}");
        }
    }

    #[test]
    fn partial_report_round_trips() {
        let json = r#"{"schema_version":1,"app_version":"0.0.0","collected_at":"2026-10-06T23:00:00Z","elevated":false,"facts":{"battery":{"check":{"status":"not_present"}}}}"#;
        let report: Report = serde_json::from_str(json).unwrap();
        assert_eq!(report.facts.battery.unwrap().check.status, CheckStatus::NotPresent);
        assert!(report.facts.storage.is_none());
    }

    // Why a section failed travels as a short code: no text from Windows can ride along.
    #[test]
    fn a_failed_check_says_why_with_a_code_and_no_text() {
        let check = Check { status: CheckStatus::Failed, code: Some(CheckCode::TimedOut), detail: None };
        let json = serde_json::to_value(&check).unwrap();
        assert_eq!(json, serde_json::json!({ "status": "failed", "code": "timed_out" }));
        assert_eq!(serde_json::from_value::<Check>(json).unwrap(), check);
        let codes = ["timed_out", "access_denied", "not_available", "crashed", "other"];
        for code in codes {
            let check: Check = serde_json::from_value(serde_json::json!({ "status": "failed", "code": code })).unwrap();
            assert_eq!(serde_json::to_value(check.code).unwrap(), code);
        }
        let schema = serde_json::to_string(&schemars::schema_for!(Report)).unwrap();
        assert!(codes.iter().all(|c| schema.contains(&format!("\"{c}\""))), "the schema lists every code");
    }

    // Reports from Fettle 1.0.3 and earlier carry Windows' own error text: they still read, and keep it.
    #[test]
    fn an_older_report_s_error_text_still_reads() {
        let json = serde_json::json!({ "status": "failed", "detail": "Access is denied." });
        let check: Check = serde_json::from_value(json.clone()).unwrap();
        assert_eq!((check.code, check.detail.as_deref()), (None, Some("Access is denied.")));
        assert_eq!(serde_json::to_value(&check).unwrap(), serde_json::json!({ "status": "failed", "code": null, "detail": "Access is denied." }));
    }
}
