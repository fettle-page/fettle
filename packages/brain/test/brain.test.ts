import { readdirSync, readFileSync } from "node:fs";
import { brotliCompressSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { LOCALES, text, toLocale, uiText } from "../src/index.ts";
import { BrainError } from "../src/core.ts";
import { brain } from "../src/node.ts";

const CASES = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
const files = readdirSync(CASES).sort();
const report = (f: string) => JSON.parse(readFileSync(new URL(f, CASES), "utf8")).report;
const copy = (crate: string, locale: string) => JSON.parse(readFileSync(new URL(`../../../crates/${crate}/copy/${locale}.json`, import.meta.url), "utf8"));

describe("the brain", () => {
  it("a report it can't read throws BrainError, with the reason", () => {
    const bad = { ...report(files[0]!), collected_at: "2026-13-40T99:00:00Z" };
    expect(() => brain.receiptFor(bad, "en")).toThrow(BrainError);
  });

  it("an offset zone shows local time without UTC", () => {
    const { model } = brain.receiptFor(report(files[0]!), "it", { offsetMinutes: 120 });
    expect(model.meta.when).toBe("06/10/2026 12:35");
  });

  it("text() reads the crates' copy, in both catalogs", () => {
    expect(text("tag.keep_an_eye", "it")).toBe(copy("fettle-receipt", "it")["tag.keep_an_eye"]);
    expect(text("level.act_soon", "en")).toBe(copy("fettle-verdicts", "en")["level.act_soon"]);
    expect(uiText("tag.keep_an_eye", "en")).toBe(copy("fettle-receipt", "en")["tag.keep_an_eye"]);
  });

  it("unknown keys throw, so a typo fails the build", () => {
    expect(() => text("no.such.key", "en")).toThrow(/unknown copy key: no\.such\.key/);
    // The browser's lookup has only the receipt's labels: verdict lines never reach the share page.
    expect(() => uiText("level.act_soon", "en")).toThrow(/unknown copy key/);
  });

  it("speaks English and Italian, and English for anything else", () => {
    expect(LOCALES).toEqual(["en", "it"]);
    expect(toLocale("it-IT")).toBe("it");
    expect(toLocale("fr")).toBe("en");
  });

  // Wasm memory grows in 64 KiB pages, so across these 1,800 calls this only sees a leak above about
  // 36 bytes per call. Enough for this interface, whose only allocations are the report and the
  // result (kilobytes each); a finer leak would need the allocator's own numbers.
  it("memory settles: thousands of receipts in one module don't keep growing it", () => {
    const largest = files.map((f) => ({ f, n: readFileSync(new URL(f, CASES)).length })).sort((a, b) => b.n - a.n)[0]!.f;
    const r = report(largest);
    let after200 = 0;
    for (let i = 1; i <= 2000; i++) {
      brain.receiptFor(r, i % 2 ? "en" : "it");
      if (i === 200) after200 = brain.memory.buffer.byteLength;
    }
    expect(brain.memory.buffer.byteLength).toBe(after200);
  });

  it("the module is small: at most 150 KB brotli", () => {
    const wasm = readFileSync(new URL("../dist/fettle.wasm", import.meta.url));
    expect(brotliCompressSync(wasm).length).toBeLessThanOrEqual(150 * 1024);
  });
});

describe("the share server's privacy pass", () => {
  it("hides what needs no names, and a second run changes nothing", () => {
    const r = report(files[0]!);
    r.facts.storage = { check: { status: "failed", detail: "no reply from john.smith@example.com at 10.0.0.7" } };
    const once = brain.redactReport(r);
    expect(once.facts.storage?.check.detail).toBe("no reply from <email> at <ip>");
    expect(brain.redactReport(once)).toEqual(once);
  });

  it("a report it can't read throws BrainError and gives nothing back", () => {
    expect(() => brain.redactReport({ not: "a report" })).toThrow(BrainError);
  });
});
