//! The Windows data sources. Each returns raw JSON, in the shape a saved scan keeps (crate::dump).
//!
//! Every WQL query names its columns: only what `map.rs` reads is selected. That keeps
//! `Win32_Processor` fast (`SELECT *` costs about 1 s) and keeps identifying fields such as serials
//! out of the raw data. (When an older Windows lacks a named column, `wmi.rs` falls back to every
//! column and drops all but the named ones straight away, before any JSON is built.)

mod elevation;
mod gpu;
pub(crate) mod nvme;
mod startup;
mod storage;
mod windows;
mod wmi;

pub use elevation::is_elevated;
/// For the app's private details (crate::private), which read outside the report on purpose.
pub(crate) use wmi::wmi_query;

use crate::Source;

const CIMV2: &str = r"ROOT\CIMV2";
const WMI: &str = r"ROOT\WMI";
const STORAGE: &str = r"ROOT\Microsoft\Windows\Storage";
const TPM: &str = r"ROOT\CIMV2\Security\MicrosoftTpm";

/// (source name, namespace, WQL). The names are the keys of a saved scan's sources (tests/dumps).
pub const WMI_SOURCES: &[(&str, &str, &str)] = &[
    ("os", CIMV2, "SELECT Caption, LastBootUpTime FROM Win32_OperatingSystem"),
    ("computer_system", CIMV2, "SELECT PCSystemType, PCSystemTypeEx FROM Win32_ComputerSystem"),
    ("cpu", CIMV2, "SELECT Name, NumberOfCores, NumberOfLogicalProcessors FROM Win32_Processor"),
    ("baseboard", CIMV2, "SELECT Manufacturer, Product FROM Win32_BaseBoard"),
    ("bios", CIMV2, "SELECT SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS"),
    (
        "memory_modules",
        CIMV2,
        "SELECT Capacity, Speed, ConfiguredClockSpeed, PartNumber, SMBIOSMemoryType, Manufacturer, BankLabel, \
         DeviceLocator, ConfiguredVoltage, FormFactor FROM Win32_PhysicalMemory",
    ),
    ("memory_array", CIMV2, "SELECT MemoryDevices FROM Win32_PhysicalMemoryArray"),
    ("video_controllers", CIMV2, "SELECT Name, DriverVersion, DriverDate FROM Win32_VideoController"),
    ("monitor_ids", WMI, "SELECT InstanceName, UserFriendlyName FROM WmiMonitorID"),
    ("monitor_params", WMI, "SELECT InstanceName, MaxHorizontalImageSize, MaxVerticalImageSize FROM WmiMonitorBasicDisplayParams"),
    ("logical_disks", CIMV2, "SELECT DeviceID, Size, FreeSpace FROM Win32_LogicalDisk WHERE DriveType = 3"),
    ("physical_disks", STORAGE, "SELECT DeviceId, FriendlyName, BusType, MediaType, Size FROM MSFT_PhysicalDisk"),
    // Which disk each drive letter is on, so a USB drive's volumes can be told apart.
    ("partitions", STORAGE, "SELECT DiskNumber, DriveLetter FROM MSFT_Partition"),
    // Only whether a battery exists: Win32_Battery.DeviceID is usually serial + maker + name.
    ("battery_cimv2", CIMV2, "SELECT Name FROM Win32_Battery"),
    // The raw ATA SMART attribute table (reallocated and pending sectors live here).
    ("smart_predict_data", WMI, "SELECT InstanceName, VendorSpecific FROM MSStorageDriver_FailurePredictData"),
    // Which disk number each SMART row belongs to (its InstanceName is PNPDeviceID + "_0"), so two
    // drives of the same model never share one row. Read only for matching; never in the report.
    ("disk_drives", CIMV2, "SELECT Index, PNPDeviceID FROM Win32_DiskDrive"),
    // InstanceName ("ACPI\PNP0C0A\1_0") pairs the three battery rows when there are two batteries.
    // Read only for matching; never in the report.
    ("battery_static", WMI, "SELECT InstanceName, DesignedCapacity FROM BatteryStaticData"),
    ("battery_full_capacity", WMI, "SELECT InstanceName, FullChargedCapacity FROM BatteryFullChargedCapacity"),
    ("battery_cycle_count", WMI, "SELECT InstanceName, CycleCount FROM BatteryCycleCount"),
    ("hotfixes", CIMV2, "SELECT InstalledOn FROM Win32_QuickFixEngineering"),
];

/// Non-elevated, the TPM query fails slowly (about 5 s), so it only runs for a Deep check.
const TPM_SOURCE: (&str, &str, &str) = ("tpm", TPM, "SELECT SpecVersion FROM Win32_Tpm");

