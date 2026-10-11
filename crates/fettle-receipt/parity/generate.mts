// The parity corpus (tests/parity/cases): the real spike dumps plus seeded reports covering every enum
// value, nulls and zeros, each threshold and its neighbours, long and markdown-special names,
// impossible dates. Same seed, same reports.
//
// `npm run parity:goldens` runs this: it writes each case's report, keeping the expected receipt of
// any report that didn't change, then has the Rust receipt fill in what's new or changed
// (FETTLE_REGEN_GOLDENS=1, tests/parity.rs). Since 2026-10-10 the Rust receipt is the only one.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseReport, type Report } from "../../../packages/report-schema/src/index.ts";
import { convertDump } from "./from-spike.mjs";

/** mulberry32: small, fast, and identical on every machine. */
function prng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const GIB = 2 ** 30;
const DAY_MS = 86400000;
const iso = (ms: number) => new Date(ms).toISOString().slice(0, 10);

export function generateReports(seed: number, count: number): Report[] {
  const rnd = prng(seed);
  const pick = <T,>(xs: readonly T[]): T => xs[Math.floor(rnd() * xs.length)]!;
  const chance = (p: number) => rnd() < p;
  const int = (lo: number, hi: number) => lo + Math.floor(rnd() * (hi - lo + 1));
  /** A field that is sometimes missing: undefined drops it from the JSON. */
  const opt = <T,>(v: T, p = 0.85): T | undefined => (chance(p) ? v : undefined);

  const section = <T,>(data: () => T, statuses = ["ok", "ok", "ok", "ok", "ok", "ok", "ok", "ok", "needs_admin", "failed", "not_present"]) => {
    const status = pick(statuses);
    if (status !== "ok") return { check: { status, ...(chance(0.3) ? { detail: "Access is denied." } : {}) } };
    return { check: { status }, ...(chance(0.97) ? { data: data() } : {}) };
  };

  const NAMES = [
    "Samsung SSD 990 PRO 2TB", "CT1000MX500SSD1", "WDC PC SN730 SDBPNTY-1T00-1101", "WD_BLACK SN850X *fast* `x`|y [z]",
    "ST2000DM008-2FR102", "Kingston SA400S37480G", "SanDisk Ultra Fit", "🚀 Speedy 💾 Drive",
    "A very long model name that goes on and on beyond forty-two characters easily", "> quoted ~strike~ ^up^ \\back",
  ];
  const PARTS = [
    "F5-6000J2836G16G", "F4-3200C16D-16GVKB", "CMK32GX5M2B6000C36", "CMW32GX4M2E3200C16", "KF560C36BBEK2-32",
    "HX432C16FB3/16", "CT16G56C46U5", "CP16G60C36U5B", "BL16G32C16U4B", "FF3D532G6000HC38ADC01", "M378A1K43CB2-CTD", "  ",
  ];
  const STARTUP = [
    "apple", "Apple", "Banana", "éclair", "Eclair", "Zoom", "Everything", "Ölmühle", "zebra", "Microsoft Edge",
    "Fosi Audio USB Audio Device Control Panel Autostart.lnk", "OneDrive", "Steam", "Discord", "_underscore", "123 Numbers",
  ];
  const VERSIONS = ["21H2", "22H2", "23H2", "24H2", "25H2", "26H1", "26H2", "21H1", "20H2", "2004", "1909", "1903", "1809", "1803", "1709", "1703", "1607", "1511", "1507", "99H9"];
  const EDITIONS = ["Core", "Professional", "ProfessionalWorkstation", "CloudEdition", "Enterprise", "Education", "ServerRdsh", "EnterpriseS", "IoTEnterpriseS", "Unknown"];

  const reports: Report[] = [];
  let tries = 0;
  while (reports.length < count) {
    tries++;
    // Collected between 2023 and 2030; some on 29 Feb 2028 (the driver cutoff's rollover day).
    const at = chance(0.05) ? Date.UTC(2028, 1, 29, int(0, 23), int(0, 59)) : Date.UTC(2023, 0, 1) + int(0, 8 * 365) * DAY_MS + int(0, DAY_MS - 1);
    const offset = pick(["Z", "Z", "+02:00", "-05:00", "+00:00"]);
    const shifted = offset === "Z" || offset === "+00:00" ? at : at + (offset.startsWith("+") ? 1 : -1) * Number(offset.slice(1, 3)) * 3600000;
    // The local wall time for that offset, sometimes with fractional seconds.
    // A few times the schema allows but no clock writes: JavaScript rolls them over (31 Feb is 3 March).
    const odd = chance(0.03) ? pick(["2026-02-31T10:00:00Z", "2027-04-31T23:30:00+02:00", "2026-10-07T24:00:00Z", "2028-02-30T12:00:00.500-05:00"]) : null;
    const collected_at = odd ?? `${new Date(shifted).toISOString().slice(0, 19)}${chance(0.2) ? ".123" : ""}${offset}`;
    const day = (deltaDays: number) => iso(at + deltaDays * DAY_MS);
    const cutoff = (() => {
      const d = new Date(at);
      d.setUTCMonth(d.getUTCMonth() - 12);
      return d.getTime();
    })();

    const disk = () => {
      const bus = pick(["nvme", "nvme", "sata", "sata", "usb", "other", undefined]);
      const nvme = bus === "nvme" && chance(0.85)
        ? {
            percentage_used: opt(pick([0, 1, 5, 79, 80, 81, 99, 100, 101, 255])),
            temperature_c: opt(pick([30, 46, 69, 70, 71, 85])),
            power_on_hours: opt(pick([0, 441, 3633, 123456])),
            media_errors: opt(pick([0, 0, 0, 1, 2, 1234567])),
            critical_warning: opt(pick([0, 0, 0, 0, 1, 4])),
            available_spare_pct: opt(pick([100, 100, 11, 10, 9]), 0.6),
            available_spare_threshold_pct: opt(10, 0.6),
          }
        : undefined;
      const smart = (bus === "sata" || bus === "other") && chance(0.6)
        ? {
            reallocated_sectors: opt(pick([0, 0, 1, 8, 1000])),
            pending_sectors: opt(pick([0, 0, 1, 3])),
            offline_uncorrectable: opt(pick([0, 2])),
            power_on_hours: opt(pick([12, 9999, 45678])),
            temperature_c: opt(pick([28, 70, 71])),
            life_used_pct: opt(pick([0, 5, 99, 100, 120]), 0.5),
          }
        : undefined;
      return {
        model: opt(pick(NAMES), 0.9),
        bus,
        media: opt(pick(["ssd", "hdd", "unknown"])),
        size_bytes: opt(pick([0, 9_999_999_999, 256_060_514_304, 500_107_862_016, 1_000_204_886_016, 2_000_398_934_016, 8_001_563_222_016])),
        nvme,
        smart,
      };
    };
    const volume = () => {
      const size = pick([0, 499_000_000_000, 1_000_000_000_000, 64_000_000]);
      const ratio = pick([0, 0.05, 0.0999, 0.1, 0.11, 0.5, 1]);
      return { letter: opt(pick(["C:", "D:", "E:"])), size_bytes: opt(size), free_bytes: opt(Math.floor(size * ratio)) };
    };
    const stick = () => ({
      capacity_bytes: opt(pick([8 * GIB, 16 * GIB, 32 * GIB, 512 * 2 ** 20, 0])),
      speed_mts: opt(pick([0, 3200, 4800, 6000])),
      configured_speed_mts: opt(pick([0, 2133, 4800, 6000])),
      part_number: opt(pick(PARTS)),
      kind: opt(pick(["ddr4", "ddr5", "other"])),
      manufacturer: opt(pick(["G.Skill", "Corsair", "Kingston", "Crucial", "Samsung", "  "])),
      bank_label: opt(pick(["P0 CHANNEL A", "P0 CHANNEL B", "BANK 0", "BANK 1", "P0 CHANNEL C", "P0 CHANNEL D"])),
      device_locator: opt(pick(["DIMM 1", "DIMM 2", "ChannelA-DIMM0", "ChannelB-DIMM1", "Controller0-ChannelA", "DIMM_A1", "  "])),
      configured_voltage_mv: opt(pick([1100, 1350])),
      form_factor: opt(pick(["dimm", "sodimm", "other"])),
    });
    const startupCount = pick([0, 1, 2, 3, 15, 16, 17, 6]);
    const adapter = () => ({
      name: opt(pick(["AMD Radeon RX 6900 XT", "NVIDIA GeForce RTX 4090", "Intel(R) UHD Graphics 620", "Microsoft Basic Render Driver"])),
      vram_bytes: opt(pick([0, 512 * 2 ** 20, 8 * GIB, 16 * GIB, 24 * GIB])),
      driver_version: opt(pick(["32.0.21045.11001", "31.0.15.3623"])),
      driver_date: opt(pick([iso(cutoff - DAY_MS), iso(cutoff), iso(cutoff + DAY_MS), day(-30), "2020-01-01", "2027-02-28", "2027-03-01", "2027-03-02", "2026-02-31"])),
      temperature_c: opt(pick([40, 55]), 0.3),
      software: opt(pick([false, false, false, true])),
    });

    const candidate = {
      schema_version: 1,
      app_version: "0.1.0",
      collected_at,
      elevated: chance(0.4),
      facts: {
        system: opt(section(() => ({
          cpu_name: opt(pick(["AMD Ryzen 7 9800X3D 8-Core Processor", "Intel Core i5-8250U CPU @ 1.60GHz", "CPU *with* `markdown`", "Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz", "AMD Ryzen 5 1600 Six-Core Processor", "11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz", "AMD FX(tm)-8350 Eight-Core Processor"])),
          cores: opt(8), threads: opt(16),
          board: opt(pick(["Gigabyte Technology Co., Ltd. X870E AORUS MASTER X3D", "LENOVO 20KN001QIX", "MSI MAG X870E TOMAHAWK WIFI (MS-7E59)"])),
          bios_version: opt(pick(["F8a", "N23ET86W (1.61 )", ""])),
          bios_date: opt(pick(["2026-09-23", "2019-01-07", "2026-02-31"])),
          chassis: opt(pick(["desktop", "laptop", "unknown"])),
          displays: [],
        }))),
        storage: opt(section(() => ({ disks: Array.from({ length: chance(0.05) ? int(10, 16) : int(0, 4) }, disk), volumes: Array.from({ length: int(0, 3) }, volume) })), 0.95),
        battery: opt(section(() => {
          const design = pick([0, 41000, 50000, 57000]);
          const ratio = pick([0, 0.59, 0.6, 0.795, 0.79, 0.8, 0.95, 1.05]);
          return { design_capacity_mwh: opt(design), full_charge_capacity_mwh: opt(Math.floor(design * ratio)), cycle_count: opt(pick([0, 312, 1500]), 0.6) };
        }), 0.7),
        memory: opt(section(() => ({ modules: Array.from({ length: int(0, 4) }, stick), slots_total: opt(pick([0, 1, 2, 4])) })), 0.95),
        windows: opt(section(() => ({
          // A fifth are Windows 10 22H2, the only release with extended security updates (ESU).
          product: opt(pick(["Windows 11 Pro", "Windows 10 Home", "Windows 11 Pro for Workstations", "Windows 10 Enterprise LTSC 2021"])),
          edition_id: opt(pick(EDITIONS), 0.95),
          build: opt(pick([17763, 19041, 19044, 19045, 22000, 22621, 22631, 26100, 26200, 26300]), 0.95),
          ubr: opt(pick([1, 6456, 6466, 6575, 9550])),
          display_version: opt(pick(VERSIONS), 0.95),
          installation_type: opt("Client"),
          uptime_seconds: opt(pick([0, 3599, 3600, 7200, 86399, 86400, 172800, 13 * 86400 + 5, 14 * 86400, 40 * 86400])),
          last_update_installed: opt(pick(["2025-10-14", "2025-10-15", "2025-10-16", "2026-01-13", "2023-05-01", "2027-10-12", "2027-10-14", "2028-10-12", "2026-02-31", day(-10)])),
          secure_boot: opt(pick([true, false])),
          tpm: opt(pick([{ present: false }, { present: true, spec_version: "2.0, 0, 1.59" }, { present: true, spec_version: "1.2, 2, 3" }, { present: true }, {}]), 0.8),
          esu: opt(pick(["enrolled", "not_enrolled", "unknown"]), 0.5),
          // A fifth are Windows 10 22H2, the only release with extended security updates (ESU).
          ...(chance(0.2) ? { build: 19045, display_version: "22H2", edition_id: pick(["Core", "Professional", "Enterprise", "Education"]) } : {}),
        })), 0.95),
        startup: opt(section(() => ({
          items: Array.from({ length: startupCount }, () => ({
            name: opt(pick(STARTUP), 0.95),
            location: opt(pick(["run_key", "run_once", "folder"])),
            enabled: opt(pick([true, true, false])),
          })),
        })), 0.95),
        graphics: opt(section(() => ({ adapters: Array.from({ length: int(0, 3) }, adapter) })), 0.95),
      },
    };
    const parsed = parseReport(JSON.parse(JSON.stringify(candidate)));
    if (parsed.ok) reports.push(parsed.report);
    if (tries > count * 1.01 + 5) throw new Error(`too many invalid reports: ${tries - reports.length} of ${tries}`);
  }
  return reports;
}

