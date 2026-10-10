import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { generate } from "../scripts/generate.mjs";
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

  it("rejects oversized strings and arrays", () => {
    const disks = (n: number, model = "SSD") => withFacts({ storage: { check: { status: "ok" }, data: { disks: Array.from({ length: n }, () => ({ model })) } } });
    expect(parseReport(disks(2)).ok).toBe(true);
    expect(parseReport(disks(10_000)).ok).toBe(false);
    expect(parseReport(disks(1, "x".repeat(10_000))).ok).toBe(false);
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
