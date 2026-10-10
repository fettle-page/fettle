import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { Locale } from "../src/index.ts";
import { brain } from "../src/node.ts";

// The same 405 cases the Rust parity test reads (crates/fettle-receipt/tests/parity.rs): what the
// website gets through the .wasm must be exactly what the goldens say, in both languages.
const DIR = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
type Case = { name: string; report: never; receipt: unknown; en: unknown; it: unknown; markdownEn: string; markdownIt: string };
const cases: Case[] = readdirSync(DIR).sort().map((f) => JSON.parse(readFileSync(new URL(f, DIR), "utf8")));

describe("the Rust receipt through WebAssembly", () => {
  it("has the whole corpus", () => {
    expect(cases.length).toBeGreaterThanOrEqual(405);
  });

  it("every case matches the goldens in both languages", () => {
    for (const c of cases) {
      for (const locale of ["en", "it"] as Locale[]) {
        const { receipt, model } = brain.receiptFor(c.report, locale);
        expect(model, `${c.name} ${locale} model`).toEqual(c[locale]);
        expect(receipt, `${c.name} receipt`).toEqual(c.receipt);
        expect(brain.markdownFor(c.report, locale), `${c.name} ${locale} markdown`).toBe(locale === "en" ? c.markdownEn : c.markdownIt);
      }
    }
  });
});
