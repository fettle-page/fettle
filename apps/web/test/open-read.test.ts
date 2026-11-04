import { readFileSync } from "node:fs";
import { BrainError, brain } from "@fettle/brain/node";
import { describe, expect, it } from "vitest";
import { OPEN_ERRORS, openFile, MAX_FILE_BYTES } from "../src/open/read.ts";
import { openText } from "../src/open/copy.ts";

// What /open does with a picked file, run with the Node brain standing in for the browser's.
const sampleText = readFileSync(new URL("../src/content/sample-report.json", import.meta.url), "utf8");
const sample = () => JSON.parse(sampleText);
const file = (text: string) => new Blob([text], { type: "application/json" });
const zone = { offsetMinutes: 60 };
const load = async () => brain;

describe("opening a saved report", () => {
  it("draws the sample report's receipt in the page's language and the visitor's time", async () => {
    const opened = await openFile(file(sampleText), "it", zone, load);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    expect(opened.model.locale).toBe("it");
    expect(opened.model.sections.length).toBeGreaterThan(0);
    expect(opened.model.meta.when).toBe("07/10/2026 08:12");
  });

  it("reads a report saved with nulls for absent fields, as the app writes them", async () => {
    const report = sample();
    report.facts.system.data.bios_date = null;
    expect((await openFile(file(JSON.stringify(report, null, 2)), "en", zone, load)).ok).toBe(true);
  });

  it("says when the file isn't JSON", async () => {
    expect(await openFile(file("Fettle receipt\nIn fine fettle."), "en", zone, load)).toEqual({ ok: false, error: "not_json" });
    expect(await openFile(file(""), "en", zone, load)).toEqual({ ok: false, error: "not_json" });
  });

  it("says when it's JSON but not a Fettle report", async () => {
    for (const text of ["[]", "null", "42", '{"name":"package"}', JSON.stringify({ ...sample(), facts: "nope" })]) {
      expect(await openFile(file(text), "en", zone, load), text.slice(0, 20)).toEqual({ ok: false, error: "not_fettle" });
    }
  });

  it("says when a newer Fettle made it, even if this version can't make sense of the rest", async () => {
    expect(await openFile(file(JSON.stringify({ ...sample(), schema_version: 2 })), "en", zone, load)).toEqual({ ok: false, error: "newer" });
    expect(await openFile(file(JSON.stringify({ schema_version: 9, facts: { all: "new" } })), "en", zone, load)).toEqual({ ok: false, error: "newer" });
  });

  it("says it couldn't be read when the file can't be read, or the receipt refuses it", async () => {
    const broken = { size: 10, text: () => Promise.reject(new Error("NotReadableError")) } as unknown as Blob;
    expect(await openFile(broken, "en", zone, load)).toEqual({ ok: false, error: "unreadable" });
    const refuses = async () => ({
      receiptFor: () => {
        throw new BrainError("collected_at isn't a real time");
      },
    });
    expect(await openFile(file(sampleText), "en", zone, refuses)).toEqual({ ok: false, error: "unreadable" });
  });

  it("doesn't read a file far bigger than any report", async () => {
    let read = false;
    const huge = { size: MAX_FILE_BYTES + 1, text: async () => ((read = true), sampleText) } as unknown as Blob;
    expect(await openFile(huge, "en", zone, load)).toEqual({ ok: false, error: "not_fettle" });
    expect(read).toBe(false);
  });

  it("says so when the part that draws receipts couldn't be loaded", async () => {
    const offline = () => Promise.reject(new TypeError("Failed to fetch"));
    expect(await openFile(file(sampleText), "en", zone, offline)).toEqual({ ok: false, error: "offline" });
  });

  it("checks the file before loading the receipt", async () => {
    let loaded = false;
    const watch = async () => ((loaded = true), brain);
    await openFile(file("{"), "en", zone, watch);
    expect(loaded).toBe(false);
  });
});

describe("the messages", () => {
  it("exist in English and Italian for every outcome, each its own, calm and plain", () => {
    for (const locale of ["en", "it"] as const) {
      const lines = OPEN_ERRORS.map((e) => openText(locale, `error.${e}`));
      expect(new Set(lines).size).toBe(OPEN_ERRORS.length);
      for (const line of lines) {
        expect(line.length).toBeGreaterThan(10);
        expect(line).not.toContain("!");
      }
    }
    expect(openText("it", "error.not_json")).not.toBe(openText("en", "error.not_json"));
  });
});
