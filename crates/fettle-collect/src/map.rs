//! Raw sources to typed facts: a port of `crates/fettle-receipt/parity/from-spike.mjs`, kept
//! function by function so the two stay easy to compare (tests/parity.rs holds them together).

use crate::{Raw, SourceResult};
use fettle_facts::*;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn facts(raw: &Raw, collected_at: SystemTime) -> Facts {
    let now = collected_at.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let s = Sources(raw);
    Facts {
        system: Some(system(&s)),
        storage: Some(storage(&s)),
        battery: Some(battery(&s)),
        memory: Some(memory(&s)),
        windows: Some(windows(&s, now)),
        startup: Some(startup(&s)),
        graphics: Some(graphics(&s)),
    }
}

struct Sources<'a>(&'a Raw);

impl Sources<'_> {
    fn get(&self, name: &str) -> Option<&SourceResult> {
        self.0.get(name)
    }
    /// A source's rows, when it worked and returned a list.
    fn rows(&self, name: &str) -> Option<&Vec<Value>> {
        self.get(name).filter(|r| r.ok).and_then(|r| r.data.as_ref()).and_then(Value::as_array)
    }
    fn first(&self, name: &str) -> Option<&Value> {
        self.rows(name).and_then(|r| r.first())
    }
    fn error(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|r| r.error.clone())
    }
    /// WMI's "access denied": the data is there for an administrator.
    fn access_denied(&self, name: &str) -> bool {
        self.error(name).is_some_and(|e| e.contains("0x80041003"))
    }
}

fn ok<T>(data: T) -> Section<T> {
    Section { check: Check { status: CheckStatus::Ok, detail: None }, data: Some(data) }
}
fn status<T>(status: CheckStatus, detail: Option<String>) -> Section<T> {
    Section { check: Check { status, detail }, data: None }
}

fn u64_of(v: &Value, key: &str) -> Option<u64> {
    let v = v.get(key)?;
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}
fn u32_of(v: &Value, key: &str) -> Option<u32> {
    u64_of(v, key).and_then(|n| n.try_into().ok())
}
fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}
/// Trimmed text, or None when there is nothing left.
fn text_of(v: &Value, key: &str) -> Option<String> {
    str_of(v, key).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// A CPU or graphics name without trademark marks: "Intel(R) Core(TM) i7" reads as "Intel Core i7".
fn product_name(v: &Value, key: &str) -> Option<String> {
    let mut name = text_of(v, key)?;
    for mark in ["(R)", "(r)", "(TM)", "(tm)", "®", "™"] {
        name = name.replace(mark, " ");
    }
    Some(name.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|s| !s.is_empty())
}

// ---------- dates ----------

/// Days since 1970-01-01 for a calendar date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// WMI CIM_DATETIME ("20261006131522.514770+120", offset in minutes) → Unix seconds.
fn wmi_date(s: &str) -> Option<i64> {
    let (stamp, rest) = s.split_once('.')?;
    let offset_at = rest.find(['+', '-'])?;
    let (fraction, offset) = rest.split_at(offset_at);
    if stamp.len() != 14 || !stamp.bytes().all(|b| b.is_ascii_digit()) || fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = |a: usize, b: usize| stamp[a..b].parse::<i64>().ok();
    let offset: i64 = offset.parse().ok()?;
    let days = days_from_civil(n(0, 4)?, n(4, 6)?, n(6, 8)?);
    Some(days * 86400 + n(8, 10)? * 3600 + n(10, 12)? * 60 + n(12, 14)? - offset * 60)
}

/// The calendar day a CIM_DATETIME names ("20260923000000.000000+120" → 2026-09-23). Release and
/// driver dates are days, so the offset must not move them across midnight.
fn wmi_day(s: &str) -> Option<String> {
    wmi_date(s)?;
    Some(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]))
}

/// Win32_QuickFixEngineering.InstalledOn ("9/29/2026") is a calendar day with no time zone.
fn hotfix_day(s: &str) -> Option<String> {
    let mut parts = s.split('/');
    let (m, d, y) = (parts.next()?, parts.next()?, parts.next()?);
    let digits = |p: &str, len: std::ops::RangeInclusive<usize>| len.contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit());
    (parts.next().is_none() && digits(m, 1..=2) && digits(d, 1..=2) && digits(y, 4..=4)).then(|| format!("{y}-{m:0>2}-{d:0>2}"))
}

// ---------- system (new: the JS converter has no system section) ----------

