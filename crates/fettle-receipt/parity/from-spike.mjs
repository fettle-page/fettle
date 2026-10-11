// Converts a Phase 0 spike dump into a Fettle report. Test tooling only, but it's also a
// dry run of the mapping the Phase 3 Rust collectors will do, so it stays readable.

const BUS = { 17: "nvme", 11: "sata", 7: "usb" };
const MEDIA = { 3: "hdd", 4: "ssd" };
const MEMORY_KIND = { 24: "ddr3", 26: "ddr4", 34: "ddr5", 29: "lpddr3", 30: "lpddr4", 35: "lpddr5" };
const FORM_FACTOR = { 8: "dimm", 12: "sodimm" };

const ok = (data) => ({ check: { status: "ok" }, data });
const failed = (detail) => ({ check: { status: "failed", detail } });
const rows = (src) => (src?.ok && Array.isArray(src.data) ? src.data : null);
const accessDenied = (src) => typeof src?.error === "string" && src.error.includes("0x80041003");

/** A UTC date for any year: Date.UTC would read years 0 to 99 as 1900 to 1999. */
function utc(y, mo, d, h = 0, mi = 0, sec = 0) {
  const date = new Date(0);
  date.setUTCFullYear(y, mo - 1, d);
  date.setUTCHours(h, mi, sec, 0);
  return date;
}

/** Whether year-month-day names a real calendar day (no month 0 or 13, no 30 February). */
function realDay(y, m, d) {
  const date = utc(y, m, d);
  return m >= 1 && m <= 12 && d >= 1 && date.getUTCFullYear() === y && date.getUTCMonth() === m - 1 && date.getUTCDate() === d;
}

/** WMI CIM_DATETIME ("20261006131522.514770+120") → Date. */
function wmiDate(s) {
  const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})\.\d+([+-]\d+)$/.exec(s ?? "");
  if (!m) return null;
  const [, y, mo, d, h, mi, sec, off] = m;
  if (!realDay(+y, +mo, +d) || +h > 23 || +mi > 59 || +sec > 60) return null;
  return new Date(utc(+y, +mo, +d, +h, +mi, +sec).getTime() - Number(off) * 60000);
}
const isoDay = (date) => (date ? date.toISOString().slice(0, 10) : null);

/** A CPU or graphics name without trademark marks: "Intel(R) Core(TM) i7" reads as "Intel Core i7". */
const productName = (s) => s?.replace(/\((R|TM)\)|®|™/gi, " ").split(/\s+/).filter(Boolean).join(" ") || undefined;

/** Win32_QuickFixEngineering.InstalledOn ("9/29/2026") is a calendar day with no time zone, month
 * first. A date that isn't a real day that way (a day-first "29/9/2026") is left out, not swapped. */
function hotfixDay(s) {
  const m = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(s ?? "");
  return m && realDay(+m[3], +m[1], +m[2]) ? `${m[3]}-${m[1].padStart(2, "0")}-${m[2].padStart(2, "0")}` : null;
}

/** The SMART attribute table: 12-byte entries from offset 2 (id, flags×2, value, worst, raw×6). */
function smartAttributes(bytes) {
  const attrs = new Map();
  // 30 entries (bytes 2 to 361): what follows them in the 512-byte block isn't attributes.
  for (let i = 2; i + 12 <= Math.min(bytes.length, 2 + 30 * 12); i += 12) {
    const id = bytes[i];
    if (!id) continue;
    const raw = bytes.slice(i + 5, i + 11).reduceRight((acc, b) => acc * 256 + b, 0);
    attrs.set(id, { value: bytes[i + 3], raw });
  }
  return attrs;
}

/** Life used, the way the drive's maker defines it (map.rs life_used); an unknown maker gets none. */
function lifeUsed(attrs, model) {
  const left = (id) => (attrs.has(id) ? Math.max(0, 100 - attrs.get(id).value) : undefined);
  const m = (model ?? "").toUpperCase();
  if (m.includes("SAMSUNG")) return left(177);
  if (m.startsWith("CT") || m.includes("CRUCIAL") || m.includes("MICRON") || m.includes("MTFD")) return attrs.has(202) ? Math.min(attrs.get(202).raw, 255) : undefined;
  if (m.startsWith("INTEL")) return left(233);
  return left(231);
}

function smartSummary(attrs, model) {
  const raw = (id) => attrs.get(id)?.raw;
  return {
    reallocated_sectors: raw(5),
    pending_sectors: raw(197),
    offline_uncorrectable: raw(198),
    // The low 4 bytes: some controllers (SandForce) pack other counters into the upper ones.
    power_on_hours: attrs.has(9) ? raw(9) % 2 ** 32 : undefined,
    // Low byte = current °C. Some drives (Samsung SATA SSDs) report it only as 190, airflow temperature.
    temperature_c: attrs.has(194) ? raw(194) % 256 : attrs.has(190) ? raw(190) % 256 : undefined,
    life_used_pct: lifeUsed(attrs, model),
  };
}

