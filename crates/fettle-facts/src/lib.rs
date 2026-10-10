//! The facts a Fettle scan collects: raw values only, no verdicts (those are fettle-verdicts').
//! These structs are the single source of truth for the report format. The JSON
//! Schema, the zod schema and the TypeScript types are all generated from them.
//!
//! Data structs use `#[serde(default)]`: any single source can fail, so every
//! fact is optional. The report envelope and each section's `check` are required.

pub mod redact;
pub mod startup_names;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Bumped whenever the shape changes in a way old readers can't handle.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Report {
    pub schema_version: u32,
    pub app_version: String,
    /// Version of the verdict engine the app used, so a shared report can show the same verdicts.
    #[serde(default)]
    pub engine_version: Option<String>,
    /// RFC 3339 timestamp.
    #[schemars(regex(pattern = r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"))]
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
    #[serde(default)]
    pub detail: Option<String>,
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
    pub letter: Option<String>,
    pub size_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
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
    pub part_number: Option<String>,
    pub kind: Option<MemoryKind>,
    pub manufacturer: Option<String>,
    /// Firmware slot labels, e.g. "P0 CHANNEL A" + "DIMM 1" (AMD) or "BANK 0" + "ChannelA-DIMM0" (Intel).
    pub bank_label: Option<String>,
    pub device_locator: Option<String>,
    pub configured_voltage_mv: Option<u32>,
    pub form_factor: Option<FormFactor>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Ddr4,
    Ddr5,
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
    pub product: Option<String>,
    pub edition_id: Option<String>,
    pub build: Option<u32>,
    pub ubr: Option<u32>,
    pub display_version: Option<String>,
    pub installation_type: Option<String>,
    pub uptime_seconds: Option<u64>,
    /// Date (YYYY-MM-DD) of the most recent installed update.
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
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
    pub name: Option<String>,
    pub vram_bytes: Option<u64>,
    pub driver_version: Option<String>,
    /// YYYY-MM-DD.
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
    pub driver_date: Option<String>,
    pub temperature_c: Option<i32>,
    pub software: Option<bool>,
}

// ---------- system ----------

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct System {
    pub cpu_name: Option<String>,
    pub cores: Option<u32>,
    pub threads: Option<u32>,
    pub board: Option<String>,
    pub bios_version: Option<String>,
    #[schemars(regex(pattern = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$"))]
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
}
