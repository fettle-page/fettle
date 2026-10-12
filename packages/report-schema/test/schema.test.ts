import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { generate, MAX_ITEMS } from "../scripts/generate.mjs";
import { parseReport, SCHEMA_VERSION } from "../src/index.ts";

const base = { schema_version: 1, app_version: "0.0.0", collected_at: "2026-10-06T23:00:00Z", elevated: false };

describe("parseReport", () => {
  it("parses the partial report", () => {
    const r = parseReport({ ...base, facts: { battery: { check: { status: "not_present" } } } });
    expect(r.ok).toBe(true);
  });

  it("rejects a missing schema_version", () => {
    const { schema_version, ...rest } = base;
    expect(parseReport({ ...rest, facts: {} }).ok).toBe(false);
  });

  it("strips unknown fields", () => {
    const r = parseReport({ ...base, facts: {}, evil: "<script>" });
    expect(r.ok).toBe(true);
    if (r.ok) expect("evil" in r.report).toBe(false);
  });

  it("rejects a wrong check status", () => {
    expect(parseReport({ ...base, facts: { battery: { check: { status: "maybe" } } } }).ok).toBe(false);
  });

  it("takes why a check failed as one of a few codes, and nothing else there", () => {
    const failed = (check: object) => parseReport({ ...base, facts: { battery: { check: { status: "failed", ...check } } } });
    for (const code of ["timed_out", "access_denied", "not_available", "crashed", "other"]) expect(failed({ code }).ok, code).toBe(true);
    expect(failed({ code: "Access is denied." }).ok).toBe(false);
  });

  it("still takes the error text reports from Fettle 1.0.3 and earlier carry", () => {
    const r = parseReport({ ...base, facts: { battery: { check: { status: "failed", detail: "Access is denied." } } } });
    expect(r.ok && r.report.facts.battery?.check.detail).toBe("Access is denied.");
  });
});

describe("generated files", () => {
  it("generated file is current", () => {
    const { code } = generate();
    expect(code).toBe(readFileSync(new URL("../src/generated.ts", import.meta.url), "utf8"));
  });
});

