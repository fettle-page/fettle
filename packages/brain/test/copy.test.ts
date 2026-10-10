import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// The copy catalogs are the source of truth since the TypeScript engine was retired (2026-10-10), and
// regenerating the goldens would bless whatever the Rust receipt prints, a raw key included. So the
// rules the old TypeScript copy tests enforced live here.
const read = (path: string) => JSON.parse(readFileSync(new URL(`../../../crates/${path}`, import.meta.url), "utf8")) as Record<string, string>;
const CATALOGS = {
  verdicts: { en: read("fettle-verdicts/copy/en.json"), it: read("fettle-verdicts/copy/it.json") },
  receipt: { en: read("fettle-receipt/copy/en.json"), it: read("fettle-receipt/copy/it.json") },
};
const DIR = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
const cases = readdirSync(DIR).sort().map((f) => JSON.parse(readFileSync(new URL(f, DIR), "utf8")));

type Ref = { key: string };
type Line = { level: string; headline: Ref; detail?: Ref; action?: Ref };

describe("the copy catalogs", () => {
  for (const [name, { en, it: itCopy }] of Object.entries(CATALOGS)) {
    it(`${name}: Italian has exactly the English keys, none of them empty`, () => {
      expect(Object.keys(itCopy).sort()).toEqual(Object.keys(en).sort());
      for (const [key, value] of [...Object.entries(en), ...Object.entries(itCopy)]) expect(value.trim(), key).not.toBe("");
    });

    it(`${name}: Italian fills the same {slots} as English`, () => {
      const slots = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
      for (const key of Object.keys(en)) expect(slots(itCopy[key] ?? ""), key).toEqual(slots(en[key]!));
    });
  }

  it("no key is in both catalogs (a lookup would quietly pick one)", () => {
    const shared = Object.keys(CATALOGS.verdicts.en).filter((k) => k in CATALOGS.receipt.en);
    expect(shared).toEqual([]);
  });

  it("every message the verdicts use on the corpus has copy", () => {
    const missing = new Set<string>();
    const need = (ref?: Ref) => ref && !(ref.key in CATALOGS.verdicts.en) && missing.add(ref.key);
    for (const c of cases) {
      need({ key: c.receipt.summary.key });
      c.receipt.summary.actions.forEach(need);
      for (const s of c.receipt.sections) for (const l of s.lines as Line[]) [l.headline, l.detail, l.action].forEach(need);
    }
    expect([...missing]).toEqual([]);
  });

  it("no receipt on the corpus shows a raw copy key", () => {
    // Dotted keys only ("storage.no_health"): a few keys are plain words that are also fine labels.
    const keys = new Set([...Object.keys(CATALOGS.verdicts.en), ...Object.keys(CATALOGS.receipt.en)].filter((k) => k.includes(".")));
    for (const c of cases) {
      for (const locale of ["en", "it"]) {
        const shown = JSON.stringify(c[locale]);
        for (const key of keys) expect(shown.includes(`"${key}"`), `${c.name} ${locale} shows ${key}`).toBe(false);
      }
    }
  });

  it("tags fit the till roll: 5 characters in English, 6 in Italian", () => {
    for (const [locale, max] of [["en", 5], ["it", 6]] as const) {
      for (const [key, value] of Object.entries(CATALOGS.receipt[locale]).filter(([k]) => k.startsWith("tag."))) {
        expect(value.length, `${locale} ${key}`).toBeLessThanOrEqual(max);
      }
    }
  });
});