/**
 * SMART rows name the drive as "SCSI\Disk&Ven_Samsung&Prod_SSD_860_EVO_1TB\…" while Windows calls it
 * "Samsung SSD 860 EVO 1TB", so match on the product part, ignoring punctuation.
 */
function smartMatches(instanceName, model) {
  const norm = (x) => x.toUpperCase().replace(/[^A-Z0-9]/g, "");
  const product = /Prod_([^\\]+)/i.exec(instanceName ?? "")?.[1];
  return !!product && norm(model).includes(norm(product));
}

/**
 * The SMART row for the disk Windows numbers `deviceId` (map.rs smart_for): a row's InstanceName is the
 * drive's PNPDeviceID plus "_<n>", and Win32_DiskDrive numbers each PNPDeviceID. Without those numbers,
 * the model decides only when no other disk shares it, so a twin never gets the other drive's data.
 */
function smartFor(s, smart, deviceId, model, models) {
  const drives = rows(s.disk_drives);
  if (drives) {
    const pnp = drives.find((d) => String(d.Index) === deviceId)?.PNPDeviceID?.toUpperCase();
    if (!pnp) return undefined;
    return smart.find((r) => {
      const name = (r.InstanceName ?? "").toUpperCase();
      return name.startsWith(pnp) && /^_\d+$/.test(name.slice(pnp.length));
    });
  }
  if (!model || models.filter((m) => m === model).length !== 1) return undefined;
  const matching = smart.filter((r) => smartMatches(r.InstanceName, model));
  return matching.length === 1 ? matching[0] : undefined;
}

function storage(s) {
  const disks = rows(s.physical_disks);
  if (!disks) return failed(s.physical_disks?.error);
  const nvmeByDrive = new Map((s.nvme_health?.data ?? []).map((d) => [String(d.drive), d.attempts.find((a) => a.ok)?.health]));
  const smart = rows(s.smart_predict_data) ?? [];
  return ok({
    disks: disks.map((d) => {
      // Trimmed, empty as missing, like map.rs text_of.
      const model = d.FriendlyName?.trim() || undefined;
      const media = MEDIA[d.MediaType] ?? "unknown";
      const nvme = nvmeByDrive.get(d.DeviceId);
      const smartRow = smartFor(s, smart, String(d.DeviceId), model, disks.map((x) => x.FriendlyName?.trim() || undefined));
      return {
        model,
        bus: BUS[d.BusType] ?? "other",
        media,
        size_bytes: d.Size,
        nvme: nvme
          ? {
              percentage_used: nvme.percentage_used,
              temperature_c: nvme.temperature_c,
              power_on_hours: nvme.power_on_hours,
              media_errors: nvme.media_errors,
              critical_warning: nvme.critical_warning,
              available_spare_pct: nvme.available_spare_pct,
              available_spare_threshold_pct: nvme.available_spare_threshold_pct,
            }
          : undefined,
        // Wear attributes mean "% life used" only on SSDs (202 counts errors on some hard drives).
        smart: smartRow ? { ...smartSummary(smartAttributes(smartRow.VendorSpecific), model), ...(media === "ssd" ? {} : { life_used_pct: undefined }) } : undefined,
      };
    }),
    volumes: (rows(s.logical_disks) ?? []).map((v) => ({ letter: v.DeviceID, size_bytes: v.Size, free_bytes: v.FreeSpace })),
  });
}

function battery(s) {
  const cim = rows(s.battery_cimv2);
  if (cim && cim.length === 0) return { check: { status: "not_present" } };
  // With two batteries, the rows are matched on the battery's instance name.
  // The first name is judged, so it's the same battery whichever order Windows lists them in.
  const byName = (a, b) => (String(a.InstanceName ?? "") < String(b.InstanceName ?? "") ? -1 : String(a.InstanceName ?? "") > String(b.InstanceName ?? "") ? 1 : 0);
  const first = [...(rows(s.battery_static) ?? [])].sort(byName)[0];
  const row = (src) => (first?.InstanceName != null ? rows(src)?.find((r) => r.InstanceName === first.InstanceName) : rows(src)?.[0]);
  const design = first?.DesignedCapacity;
  const full = row(s.battery_full_capacity)?.FullChargedCapacity;
  const cycles = row(s.battery_cycle_count)?.CycleCount;
  if (design == null && full == null) return accessDenied(s.battery_static) ? { check: { status: "needs_admin" } } : failed(s.battery_static?.error);
  return ok({ design_capacity_mwh: design, full_charge_capacity_mwh: full, cycle_count: cycles });
}