/// Read if any of its sources worked; failed (with the CPU query's error) only when none did.
fn system(s: &Sources) -> Section<System> {
    const PARTS: [&str; 4] = ["cpu", "baseboard", "bios", "computer_system"];
    if PARTS.iter().all(|p| s.rows(p).is_none()) {
        return status(CheckStatus::Failed, s.error("cpu"));
    }
    let cpu = s.first("cpu");
    let board = s.first("baseboard").and_then(|b| {
        let joined = [text_of(b, "Manufacturer"), text_of(b, "Product")].into_iter().flatten().collect::<Vec<_>>().join(" ");
        (!joined.is_empty()).then_some(joined)
    });
    let bios = s.first("bios");
    let params = s.rows("monitor_params").cloned().unwrap_or_default();
    let displays = s
        .rows("monitor_ids")
        .map(|ids| {
            ids.iter()
                .map(|m| {
                    let p = params.iter().find(|p| str_of(p, "InstanceName").is_some() && str_of(p, "InstanceName") == str_of(m, "InstanceName"));
                    Display {
                        name: monitor_name(m.get("UserFriendlyName")),
                        width_cm: p.and_then(|p| u32_of(p, "MaxHorizontalImageSize")).filter(|n| *n != 0),
                        height_cm: p.and_then(|p| u32_of(p, "MaxVerticalImageSize")).filter(|n| *n != 0),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    ok(System {
        cpu_name: cpu.and_then(|c| product_name(c, "Name")),
        cores: cpu.and_then(|c| u32_of(c, "NumberOfCores")),
        threads: cpu.and_then(|c| u32_of(c, "NumberOfLogicalProcessors")),
        board,
        bios_version: bios.and_then(|b| text_of(b, "SMBIOSBIOSVersion")),
        bios_date: bios.and_then(|b| str_of(b, "ReleaseDate")).and_then(wmi_day),
        // PCSystemTypeEx: 1 desktop, 2 mobile, 3 workstation, 8 slate (tablets and 2-in-1s). Older
        // systems only have PCSystemType, where 8 means "Maximum" (the end of the range), not slate.
        chassis: s.first("computer_system").map(|c| match (u64_of(c, "PCSystemTypeEx"), u64_of(c, "PCSystemType")) {
            (Some(1 | 3), _) | (None, Some(1 | 3)) => Chassis::Desktop,
            (Some(2 | 8), _) | (None, Some(2)) => Chassis::Laptop,
            _ => Chassis::Unknown,
        }),
        displays,
    })
}

/// WmiMonitorID names are UTF-16 code units, padded with zeros.
fn monitor_name(units: Option<&Value>) -> Option<String> {
    let units: Vec<u16> = units?.as_array()?.iter().filter_map(|u| u.as_u64()).take_while(|u| *u != 0).filter_map(|u| u16::try_from(u).ok()).collect();
    let name = String::from_utf16_lossy(&units).trim().to_string();
    (!name.is_empty()).then_some(name)
}

// ---------- storage ----------

/// The SMART attribute table: 12-byte entries from offset 2 (id, flags×2, value, worst, raw×6).
fn smart_attributes(bytes: &[u8]) -> Vec<(u8, u8, u64)> {
    let mut attrs = Vec::new();
    let mut i = 2;
    while i + 12 <= bytes.len() {
        let id = bytes[i];
        if id != 0 {
            let raw = bytes[i + 5..i + 11].iter().rev().fold(0u64, |acc, b| acc * 256 + u64::from(*b));
            attrs.push((id, bytes[i + 3], raw));
        }
        i += 12;
    }
    attrs
}

fn smart_summary(attrs: &[(u8, u8, u64)]) -> SmartSummary {
    let find = |id: u8| attrs.iter().find(|a| a.0 == id);
    let raw = |id: u8| find(id).map(|a| a.2);
    // Vendor wear attributes: Crucial 202 = % life used; Samsung 177 = normalized life left.
    let life_used = match (find(202), find(177)) {
        (Some(a), _) => Some(a.2.min(255) as u8),
        (None, Some(a)) => Some(100u8.saturating_sub(a.1)),
        _ => None,
    };
    SmartSummary {
        reallocated_sectors: raw(5),
        pending_sectors: raw(197),
        offline_uncorrectable: raw(198),
        power_on_hours: raw(9),
        temperature_c: raw(194).map(|r| (r % 256) as i32), // low byte = current °C
        life_used_pct: life_used,
    }
}

/// SMART rows name the drive as "SCSI\Disk&Ven_Samsung&Prod_SSD_860_EVO_1TB\…" while Windows calls
/// it "Samsung SSD 860 EVO 1TB", so match on the product part, ignoring punctuation.
fn smart_matches(instance_name: &str, model: &str) -> bool {
    let norm = |x: &str| x.to_uppercase().chars().filter(char::is_ascii_alphanumeric).collect::<String>();
    let lower = instance_name.to_ascii_lowercase();
    let Some(at) = lower.find("prod_") else { return false };
    let product = instance_name[at + 5..].split('\\').next().unwrap_or("");
    !product.is_empty() && norm(model).contains(&norm(product))
}

fn bus(code: Option<u64>) -> Bus {
    match code {
        Some(17) => Bus::Nvme,
        Some(11) => Bus::Sata,
        Some(7) => Bus::Usb,
        _ => Bus::Other,
    }
}

fn media(code: Option<u64>) -> Media {
    match code {
        Some(3) => Media::Hdd,
        Some(4) => Media::Ssd,
        _ => Media::Unknown,
    }
}

fn nvme_health(s: &Sources, device_id: &str) -> Option<NvmeHealth> {
    let drives = s.get("nvme_health")?.data.as_ref()?.as_array()?;
    let drive = drives.iter().find(|d| d.get("drive").map(value_key).as_deref() == Some(device_id))?;
    let health = drive.get("attempts")?.as_array()?.iter().find(|a| a.get("ok") == Some(&Value::Bool(true)))?.get("health")?;
    Some(NvmeHealth {
        percentage_used: u64_of(health, "percentage_used").map(|n| n.min(255) as u8),
        temperature_c: health.get("temperature_c").and_then(Value::as_i64).map(|t| t as i32),
        power_on_hours: u64_of(health, "power_on_hours"),
        media_errors: u64_of(health, "media_errors"),
        critical_warning: u64_of(health, "critical_warning").map(|n| n.min(255) as u8),
        available_spare_pct: u64_of(health, "available_spare_pct").map(|n| n.min(255) as u8),
        available_spare_threshold_pct: u64_of(health, "available_spare_threshold_pct").map(|n| n.min(255) as u8),
    })
}

/// The SMART table for the disk Windows numbers `device_id`. A SMART row's InstanceName is the
/// drive's PNPDeviceID plus "_<n>", and Win32_DiskDrive gives each PNPDeviceID its disk number. If
/// those numbers aren't available, the model name decides only when no other disk shares it, so a
/// twin never gets the other drive's data.
fn smart_for<'a>(s: &Sources, smart: &'a [Value], device_id: &str, model: Option<&str>, models: &[Option<String>]) -> Option<&'a Value> {
    if let Some(drives) = s.rows("disk_drives") {
        let pnp = drives.iter().find(|d| d.get("Index").map(value_key).as_deref() == Some(device_id)).and_then(|d| str_of(d, "PNPDeviceID"))?;
        let pnp = pnp.to_uppercase();
        return smart.iter().find(|r| {
            str_of(r, "InstanceName").is_some_and(|i| {
                let i = i.to_uppercase();
                i.strip_prefix(&pnp).and_then(|rest| rest.strip_prefix('_')).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        });
    }
    let model = model?;
    if models.iter().filter(|m| m.as_deref() == Some(model)).count() != 1 {
        return None;
    }
    let mut matching = smart.iter().filter(|r| str_of(r, "InstanceName").is_some_and(|i| smart_matches(i, model)));
    match (matching.next(), matching.next()) {
        (Some(row), None) => Some(row),
        _ => None,
    }
}

/// A JSON number or string as a lookup key ("0" and 0 match).
fn value_key(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
}

fn storage(s: &Sources) -> Section<Storage> {
    let Some(disks) = s.rows("physical_disks") else { return status(CheckStatus::Failed, s.error("physical_disks")) };
    let smart = s.rows("smart_predict_data").cloned().unwrap_or_default();
    let models: Vec<Option<String>> = disks.iter().map(|d| text_of(d, "FriendlyName")).collect();
    let disks = disks
        .iter()
        .map(|d| {
            let model = text_of(d, "FriendlyName");
            let device_id = d.get("DeviceId").map(value_key).unwrap_or_default();
            let smart_row = smart_for(s, &smart, &device_id, model.as_deref(), &models);
            let smart = smart_row.and_then(|r| r.get("VendorSpecific")?.as_array().map(|b| b.iter().filter_map(|x| x.as_u64().map(|n| n as u8)).collect::<Vec<u8>>()));
            let media = media(u64_of(d, "MediaType"));
            Disk {
                bus: Some(bus(u64_of(d, "BusType"))),
                media: Some(media),
                size_bytes: u64_of(d, "Size"),
                nvme: nvme_health(s, &device_id),
                // Wear attributes mean "% life used" only on SSDs (202 counts errors on some hard drives).
                smart: smart.map(|b| smart_summary(&smart_attributes(&b))).map(|m| SmartSummary { life_used_pct: m.life_used_pct.filter(|_| media == Media::Ssd), ..m }),
                model,
            }
        })
        .collect();
    let volumes = s
        .rows("logical_disks")
        .map(|rows| rows.iter().map(|v| Volume { letter: text_of(v, "DeviceID"), size_bytes: u64_of(v, "Size"), free_bytes: u64_of(v, "FreeSpace") }).collect())
        .unwrap_or_default();
    ok(Storage { disks, volumes })
}

// ---------- battery ----------

fn battery(s: &Sources) -> Section<Battery> {
    if s.rows("battery_cimv2").is_some_and(Vec::is_empty) {
        return status(CheckStatus::NotPresent, None);
    }
    let design = s.first("battery_static").and_then(|r| u32_of(r, "DesignedCapacity"));
    let full = s.first("battery_full_capacity").and_then(|r| u32_of(r, "FullChargedCapacity"));
    let cycles = s.first("battery_cycle_count").and_then(|r| u32_of(r, "CycleCount"));
    if design.is_none() && full.is_none() {
        return if s.access_denied("battery_static") { status(CheckStatus::NeedsAdmin, None) } else { status(CheckStatus::Failed, s.error("battery_static")) };
    }
    ok(Battery { design_capacity_mwh: design, full_charge_capacity_mwh: full, cycle_count: cycles })
}

// ---------- memory ----------

fn memory(s: &Sources) -> Section<Memory> {
    let Some(modules) = s.rows("memory_modules") else { return status(CheckStatus::Failed, s.error("memory_modules")) };
    let modules = modules
        .iter()
        .map(|m| MemoryModule {
            capacity_bytes: u64_of(m, "Capacity"),
            speed_mts: u32_of(m, "Speed"),
            configured_speed_mts: u32_of(m, "ConfiguredClockSpeed"),
            part_number: text_of(m, "PartNumber"),
            kind: Some(match u64_of(m, "SMBIOSMemoryType") {
                // SMBIOS type codes (LPDDR4X and LPDDR5X report as LPDDR4 and LPDDR5).
                Some(24) => MemoryKind::Ddr3,
                Some(26) => MemoryKind::Ddr4,
                Some(34) => MemoryKind::Ddr5,
                Some(29) => MemoryKind::Lpddr3,
                Some(30) => MemoryKind::Lpddr4,
                Some(35) => MemoryKind::Lpddr5,
                _ => MemoryKind::Other,
            }),
            // Some firmware writes the word "Unknown" instead of leaving it empty.
            manufacturer: text_of(m, "Manufacturer").filter(|s| !s.eq_ignore_ascii_case("unknown")),
            bank_label: text_of(m, "BankLabel"),
            device_locator: text_of(m, "DeviceLocator"),
            configured_voltage_mv: u32_of(m, "ConfiguredVoltage").filter(|v| *v != 0),
            // 0 is "Unknown" (some laptops report it for their SO-DIMMs): not known, not "other".
            form_factor: u64_of(m, "FormFactor").filter(|f| *f != 0).map(|f| match f {
                8 => FormFactor::Dimm,
                12 => FormFactor::Sodimm,
                _ => FormFactor::Other,
            }),
        })
        .collect();
    ok(Memory { modules, slots_total: s.first("memory_array").and_then(|a| u32_of(a, "MemoryDevices")) })
}

// ---------- windows ----------

fn windows(s: &Sources, now: i64) -> Section<Windows> {
    let reg = s.get("windows_registry").filter(|r| r.ok).and_then(|r| r.data.as_ref());
    let (Some(reg), Some(os)) = (reg, s.first("os")) else {
        return status(CheckStatus::Failed, s.error("windows_registry").or_else(|| s.error("os")));
    };
    let boot = str_of(os, "LastBootUpTime").and_then(wmi_date);
    let last_update = s.rows("hotfixes").and_then(|rows| rows.iter().filter_map(|h| str_of(h, "InstalledOn").and_then(hotfix_day)).max());
    let secure_boot = s.get("secure_boot").filter(|r| r.ok).map(|r| r.data.as_ref().and_then(|d| u64_of(d, "uefi_secure_boot_enabled")) == Some(1));
    ok(Windows {
        // The registry's ProductName says "Windows 10" on Windows 11; the OS caption doesn't.
        product: text_of(os, "Caption").map(|c| c.strip_prefix("Microsoft ").map(str::to_string).unwrap_or(c)),
        edition_id: text_of(reg, "EditionID"),
        build: u32_of(reg, "CurrentBuild"),
        ubr: u32_of(reg, "UBR"),
        display_version: text_of(reg, "DisplayVersion").or_else(|| text_of(reg, "ReleaseId")),
        installation_type: text_of(reg, "InstallationType"),
        uptime_seconds: boot.map(|b| (now - b).max(0) as u64),
        last_update_installed: last_update,
        secure_boot,
        tpm: s.first("tpm").map(|t| Tpm { present: Some(true), spec_version: text_of(t, "SpecVersion") }),
        esu: None,
    })
}

// ---------- startup ----------

fn startup(s: &Sources) -> Section<Startup> {
    let Some(reg) = s.rows("startup_registry") else { return status(CheckStatus::Failed, s.error("startup_registry")) };
    let approved = s.rows("startup_approved").cloned().unwrap_or_default();
    let enabled = |hive: &str, key: &str, name: &str| {
        approved
            .iter()
            .find(|a| str_of(a, "hive") == Some(hive) && str_of(a, "key") == Some(key) && str_of(a, "name") == Some(name))
            .and_then(|a| a.get("enabled")?.as_bool())
    };
    let mut items: Vec<StartupItem> = reg
        .iter()
        .filter(|r| r.get("error").is_none_or(Value::is_null))
        .map(|r| {
            let key = str_of(r, "key").unwrap_or("");
            let name = str_of(r, "name").unwrap_or("");
            let once = key.ends_with("RunOnce");
            let approved_key = if key.to_ascii_lowercase().contains("wow6432node") { "Run32" } else { "Run" };
            StartupItem {
                name: str_of(r, "name").map(str::to_string),
                location: Some(if once { StartupLocation::RunOnce } else { StartupLocation::RunKey }),
                enabled: if once { None } else { enabled(str_of(r, "hive").unwrap_or(""), approved_key, name) },
            }
        })
        .collect();
    for f in s.rows("startup_folders").map(Vec::as_slice).unwrap_or_default() {
        if f.get("error").is_some_and(|e| !e.is_null()) {
            continue;
        }
        let hive = if str_of(f, "folder") == Some("user") { "HKCU" } else { "HKLM" };
        let file = str_of(f, "file").unwrap_or("");
        items.push(StartupItem { name: str_of(f, "file").map(str::to_string), location: Some(StartupLocation::Folder), enabled: enabled(hive, "StartupFolder", file) });
    }
    ok(Startup { items })
}

// ---------- graphics ----------

fn graphics(s: &Sources) -> Section<Graphics> {
    let Some(adapters) = s.rows("dxgi_adapters") else { return status(CheckStatus::Failed, s.error("dxgi_adapters")) };
    let controllers = s.rows("video_controllers").cloned().unwrap_or_default();
    let adapters = adapters
        .iter()
        .map(|a| {
            let name = str_of(a, "description");
            let wmi = controllers.iter().find(|c| name.is_some() && str_of(c, "Name") == name);
            Gpu {
                name: product_name(a, "description"),
                vram_bytes: u64_of(a, "dedicated_video_memory"),
                driver_version: wmi.and_then(|c| text_of(c, "DriverVersion")),
                driver_date: wmi.and_then(|c| str_of(c, "DriverDate")).and_then(wmi_day),
                temperature_c: None,
                software: a.get("software").and_then(Value::as_bool),
            }
        })
        .collect();
    ok(Graphics { adapters })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raw(entries: &[(&str, Value)]) -> Raw {
        entries.iter().map(|(k, v)| (k.to_string(), serde_json::from_value(v.clone()).unwrap())).collect()
    }
    fn rows(data: Value) -> Value {
        json!({ "ok": true, "ms": 1, "data": data, "error": null })
    }
    fn err(e: &str) -> Value {
        json!({ "ok": false, "ms": 1, "data": null, "error": e })
    }

    #[test]
    fn dates() {
        assert_eq!(wmi_date("20261006131522.514770+120"), Some(days_from_civil(2026, 10, 6) * 86400 + 11 * 3600 + 15 * 60 + 22));
        assert_eq!(wmi_day("20260925000000.000000-000").as_deref(), Some("2026-09-25"));
        assert_eq!(wmi_date("garbage"), None);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(hotfix_day("9/29/2026").as_deref(), Some("2026-09-29"));
        assert_eq!(hotfix_day("12/1/2025").as_deref(), Some("2025-12-01"));
        assert_eq!(hotfix_day("2026-09-29"), None);
    }

    #[test]
    fn bios_and_driver_dates_are_calendar_days() {
        let r = raw(&[
            ("bios", rows(json!([{ "SMBIOSBIOSVersion": "F8a", "ReleaseDate": "20260923000000.000000+120" }]))),
            ("dxgi_adapters", rows(json!([{ "description": "GPU" }]))),
            ("video_controllers", rows(json!([{ "Name": "GPU", "DriverDate": "20260925000000.000000+060" }]))),
        ]);
        assert_eq!(system(&Sources(&r)).data.unwrap().bios_date.as_deref(), Some("2026-09-23"));
        assert_eq!(graphics(&Sources(&r)).data.unwrap().adapters[0].driver_date.as_deref(), Some("2026-09-25"));
    }

    #[test]
    fn empty_text_becomes_none_everywhere() {
        let r = raw(&[
            ("os", rows(json!([{ "Caption": "" }]))),
            ("windows_registry", rows(json!({ "EditionID": " ", "DisplayVersion": "", "InstallationType": "" }))),
            ("tpm", rows(json!([{ "SpecVersion": "" }]))),
            ("dxgi_adapters", rows(json!([{ "description": "" }]))),
        ]);
        let w = windows(&Sources(&r), 0).data.unwrap();
        assert_eq!((w.product, w.edition_id, w.display_version, w.installation_type), (None, None, None, None));
        assert_eq!(w.tpm.unwrap().spec_version, None);
        assert_eq!(graphics(&Sources(&r)).data.unwrap().adapters[0].name, None);
    }

    #[test]
    fn an_unknown_memory_form_factor_stays_unknown() {
        // Seen on a Lenovo laptop: its SO-DIMM reports FormFactor 0 ("Unknown").
        let r = raw(&[("memory_modules", rows(json!([{ "FormFactor": 0 }, { "FormFactor": 12 }, { "FormFactor": 1 }])))]);
        let forms: Vec<_> = memory(&Sources(&r)).data.unwrap().modules.into_iter().map(|m| m.form_factor).collect();
        assert_eq!(forms, [None, Some(FormFactor::Sodimm), Some(FormFactor::Other)]);
    }

    #[test]
    fn trademark_marks_are_dropped_from_cpu_and_graphics_names() {
        let r = raw(&[
            ("cpu", rows(json!([{ "Name": "Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz" }]))),
            ("dxgi_adapters", rows(json!([{ "description": "AMD Radeon(TM) Graphics" }, { "description": "NVIDIA GeForce RTX™ 4090" }]))),
            ("video_controllers", rows(json!([{ "Name": "AMD Radeon(TM) Graphics", "DriverVersion": "32.0" }]))),
        ]);
        assert_eq!(system(&Sources(&r)).data.unwrap().cpu_name.as_deref(), Some("Intel Core i7-8700 CPU @ 3.20GHz"));
        let gpus = graphics(&Sources(&r)).data.unwrap().adapters;
        assert_eq!(gpus.iter().map(|g| g.name.as_deref()).collect::<Vec<_>>(), [Some("AMD Radeon Graphics"), Some("NVIDIA GeForce RTX 4090")]);
        // The driver is still found under Windows' own spelling of the name.
        assert_eq!(gpus[0].driver_version.as_deref(), Some("32.0"));
    }

    #[test]
    fn an_unknown_memory_maker_stays_unknown() {
        // Seen on an MSI X870E desktop with Corsair sticks: Manufacturer is the word "Unknown".
        let r = raw(&[("memory_modules", rows(json!([{ "Manufacturer": "Unknown" }, { "Manufacturer": " unknown " }, { "Manufacturer": "Kingston" }])))]);
        let makers: Vec<_> = memory(&Sources(&r)).data.unwrap().modules.into_iter().map(|m| m.manufacturer).collect();
        assert_eq!(makers, [None, None, Some("Kingston".into())]);
    }

    #[test]
    fn a_desktop_has_no_battery_and_a_locked_one_needs_admin() {
        let none = battery(&Sources(&raw(&[("battery_cimv2", rows(json!([])))])));
        assert_eq!(none.check.status, CheckStatus::NotPresent);
        let locked = battery(&Sources(&raw(&[("battery_cimv2", rows(json!([{ "Name": "x" }]))), ("battery_static", err("WMI: 0x80041003"))])));
        assert_eq!(locked.check.status, CheckStatus::NeedsAdmin);
        let missing = battery(&Sources(&raw(&[("battery_cimv2", rows(json!([{ "Name": "x" }]))), ("battery_static", err("Invalid class"))])));
        assert_eq!((missing.check.status, missing.check.detail.as_deref()), (CheckStatus::Failed, Some("Invalid class")));
    }

    #[test]
    fn odd_text_from_the_pc_is_mapped_without_panicking() {
        let r = raw(&[(
            "physical_disks",
            rows(json!([{ "FriendlyName": "  ", "BusType": 99, "DeviceId": "0" }, { "FriendlyName": "Unità <img>", "BusType": 17, "MediaType": 4, "Size": "1000" }])),
        )]);
        let disks = storage(&Sources(&r)).data.unwrap().disks;
        assert_eq!(disks[0].model, None);
        assert_eq!(disks[0].bus, Some(Bus::Other));
        assert_eq!(disks[1].model.as_deref(), Some("Unità <img>"));
        assert_eq!(disks[1].size_bytes, Some(1000));
    }

    #[test]
    fn smart_rows_match_drives_by_product_name() {
        assert!(smart_matches(r"SCSI\Disk&Ven_Samsung&Prod_SSD_860_EVO_1TB\4&1", "Samsung SSD 860 EVO 1TB"));
        assert!(!smart_matches(r"SCSI\Disk&Ven_WDC&Prod_WD10EZEX\4&1", "Samsung SSD 860 EVO 1TB"));
        assert!(!smart_matches("no product here", "Samsung SSD 860 EVO 1TB"));
    }

    #[test]
    fn system_from_the_dev_workstation_dump() {
        #[derive(serde::Deserialize)]
        struct Dump {
            sources: Raw,
        }
        let text = include_str!("../../../spike/dumps/dev-workstation-user-1791321847.json");
        let dump: Dump = serde_json::from_str(text).unwrap();
        let sys = system(&Sources(&dump.sources));
        assert_eq!(sys.check.status, CheckStatus::Ok);
        assert_eq!(
            sys.data.unwrap(),
            System {
                cpu_name: Some("AMD Ryzen 7 9800X3D 8-Core Processor".into()),
                cores: Some(8),
                threads: Some(16),
                board: Some("Gigabyte Technology Co., Ltd. X870E AORUS MASTER X3D".into()),
                bios_version: Some("F8a".into()),
                bios_date: Some("2026-09-23".into()),
                chassis: Some(Chassis::Desktop),
                displays: vec![Display { name: Some("Odyssey G80SH".into()), width_cm: Some(60), height_cm: Some(34) }],
            }
        );
    }

    #[test]
    fn tablets_and_2_in_1s_are_laptops() {
        // Slate (8) only exists in PCSystemTypeEx; PCSystemType says 2 (mobile) or 8 (its "Maximum").
        let chassis = |row: Value| system(&Sources(&raw(&[("computer_system", rows(json!([row])))]))).data.unwrap().chassis;
        assert_eq!(chassis(json!({ "PCSystemType": 1, "PCSystemTypeEx": 8 })), Some(Chassis::Laptop));
        assert_eq!(chassis(json!({ "PCSystemType": 8 })), Some(Chassis::Unknown));
    }

    #[test]
    fn system_edge_cases() {
        let r = raw(&[
            ("computer_system", rows(json!([{ "PCSystemType": 2 }]))),
            ("baseboard", rows(json!([{ "Manufacturer": " ", "Product": "" }]))),
            ("monitor_ids", rows(json!([{ "InstanceName": "A", "UserFriendlyName": [68, 69, 76, 76, 0, 0] }, { "InstanceName": "B" }]))),
        ]);
        let sys = system(&Sources(&r)).data.unwrap();
        assert_eq!((sys.chassis, sys.board), (Some(Chassis::Laptop), None));
        assert_eq!(sys.displays, vec![Display { name: Some("DELL".into()), width_cm: None, height_cm: None }, Display::default()]);
        let none = system(&Sources(&raw(&[("cpu", err("timed out after 4000 ms"))])));
        assert_eq!((none.check.status, none.check.detail.as_deref()), (CheckStatus::Failed, Some("timed out after 4000 ms")));
    }

    fn nvme_drive(n: u64, health: Value) -> Value {
        json!({ "drive": n, "attempts": [{ "access": "none", "ok": true, "health": health }] })
    }

    // The drive's own verdict (critical-warning bits, spare below its threshold) is what the
    // "drive reports a problem" rule reads: it must reach the report.
    #[test]
    fn an_nvme_drive_s_warning_flags_and_spare_reach_the_report() {
        let health = json!({ "critical_warning": 4, "temperature_c": 41, "available_spare_pct": 3, "available_spare_threshold_pct": 10,
            "percentage_used": 12, "power_on_hours": 900, "media_errors": 0 });
        let r = raw(&[
            ("physical_disks", rows(json!([{ "DeviceId": "0", "FriendlyName": "WDC PC SN730", "BusType": 17, "MediaType": 4, "Size": 1_000_000_000_000u64 }]))),
            ("nvme_health", rows(json!([nvme_drive(0, health)]))),
        ]);
        let nvme = storage(&Sources(&r)).data.unwrap().disks[0].nvme.clone().unwrap();
        assert_eq!((nvme.critical_warning, nvme.available_spare_pct, nvme.available_spare_threshold_pct), (Some(4), Some(3), Some(10)));
        assert_eq!((nvme.percentage_used, nvme.media_errors, nvme.temperature_c), (Some(12), Some(0), Some(41)));
    }

    /// A SMART table whose reallocated-sector count (attribute 5) is `realloc`.
    fn smart_bytes(realloc: u8) -> Value {
        let mut b = vec![0u8; 362];
        b[2..14].copy_from_slice(&[5, 0, 0, 100, 100, realloc, 0, 0, 0, 0, 0, 0]);
        json!(b)
    }
    fn sata(id: &str) -> Value {
        json!({ "DeviceId": id, "FriendlyName": "Samsung SSD 860 EVO 1TB", "BusType": 11, "MediaType": 4, "Size": 1_000_000_000_000u64 })
    }
    const TWIN_A: &str = r"SCSI\Disk&Ven_Samsung&Prod_SSD_860_EVO_1TB\4&2a7e3ef3&0&000000";
    const TWIN_B: &str = r"SCSI\Disk&Ven_Samsung&Prod_SSD_860_EVO_1TB\4&2a7e3ef3&0&000100";

    // Two drives of the same model: each gets its own SMART data, matched through Windows' disk
    // number, never the first twin's.
    #[test]
    fn twin_drives_each_get_their_own_smart_data() {
        let r = raw(&[
            ("physical_disks", rows(json!([sata("0"), sata("1")]))),
            ("smart_predict_data", rows(json!([
                { "InstanceName": format!("{TWIN_A}_0"), "VendorSpecific": smart_bytes(0) },
                { "InstanceName": format!("{TWIN_B}_0"), "VendorSpecific": smart_bytes(37) },
            ]))),
            ("disk_drives", rows(json!([
                { "Index": 1, "PNPDeviceID": TWIN_B.to_uppercase() },
                { "Index": 0, "PNPDeviceID": TWIN_A.to_uppercase() },
            ]))),
        ]);
        let disks = storage(&Sources(&r)).data.unwrap().disks;
        let realloc = |i: usize| disks[i].smart.as_ref().and_then(|m| m.reallocated_sectors);
        assert_eq!((realloc(0), realloc(1)), (Some(0), Some(37)));
    }

    // Without the disk numbers, a model name only identifies a drive when no other drive shares it.
    #[test]
    fn without_disk_numbers_twins_get_no_smart_data_rather_than_the_wrong_one() {
        let smart = rows(json!([
            { "InstanceName": format!("{TWIN_A}_0"), "VendorSpecific": smart_bytes(0) },
            { "InstanceName": format!("{TWIN_B}_0"), "VendorSpecific": smart_bytes(37) },
        ]));
        let twins = raw(&[("physical_disks", rows(json!([sata("0"), sata("1")]))), ("smart_predict_data", smart.clone()), ("disk_drives", err("timed out"))]);
        assert!(storage(&Sources(&twins)).data.unwrap().disks.iter().all(|d| d.smart.is_none()));

        let single = raw(&[
            ("physical_disks", rows(json!([sata("0")]))),
            ("smart_predict_data", rows(json!([{ "InstanceName": format!("{TWIN_A}_0"), "VendorSpecific": smart_bytes(2) }]))),
        ]);
        let disks = storage(&Sources(&single)).data.unwrap().disks;
        assert_eq!(disks[0].smart.as_ref().and_then(|m| m.reallocated_sectors), Some(2));
    }

    // SMART attribute 202 is "% life used" on SSDs but an error count on some hard drives, so wear is
    // only read for SSDs.
    #[test]
    fn wear_is_read_only_for_ssds() {
        let mut bytes = vec![0u8; 362];
        bytes[2..14].copy_from_slice(&[202, 0, 0, 100, 100, 150, 0, 0, 0, 0, 0, 0]);
        let disk = |media: u64| json!({ "DeviceId": "0", "FriendlyName": "Some Drive", "BusType": 11, "MediaType": media, "Size": 1 });
        let smart = rows(json!([{ "InstanceName": r"SCSI\Disk&Ven_X&Prod_Some_Drive\4&1&0&000000_0", "VendorSpecific": bytes }]));
        let wear = |media: u64| {
            let r = raw(&[("physical_disks", rows(json!([disk(media)]))), ("smart_predict_data", smart.clone())]);
            storage(&Sources(&r)).data.unwrap().disks[0].smart.as_ref().unwrap().life_used_pct
        };
        assert_eq!(wear(4), Some(150), "SSD");
        assert_eq!(wear(3), None, "HDD");
        assert_eq!(wear(0), None, "unknown media");
    }

    // SMBIOS memory types: older PCs have DDR3, thin laptops soldered LPDDR.
    #[test]
    fn memory_types_older_pcs_and_thin_laptops_use_are_named() {
        let module = |t: u64| json!({ "Capacity": 8_589_934_592u64, "SMBIOSMemoryType": t });
        let r = raw(&[("memory_modules", rows(json!([24, 29, 30, 35, 26, 34, 0].map(module))))]);
        let kinds: Vec<_> = memory(&Sources(&r)).data.unwrap().modules.iter().map(|m| m.kind).collect();
        use MemoryKind::*;
        assert_eq!(kinds, [Ddr3, Lpddr3, Lpddr4, Lpddr5, Ddr4, Ddr5, Other].map(Some));
    }

    #[test]
    fn a_failed_source_fails_its_section_with_the_error() {
        let r = raw(&[("memory_modules", err("timed out after 4000 ms"))]);
        let m = memory(&Sources(&r));
        assert_eq!((m.check.status, m.check.detail.as_deref()), (CheckStatus::Failed, Some("timed out after 4000 ms")));
        let missing = graphics(&Sources(&Raw::new()));
        assert_eq!(missing.check.status, CheckStatus::Failed);
    }
}
