import type { Level, Receipt } from "@fettle/brain";
import { describe, expect, it } from "vitest";
import { escapeHtml, inlineJson, ogText } from "../src/og.ts";

type Line = Receipt["sections"][number]["lines"][number];

const lines = (level: Level, n: number): Line[] =>
  Array.from({ length: n }, (_, i) => ({ id: `${level}-${i}`, level, headline: { key: "x" } }));

const receipt = (summary: Receipt["summary"]["key"], all: Line[]): Receipt => ({
  summary: { key: summary, actions: [] },
  sections: [{ section: "storage", lines: all.slice(0, 3) }, { section: "windows", lines: all.slice(3) }],
  engineVersion: "0.1.0",
});

const mixed = receipt("summary.attention", [
  ...lines("act_soon", 1), ...lines("keep_an_eye", 3), ...lines("couldnt_check", 2), ...lines("fine", 12), ...lines("info", 1),
]);

describe("ogText", () => {
  it("leads with the summary and counts what matters", () => {
    expect(ogText(mixed, "en")).toEqual({
      title: "Fettle receipt: Needs attention.",
      description: "1 to act on soon · 3 to keep an eye on · 2 couldn't check · 12 fine",
    });
  });

  it("speaks Italian", () => {
    expect(ogText(mixed, "it")).toEqual({
      title: "Scontrino Fettle: Richiede attenzione.",
      description: "1 da sistemare presto · 3 da tenere d'occhio · 2 non verificati · 12 a posto",
    });
    const one = receipt("summary.partial", lines("couldnt_check", 1));
    expect(ogText(one, "it").description).toBe("1 non verificato");
  });

  it("says so plainly when every counted line is fine", () => {
    const fine = receipt("summary.fine", [...lines("fine", 12), ...lines("info", 2)]);
    expect(ogText(fine, "en").description).toBe("12 checks, all fine");
    expect(ogText(fine, "it").description).toBe("12 controlli, tutti a posto");
    const one = receipt("summary.fine", lines("fine", 1));
    expect(ogText(one, "en").description).toBe("1 check, fine");
    expect(ogText(one, "it").description).toBe("1 controllo, a posto");
  });

  it("falls back to a plain line when nothing is counted", () => {
    const quiet = receipt("summary.partial", lines("info", 2));
    expect(ogText(quiet, "en").description).toBe("A PC check-up by Fettle.");
    expect(ogText(quiet, "it").description).toBe("Un controllo del PC fatto con Fettle.");
  });
});

describe("escaping", () => {
  it("escapes HTML attribute text", () => {
    expect(escapeHtml(`"<&>'`)).toBe("&quot;&lt;&amp;&gt;&#39;");
  });

  it("makes JSON safe inside a script element", () => {
    const value = { s: "</script> & " };
    const out = inlineJson(value);
    for (const raw of ["<", ">", "&", " ", " "]) expect(out.includes(raw), JSON.stringify(raw)).toBe(false);
    expect(JSON.parse(out)).toEqual(value);
  });
});