/// Every source for one scan.
pub fn sources(elevated: bool) -> Vec<(String, Source)> {
    let mut list: Vec<(String, Source)> = Vec::new();
    let tpm = elevated.then_some(&TPM_SOURCE);
    for &(name, namespace, wql) in WMI_SOURCES.iter().chain(tpm) {
        list.push((name.into(), Box::new(move || wmi::wmi_query(namespace, wql))));
    }
    list.push(("nvme_health".into(), Box::new(storage::nvme_health)));
    list.push(("windows_registry".into(), Box::new(windows::windows_registry)));
    list.push(("secure_boot".into(), Box::new(windows::secure_boot)));
    list.push(("startup_registry".into(), Box::new(startup::startup_registry)));
    list.push(("startup_folders".into(), Box::new(startup::startup_folders)));
    list.push(("startup_approved".into(), Box::new(startup::startup_approved)));
    list.push(("dxgi_adapters".into(), Box::new(gpu::dxgi_adapters)));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_query_selects_everything_or_a_product_key() {
        for &(name, _, wql) in WMI_SOURCES.iter().chain([&TPM_SOURCE]) {
            assert!(!wql.contains('*'), "{name}");
            assert!(!wql.to_ascii_lowercase().contains("productkey"), "{name}");
            assert!(!wql.to_ascii_lowercase().contains("serial"), "{name}");
        }
    }

    #[test]
    fn the_tpm_is_only_asked_for_a_deep_check() {
        let names = |elevated| sources(elevated).into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        assert!(!names(false).contains(&"tpm".to_string()));
        assert!(names(true).contains(&"tpm".to_string()));
    }

    /// The test dumps' rows carry every column `SELECT *` returns, the ones our queries don't name
    /// holding "FAKE" decoys. Cut each WMI source down to the columns our queries name: if the
    /// mapping reads any column we don't select, the facts come out different.
    #[test]
    fn the_mapping_only_reads_columns_the_queries_select() {
        use crate::dump::Dump;
        let columns = |wql: &str| -> Vec<String> {
            let list = wql.trim_start_matches("SELECT ").split(" FROM ").next().unwrap();
            list.split(',').map(|c| c.trim().to_string()).collect()
        };
        for text in [
            include_str!("../../tests/dumps/dev-workstation-admin-1791282900.json"),
            include_str!("../../tests/dumps/dev-workstation-user-1791282600.json"),
            // A Windows 10 laptop: battery, TPM and SO-DIMM rows.
            include_str!("../../tests/dumps/laptop-win10-admin-1791385440.json"),
        ] {
            let dump = Dump::from_json(text).unwrap();
            let mut cut = dump.sources.clone();
            for &(name, _, wql) in WMI_SOURCES.iter().chain([&TPM_SOURCE]) {
                let keep = columns(wql);
                let Some(rows) = cut.get_mut(name).and_then(|r| r.data.as_mut()).and_then(|d| d.as_array_mut()) else { continue };
                for row in rows.iter_mut().filter_map(|r| r.as_object_mut()) {
                    row.retain(|k, _| keep.contains(k));
                }
            }
            let at = dump.collected_at();
            assert_eq!(crate::map::facts(&cut, at), crate::map::facts(&dump.sources, at));
        }
        // Battery rows with other columns WMI has, made up here.
        let row = |v: serde_json::Value| crate::SourceResult { ok: true, ms: 1, data: Some(v), error: None };
        let full: crate::Raw = [
            ("battery_cimv2", serde_json::json!([{ "Name": "Battery", "DeviceID": "123SERIAL", "Chemistry": 6 }])),
            ("battery_static", serde_json::json!([{ "DesignedCapacity": 45000, "SerialNumber": "X", "ManufactureName": "LG" }])),
            ("battery_full_capacity", serde_json::json!([{ "FullChargedCapacity": 26100, "Tag": 1 }])),
            ("battery_cycle_count", serde_json::json!([{ "CycleCount": 611, "Tag": 1 }])),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), row(v)))
        .collect();
        let mut cut = full.clone();
        for &(name, _, wql) in WMI_SOURCES {
            let keep = columns(wql);
            if let Some(rows) = cut.get_mut(name).and_then(|r| r.data.as_mut()).and_then(|d| d.as_array_mut()) {
                rows.iter_mut().filter_map(|r| r.as_object_mut()).for_each(|r| r.retain(|k, _| keep.contains(k)));
            }
        }
        let at = std::time::SystemTime::now();
        assert_eq!(crate::map::facts(&cut, at).battery, crate::map::facts(&full, at).battery);
        assert_eq!(crate::map::facts(&cut, at).battery.unwrap().data.unwrap().cycle_count, Some(611));
    }

    /// Scans this PC for real: `cargo test -p fettle-collect -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn scans_this_pc() {
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let raw = crate::run_all(sources(is_elevated()), Duration::from_secs(4));
        let took = start.elapsed();
        for (name, r) in &raw {
            eprintln!("{name:<22} {:<4} {:>5} ms  {}", if r.ok { "ok" } else { "FAIL" }, r.ms, r.error.as_deref().unwrap_or(""));
        }
        eprintln!("total {} ms", took.as_millis());
        assert!(took < Duration::from_secs(5));
        for name in ["os", "cpu", "memory_modules", "physical_disks", "dxgi_adapters", "windows_registry", "startup_registry"] {
            assert!(raw[name].ok, "{name}: {:?}", raw[name].error);
        }
        let facts = crate::map::facts(&raw, std::time::SystemTime::now());
        let system = facts.system.unwrap().data.unwrap();
        assert!(system.cpu_name.is_some() && system.board.is_some() && system.bios_date.is_some(), "{system:?}");
        let memory = facts.memory.unwrap().data.unwrap();
        assert!(memory.modules.iter().all(|m| m.capacity_bytes.is_some() && m.part_number.is_some()), "{memory:?}");
        let storage = facts.storage.unwrap().data.unwrap();
        assert!(storage.disks.iter().all(|d| d.model.is_some() && d.size_bytes.is_some()), "{storage:?}");
        assert!(!storage.volumes.is_empty());
        let windows = facts.windows.unwrap().data.unwrap();
        assert!(windows.build.is_some() && windows.uptime_seconds.is_some() && windows.last_update_installed.is_some(), "{windows:?}");
    }
}
