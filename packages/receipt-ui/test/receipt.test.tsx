import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ReceiptModel } from "@fettle/brain";
import { Receipt } from "../src/Receipt.tsx";
import { battery, model, nvme, startup, storage } from "./helpers/report.ts";

const html = (m: ReceiptModel, printing = false) => renderToStaticMarkup(<Receipt model={m} printing={printing} />);
const count = (s: string, re: RegExp) => (s.match(re) ?? []).length;

const handBuilt = (headline: string): ReceiptModel => ({
  locale: "en",
  tone: "fine",
  summary: "In fine fettle.",
  actions: [],
  machine: [],
  sections: [{ id: "graphics", title: "GRAPHICS", lines: [{ id: "g", level: "fine", tag: "OK", levelText: "Fine", headline }], details: [] }],
  meta: { when: "07/10/2026 14:32", collectedAt: "2026-10-07T14:32:00+02:00", deep: false, footer: "Fettle 0.1.0 · verdicts 0.1.0", scan: "Quick check (no administrator)" },
  labels: { details: "details", nextStep: "Next step", machine: "THIS PC", stamp: "Checked" },
});

describe("<Receipt>", () => {
  const m = model({ storage: storage([nvme(3)]), battery: battery(50000, 25000), startup: startup(18) });
  const out = html(m);

  it("marks each section with its id, for hosts that annotate it", () => {
    for (const s of m.sections) expect(out).toContain(`data-section="${s.id}"`);
    expect(count(out, /data-section=/g)).toBe(m.sections.length);
  });

  it("summary is the h2 and each section has an h3", () => {
    expect(out).toMatch(/<h2[^>]*>Needs attention\.<\/h2>/);
    expect(count(out, /<h3/g)).toBe(m.sections.length);
  });

  it("every line shows its tag word and a screen-reader level", () => {
    expect(out).toContain(">ACT<");
    expect(out).toContain(">WATCH<");
    expect(out).toContain(">OK<");
    expect(out).toContain("Act soon: ");
    expect(count(out, /class="sr-only"/g)).toBeGreaterThanOrEqual(3);
  });

  it("tag stays in the first row: sr text, then headline, then tag", () => {
    const li = out.match(/<li[^>]*data-level="act_soon"[^>]*>[\s\S]*?<\/li>/)?.[0] ?? "";
    expect(li).toMatch(/class="sr-only"[\s\S]*class="head"[\s\S]*class="tag/);
  });

  it("next steps are listed under the summary", () => {
    expect(out).toContain("Next step");
    expect(out).toMatch(/<ol[\s\S]*A replacement battery would bring the runtime back\.[\s\S]*<\/ol>/);
  });

  it("details folds only where there are details", () => {
    expect(out).toContain("<details");
    expect(html(handBuilt("RX 6900 XT is installed."))).not.toContain("<details");
  });

  it("renders hostile names as text", () => {
    const hostile = html(handBuilt('<img src=x onerror=alert(1)>'));
    expect(hostile).toContain("&lt;img");
    expect(hostile).not.toContain("<img");
  });

  it("marks the language and the tone", () => {
    const it = html(model({ startup: startup(18) }, "it-IT"));
    expect(it).toMatch(/<article[^>]*lang="it"/);
    expect(out).toMatch(/<article[^>]*data-tone="attention"/);
  });

  it("the header carries the real wordmark, not typed text", () => {
    expect(out).toMatch(/<svg[^>]*class="wordmark"[^>]*role="img"[^>]*aria-label="Fettle"/);
    expect(out).not.toContain(">fettle.<");
  });

  it("printing staggers the lines", () => {
    const printing = html(m, true);
    expect(printing).toContain("data-printing");
    expect(printing).toContain("--i:0");
    expect(out).not.toContain("data-printing");
  });
});