const here = (rel: string) => fileURLToPath(new URL(rel, import.meta.url));
export const CASES_DIR = here("../tests/parity/cases");
const DUMPS = here("../../../spike/dumps");
const SEED = 20261009;
const GENERATED = 400;

/** Every case's name and report: the tracked spike dumps first, then the seeded ones. */
export function corpus(): [string, Report][] {
  const dumps = readdirSync(DUMPS)
    .filter((f) => f.endsWith(".json"))
    .sort()
    .map((f): [string, Report] => {
      const parsed = parseReport(convertDump(JSON.parse(readFileSync(join(DUMPS, f), "utf8"))));
      if (!parsed.ok) throw new Error(`${f}: ${parsed.error}`);
      return [`dump-${f.replace(/\.json$/, "")}`, parsed.report];
    });
  const generated = generateReports(SEED, GENERATED).map((r, i): [string, Report] => [`gen-${String(i).padStart(3, "0")}`, r]);
  return [...dumps, ...generated];
}

/** Writes the reports; a case whose report is unchanged keeps its file (and its expected receipt) as is. */
export function writeReports(): { written: number; kept: number } {
  mkdirSync(CASES_DIR, { recursive: true });
  let written = 0;
  let kept = 0;
  for (const [name, report] of corpus()) {
    const file = join(CASES_DIR, `${name}.json`);
    const old = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : null;
    // Through JSON, as the file stores it: undefined fields drop out.
    const fresh = JSON.parse(JSON.stringify(report));
    if (old && JSON.stringify(old.report) === JSON.stringify(fresh)) {
      kept++;
      continue;
    }
    writeFileSync(file, `${JSON.stringify({ name, report: fresh })}\n`);
    written++;
  }
  return { written, kept };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { written, kept } = writeReports();
  console.log(`parity: ${written} report(s) written, ${kept} unchanged`);
  // rustup's bin folder isn't always on PATH for Node.
  const env = { ...process.env, FETTLE_REGEN_GOLDENS: "1", PATH: `${join(homedir(), ".cargo", "bin")}${delimiter}${process.env.PATH}` };
  const args = ["test", "-q", "-p", "fettle-receipt", "--test", "parity", "regenerate_goldens", "--", "--nocapture"];
  const run = spawnSync("cargo", args, { cwd: here("../../../"), env, stdio: "inherit" });
  process.exit(run.status ?? 1);
}