function memory(s) {
  const modules = rows(s.memory_modules);
  if (!modules) return failed(s.memory_modules?.error);
  return ok({
    slots_total: rows(s.memory_array)?.[0]?.MemoryDevices,
    modules: modules.map((m) => ({
      capacity_bytes: m.Capacity,
      speed_mts: m.Speed,
      configured_speed_mts: m.ConfiguredClockSpeed,
      part_number: m.PartNumber?.trim(),
      kind: MEMORY_KIND[m.SMBIOSMemoryType] ?? "other",
      // Some firmware writes the word "Unknown" instead of leaving it empty.
      manufacturer: /^unknown$/i.test(m.Manufacturer?.trim() ?? "") ? undefined : m.Manufacturer?.trim() || undefined,
      bank_label: m.BankLabel?.trim() || undefined,
      device_locator: m.DeviceLocator?.trim() || undefined,
      configured_voltage_mv: m.ConfiguredVoltage || undefined,
      // 0 is "Unknown" (some laptops report it for their SO-DIMMs): not known, not "other".
      form_factor: !m.FormFactor ? undefined : FORM_FACTOR[m.FormFactor] ?? "other",
    })),
  });
}

function windows(s, collectedAt) {
  const reg = s.windows_registry?.data;
  const os = rows(s.os)?.[0];
  if (!reg || !os) return failed(s.windows_registry?.error ?? s.os?.error);
  const boot = wmiDate(os.LastBootUpTime);
  const tpm = rows(s.tpm)?.[0];
  const updates = (rows(s.hotfixes) ?? []).map((h) => hotfixDay(h.InstalledOn)).filter(Boolean).sort();
  return ok({
    // The registry's ProductName says "Windows 10" on Windows 11; the OS caption doesn't.
    product: os.Caption?.replace(/^Microsoft /, ""),
    edition_id: reg.EditionID,
    build: Number(reg.CurrentBuild),
    ubr: reg.UBR,
    display_version: reg.DisplayVersion ?? reg.ReleaseId,
    installation_type: reg.InstallationType,
    uptime_seconds: boot ? Math.max(0, Math.round((collectedAt - boot) / 1000)) : undefined,
    last_update_installed: updates.at(-1),
    secure_boot: s.secure_boot?.ok ? s.secure_boot.data.uefi_secure_boot_enabled === 1 : undefined,
    // Elevated, Windows answers with no rows when the PC has no TPM; a failed query proves nothing.
    tpm: rows(s.tpm) ? (tpm ? { present: true, spec_version: tpm.SpecVersion } : { present: false }) : undefined,
  });
}

function startup(s) {
  const reg = rows(s.startup_registry);
  if (!reg) return failed(s.startup_registry?.error);
  const approved = rows(s.startup_approved) ?? [];
  const enabled = (hive, key, name) => approved.find((a) => a.hive === hive && a.key === key && a.name === name)?.enabled;
  const items = reg
    .filter((r) => !r.error)
    .map((r) => {
      const once = /RunOnce$/.test(r.key);
      const approvedKey = /WOW6432Node/i.test(r.key) ? "Run32" : "Run";
      return { name: r.name, location: once ? "run_once" : "run_key", enabled: once ? undefined : enabled(r.hive, approvedKey, r.name) };
    });
  for (const f of rows(s.startup_folders) ?? []) {
    if (f.error) continue;
    const hive = f.folder === "user" ? "HKCU" : "HKLM";
    items.push({ name: f.file, location: "folder", enabled: enabled(hive, "StartupFolder", f.file) });
  }
  return ok({ items });
}

function graphics(s) {
  const adapters = rows(s.dxgi_adapters);
  if (!adapters) return failed(s.dxgi_adapters?.error);
  const controllers = rows(s.video_controllers) ?? [];
  return ok({
    adapters: adapters.map((a) => {
      const wmi = controllers.find((c) => c.Name === a.description);
      return {
        name: productName(a.description),
        vram_bytes: a.dedicated_video_memory,
        driver_version: wmi?.DriverVersion,
        driver_date: isoDay(wmiDate(wmi?.DriverDate)),
        software: a.software,
      };
    }),
  });
}

export function convertDump(dump) {
  const s = dump.sources;
  const collectedAt = new Date(dump.collected_at_unix * 1000);
  return {
    schema_version: 1,
    app_version: `spike-${dump.spike_version}`,
    collected_at: collectedAt.toISOString(),
    elevated: dump.elevated,
    facts: {
      storage: storage(s),
      battery: battery(s),
      memory: memory(s),
      windows: windows(s, collectedAt),
      startup: startup(s),
      graphics: graphics(s),
    },
  };
}