describe("edge cases", () => {
  const withFacts = (facts: object) => ({ ...base, facts });

  it("each section accepts data: {} (Vec defaults stay optional)", () => {
    for (const section of ["system", "storage", "battery", "memory", "windows", "startup", "graphics"]) {
      expect(parseReport(withFacts({ [section]: { check: { status: "ok" }, data: {} } })).ok, section).toBe(true);
    }
  });

  it("keeps a memory stick's maker, slot, voltage and form factor", () => {
    const stick = { capacity_bytes: 17179869184, manufacturer: "G.SKILL", bank_label: "P0 CHANNEL A", device_locator: "DIMM 1", configured_voltage_mv: 1100, form_factor: "dimm" };
    const r = parseReport(withFacts({ memory: { check: { status: "ok" }, data: { modules: [stick] } } }));
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.report.facts.memory?.data?.modules?.[0]).toMatchObject(stick);
  });

  it("rejects a newer schema_version", () => {
    const r = parseReport({ ...base, schema_version: SCHEMA_VERSION + 1, facts: {} });
    expect(r.ok).toBe(false);
  });

  it("caps lists where the app cuts them", () => {
    const rust = readFileSync(new URL("../../../crates/fettle-facts/src/lib.rs", import.meta.url), "utf8");
    expect(Number(/pub const ITEMS: usize = (\d+);/.exec(rust)?.[1])).toBe(MAX_ITEMS);
  });

  it("SCHEMA_VERSION matches the Rust constant", () => {
    const rust = readFileSync(new URL("../../../crates/fettle-facts/src/lib.rs", import.meta.url), "utf8");
    expect(Number(/pub const SCHEMA_VERSION: u32 = (\d+);/.exec(rust)?.[1])).toBe(SCHEMA_VERSION);
  });

  it("rejects impossible dates and a non-timestamp collected_at", () => {
    const gpu = (driver_date: string) => withFacts({ graphics: { check: { status: "ok" }, data: { adapters: [{ name: "GPU", driver_date }] } } });
    expect(parseReport(gpu("2024-99-99")).ok).toBe(false);
    expect(parseReport(gpu("2024-09-25")).ok).toBe(true);
    expect(parseReport({ ...base, collected_at: "garbage", facts: {} }).ok).toBe(false);
  });

  it("holds numbers to what the Rust type can carry", () => {
    const win = (data: object) => withFacts({ windows: { check: { status: "ok" }, data } });
    const disk = (data: object) => withFacts({ storage: { check: { status: "ok" }, data: { disks: [{ model: "SSD", ...data }] } } });
    // u32: the build number.
    expect(parseReport(win({ build: 2 ** 32 - 1 })).ok).toBe(true);
    expect(parseReport(win({ build: 2 ** 32 })).ok).toBe(false);
    // u8: NVMe wear.
    expect(parseReport(disk({ nvme: { percentage_used: 255 } })).ok).toBe(true);
    expect(parseReport(disk({ nvme: { percentage_used: 256 } })).ok).toBe(false);
    // i32: a temperature can be below zero, but not below what Rust can read.
    expect(parseReport(disk({ smart: { temperature_c: -5 } })).ok).toBe(true);
    expect(parseReport(disk({ smart: { temperature_c: -(2 ** 31) - 1 } })).ok).toBe(false);
    // u64: JSON numbers stay exact only up to 2^53 - 1.
    expect(parseReport(disk({ size_bytes: Number.MAX_SAFE_INTEGER })).ok).toBe(true);
    expect(parseReport(disk({ size_bytes: 2 ** 60 })).ok).toBe(false);
  });

  it("rejects oversized strings and arrays", () => {
    const disks = (n: number, model = "SSD") => withFacts({ storage: { check: { status: "ok" }, data: { disks: Array.from({ length: n }, () => ({ model })) } } });
    expect(parseReport(disks(2)).ok).toBe(true);
    expect(parseReport(disks(10_000)).ok).toBe(false);
    expect(parseReport(disks(1, "x".repeat(10_000))).ok).toBe(false);
  });

  it("holds each string to its own limit: names 128, versions and dates 64, details 256", () => {
    const ok = (data: object) => ({ check: { status: "ok" }, data });
    const fields: [string, number, (s: string) => object][] = [
      ["app_version", 64, (s) => ({ ...base, app_version: s, facts: {} })],
      ["engine_version", 64, (s) => ({ ...base, engine_version: s, facts: {} })],
      ["check.detail", 256, (s) => withFacts({ battery: { check: { status: "failed", detail: s } } })],
      ["disk.model", 128, (s) => withFacts({ storage: ok({ disks: [{ model: s }] }) })],
      ["volume.letter", 64, (s) => withFacts({ storage: ok({ volumes: [{ letter: s }] }) })],
      ["module.part_number", 128, (s) => withFacts({ memory: ok({ modules: [{ part_number: s }] }) })],
      ["module.manufacturer", 128, (s) => withFacts({ memory: ok({ modules: [{ manufacturer: s }] }) })],
      ["module.bank_label", 128, (s) => withFacts({ memory: ok({ modules: [{ bank_label: s }] }) })],
      ["module.device_locator", 128, (s) => withFacts({ memory: ok({ modules: [{ device_locator: s }] }) })],
      ["windows.product", 128, (s) => withFacts({ windows: ok({ product: s }) })],
      ["windows.edition_id", 64, (s) => withFacts({ windows: ok({ edition_id: s }) })],
      ["windows.display_version", 64, (s) => withFacts({ windows: ok({ display_version: s }) })],
      ["windows.installation_type", 64, (s) => withFacts({ windows: ok({ installation_type: s }) })],
      ["tpm.spec_version", 64, (s) => withFacts({ windows: ok({ tpm: { spec_version: s } }) })],
      ["startup.name", 128, (s) => withFacts({ startup: ok({ items: [{ name: s }] }) })],
      ["gpu.name", 128, (s) => withFacts({ graphics: ok({ adapters: [{ name: s }] }) })],
      ["gpu.driver_version", 64, (s) => withFacts({ graphics: ok({ adapters: [{ driver_version: s }] }) })],
      ["system.cpu_name", 128, (s) => withFacts({ system: ok({ cpu_name: s }) })],
      ["system.board", 128, (s) => withFacts({ system: ok({ board: s }) })],
      ["system.bios_version", 64, (s) => withFacts({ system: ok({ bios_version: s }) })],
      ["display.name", 128, (s) => withFacts({ system: ok({ displays: [{ name: s }] }) })],
    ];
    for (const [field, max, report] of fields) {
      expect(parseReport(report("x".repeat(max))).ok, `${field} at ${max}`).toBe(true);
      expect(parseReport(report("x".repeat(max + 1))).ok, `${field} over ${max}`).toBe(false);
    }
  });

  it("still accepts every real report and test case", () => {
    const cases = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
    const names = readdirSync(cases);
    expect(names.length).toBeGreaterThan(100);
    for (const name of names) {
      const r = parseReport(JSON.parse(readFileSync(new URL(name, cases), "utf8")).report);
      expect(r.ok, `${name}: ${r.ok ? "" : r.error}`).toBe(true);
    }
    const fixtures = new URL("../../../crates/fettle-collect/tests/fixtures/", import.meta.url);
    for (const name of readdirSync(fixtures).filter((n) => n.endsWith(".facts.json"))) {
      const r = parseReport(withFacts(JSON.parse(readFileSync(new URL(name, fixtures), "utf8"))));
      expect(r.ok, `${name}: ${r.ok ? "" : r.error}`).toBe(true);
    }
  });

  it("schema/report.schema.json is current too", () => {
    const { schemaJson } = generate();
    expect(schemaJson).toBe(readFileSync(new URL("../schema/report.schema.json", import.meta.url), "utf8"));
  });
});

describe("strict CSP", () => {
  it("never makes zod probe eval (our pages forbid it, and the probe logs a CSP violation)", async () => {
    const { z } = await import("zod");
    await import("../src/index.ts");
    expect(z.config().jitless).toBe(true);
  });

  it("sets that before the schemas are built (zod reads it at construction)", async () => {
    const { readFileSync } = await import("node:fs");
    const index = readFileSync(new URL("../src/index.ts", import.meta.url), "utf8");
    expect(index.split(/\r?\n/).find((l) => l.startsWith("import"))).toBe('import "./zod-config.ts";');
  });
});
