// Converts a Phase 0 spike dump into a Fettle report. Test tooling only, but it's also a
// dry run of the mapping the Phase 3 Rust collectors will do, so it stays readable.

const BUS = { 17: "nvme", 11: "sata", 7: "usb" };
const MEDIA = { 3: "hdd", 4: "ssd" };
const MEMORY_KIND = { 26: "ddr4", 34: "ddr5" };
const FORM_FACTOR = { 8: "dimm", 12: "sodimm" };

const ok = (data) => ({ check: { status: "ok" }, data });
const failed = (detail) => ({ check: { status: "failed", detail } });
const rows = (src) => (src?.ok && Array.isArray(src.data) ? src.data : null);
const accessDenied = (src) => typeof src?.error === "string" && src.error.includes("0x80041003");

/** WMI CIM_DATETIME ("20261006131522.514770+120") → Date. */
function wmiDate(s) {
  const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})\.\d+([+-]\d+)$/.exec(s ?? "");
  if (!m) return null;
  const [, y, mo, d, h, mi, sec, off] = m;
  return new Date(Date.UTC(+y, +mo - 1, +d, +h, +mi, +sec) - Number(off) * 60000);
}
const isoDay = (date) => (date ? date.toISOString().slice(0, 10) : null);

/** A CPU or graphics name without trademark marks: "Intel(R) Core(TM) i7" reads as "Intel Core i7". */
const productName = (s) => s?.replace(/\((R|TM)\)|®|™/gi, " ").split(/\s+/).filter(Boolean).join(" ") || undefined;

/** Win32_QuickFixEngineering.InstalledOn ("9/29/2026") is a calendar day with no time zone. */
function hotfixDay(s) {
  const m = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(s ?? "");
  return m ? `${m[3]}-${m[1].padStart(2, "0")}-${m[2].padStart(2, "0")}` : null;
}

/** The SMART attribute table: 12-byte entries from offset 2 (id, flags×2, value, worst, raw×6). */
function smartAttributes(bytes) {
  const attrs = new Map();
  for (let i = 2; i + 12 <= bytes.length; i += 12) {
    const id = bytes[i];
    if (!id) continue;
    const raw = bytes.slice(i + 5, i + 11).reduceRight((acc, b) => acc * 256 + b, 0);
    attrs.set(id, { value: bytes[i + 3], raw });
  }
  return attrs;
}

function smartSummary(attrs) {
  const raw = (id) => attrs.get(id)?.raw;
  // Vendor wear attributes: Crucial 202 = % life used; Samsung 177 = normalized life left.
  const lifeUsed = attrs.has(202) ? raw(202) : attrs.has(177) ? 100 - attrs.get(177).value : undefined;
  return {
    reallocated_sectors: raw(5),
    pending_sectors: raw(197),
    offline_uncorrectable: raw(198),
    power_on_hours: raw(9),
    temperature_c: attrs.has(194) ? raw(194) % 256 : undefined, // low byte = current °C
    life_used_pct: lifeUsed,
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

function storage(s) {
  const disks = rows(s.physical_disks);
  if (!disks) return failed(s.physical_disks?.error);
  const nvmeByDrive = new Map((s.nvme_health?.data ?? []).map((d) => [String(d.drive), d.attempts.find((a) => a.ok)?.health]));
  const smart = rows(s.smart_predict_data) ?? [];
  return ok({
    disks: disks.map((d) => {
      const model = d.FriendlyName;
      const nvme = nvmeByDrive.get(d.DeviceId);
      const smartRow = smart.find((r) => smartMatches(r.InstanceName, model));
      return {
        model,
        bus: BUS[d.BusType] ?? "other",
        media: MEDIA[d.MediaType] ?? "unknown",
        size_bytes: d.Size,
        nvme: nvme ? { percentage_used: nvme.percentage_used, temperature_c: nvme.temperature_c, power_on_hours: nvme.power_on_hours, media_errors: nvme.media_errors } : undefined,
        smart: smartRow ? smartSummary(smartAttributes(smartRow.VendorSpecific)) : undefined,
      };
    }),
    volumes: (rows(s.logical_disks) ?? []).map((v) => ({ letter: v.DeviceID, size_bytes: v.Size, free_bytes: v.FreeSpace })),
  });
}

function battery(s) {
  const cim = rows(s.battery_cimv2);
  if (cim && cim.length === 0) return { check: { status: "not_present" } };
  const design = rows(s.battery_static)?.[0]?.DesignedCapacity;
  const full = rows(s.battery_full_capacity)?.[0]?.FullChargedCapacity;
  const cycles = rows(s.battery_cycle_count)?.[0]?.CycleCount;
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
    tpm: tpm ? { present: true, spec_version: tpm.SpecVersion } : undefined,
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
