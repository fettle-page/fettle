import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parseReport } from "../src/index.ts";

// The Rust collector's report from impossible values (crates/fettle-collect/tests/report.rs writes
// it): whatever a PC sends, the share server must accept the report the app makes from it.
describe("a report built from impossible values", () => {
  it("passes the schema", () => {
    const report = JSON.parse(readFileSync(new URL("../../../crates/fettle-collect/tests/fixtures/hostile.report.json", import.meta.url), "utf8"));
    const parsed = parseReport(report);
    expect(parsed.ok ? "ok" : parsed.error).toBe("ok");
  });
});
