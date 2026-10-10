// Reports for the test page: the real dev-workstation spike runs, plus a made-up tired laptop
// that trips every verdict level and the awkward inputs (empty values, odd sizes, partial scans).
import { parseReport, type Report } from "@fettle/report-schema";
import { convertDump } from "../../../crates/fettle-receipt/parity/from-spike.mjs";
import adminSmart from "../../../spike/dumps/dev-workstation-admin-1791322268.json" with { type: "json" };
import user from "../../../spike/dumps/dev-workstation-user-1791321847.json" with { type: "json" };

function valid(input: unknown): Report {
  const parsed = parseReport(input);
  if (!parsed.ok) throw new Error(parsed.error);
  return parsed.report;
}

const GB = 1e9;
const startupApps = [
  "OneDrive", "Microsoft Teams", "Spotify", "Steam", "Discord", "Adobe Creative Cloud", "Epic Games Launcher",
  "Dropbox", "Zoom", "Skype", "HP Support Assistant", "McAfee WebAdvisor", "Realtek Audio Console", "Google Drive",
  "Opera GX", "WhatsApp", "EA app", "Logitech Options", "MicrosoftEdgeAutoLaunch_D381F9995CD4E4E9D3547570FCA4BB36",
];

const tiredLaptop: Report = valid({
  schema_version: 1,
  app_version: "0.1.0",
  collected_at: "2026-10-07T09:12:00+02:00",
  elevated: false,
  facts: {
    system: {
      check: { status: "ok" },
      data: { cpu_name: "Intel Core i5-6200U", board: "HP 8079", bios_version: "N75 Ver. 01.42", bios_date: "2019-03-11", chassis: "laptop", displays: [] },
    },
    storage: {
      check: { status: "ok" },
      data: {
        disks: [
          {
            model: "WDC PC SN730 SDBPNTY-1T00-1101", bus: "nvme", media: "ssd", size_bytes: 1_024_209_543_168,
            nvme: { percentage_used: 87, temperature_c: 52, power_on_hours: 19_840, media_errors: 4, available_spare_pct: 62, available_spare_threshold_pct: 10 },
          },
        ],
        volumes: [{ letter: "C:", size_bytes: 1_023_000_000_000, free_bytes: 61 * GB }],
      },
    },
    battery: { check: { status: "ok" }, data: { design_capacity_mwh: 45_000, full_charge_capacity_mwh: 26_100, cycle_count: 611 } },
    memory: {
      check: { status: "ok" },
      data: {
        modules: [{
          capacity_bytes: 8 * 2 ** 30, kind: "ddr4", form_factor: "sodimm", speed_mts: 2400, configured_speed_mts: 2133,
          manufacturer: "SK Hynix", part_number: "HMA81GS6AFR8N-UH", bank_label: "BANK 0", device_locator: "ChannelA-DIMM0", configured_voltage_mv: 1200,
        }],
        slots_total: 2,
      },
    },
    windows: {
      check: { status: "ok" },
      data: {
        product: "Windows 10 Pro", edition_id: "Professional", build: 19045, ubr: 6332, display_version: "22H2",
        uptime_seconds: 23 * 86400 + 4000, last_update_installed: "2026-09-10", secure_boot: false, esu: "unknown",
      },
    },
    startup: {
      check: { status: "ok" },
      data: {
        items: [
          ...startupApps.map((name) => ({ name, location: "run_key", enabled: true })),
          { name: "Cortana", location: "run_key", enabled: false },
          { name: "Java Update Scheduler", location: "run_key", enabled: false },
        ],
      },
    },
    graphics: {
      check: { status: "ok" },
      data: {
        adapters: [
          { name: "Intel HD Graphics 520", vram_bytes: 128 * 2 ** 20, driver_version: "27.20.100.8681", driver_date: "2023-02-14" },
          { name: "<img src=x onerror=alert(1)>", driver_version: "1.0", driver_date: "2026-01-01" },
        ],
      },
    },
  },
});

export const FIXTURES: { id: string; label: string; report: Report }[] = [
  { id: "tired", label: "Tired laptop (made up)", report: tiredLaptop },
  { id: "admin", label: "Dev workstation · Deep check", report: valid(convertDump(adminSmart)) },
  { id: "user", label: "Dev workstation · Quick check", report: valid(convertDump(user)) },
];
