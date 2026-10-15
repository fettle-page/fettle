import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { stamp } from "../src/stamp.ts";

// The one formatting the browser still does itself: once a share page is on
// screen, the scan time is shown in the viewer's own zone. In UTC it must read exactly as the Rust
// receipt writes it, on every case, or the page would jump when it swaps.
const DIR = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
const cases = readdirSync(DIR).sort().map((f) => JSON.parse(readFileSync(new URL(f, DIR), "utf8")));

describe("stamp", () => {
  it("in UTC matches the Rust receipt on every case, in both languages", () => {
    expect(cases.length).toBeGreaterThan(300);
    for (const c of cases) {
      for (const locale of ["en", "it"]) {
        const { meta } = c[locale];
        expect(stamp(meta.collectedAt, "UTC"), `${c.name} ${locale}`).toBe(meta.when);
      }
    }
  });

  it("shows a viewer's own zone without a label", () => {
    expect(stamp("2026-10-07T20:29:30Z", "Europe/Rome")).toBe("07/10/2026 22:29");
  });

  it("leaves a time it can't read as it is", () => {
    expect(stamp("not a time", "UTC")).toBe("not a time");
  });
});
