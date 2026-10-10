import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { uiText as ui } from "@fettle/brain";
import { home } from "../src/i18n/home.ts";
import { page, text } from "./dist-helpers.ts";

const site = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const built = { en: page("index.html"), it: page("it.html") } as const;
const decode = (s: string) => s.replace(/&#39;/g, "'").replace(/&#x27;/g, "'");

/** Text of every `@supports (...) { ... }` block whose condition matches `condition`. */
function supportsBlocks(css: string, condition: RegExp): string {
  let out = "";
  for (const m of css.matchAll(/@supports\s*([^{]*)\{/g)) {
    if (!condition.test(m[1]!)) continue;
    let depth = 1;
    let i = m.index! + m[0].length;
    const start = i;
    while (depth > 0 && i < css.length) {
      if (css[i] === "{") depth++;
      else if (css[i] === "}") depth--;
      i++;
    }
    out += css.slice(start, i - 1);
  }
  return out;
}

describe("the annotated sample receipt", () => {
  for (const locale of ["en", "it"] as const) {
    const html = built[locale];
    const h = home(locale);

    it(`${locale}: has one annotated receipt instead of a strip and a tray`, () => {
      expect(html).not.toContain('class="strip"');
      expect(html).not.toContain('class="tray"');
      expect(html).not.toContain("bin-");
      expect(html.match(/class="fettle-receipt/g) ?? []).toHaveLength(1);
      expect(html).toContain('id="how"');
      expect(html).toContain('id="sample"');
    });

    it(`${locale}: has a note for the header and for every receipt section`, () => {
      const sections = [...html.matchAll(/data-section="([a-z]+)"/g)].map(([, id]) => id!);
      expect(sections.length).toBeGreaterThanOrEqual(6);
      for (const id of sections) expect(html, id).toContain(`data-note-for="${id}"`);
      expect(html).toContain('data-note-for="machine"');
    });

    it(`${locale}: puts the legend beside the intro, not among the notes`, () => {
      const how = html.slice(html.indexOf('id="how"'), html.indexOf('class="annotated"'));
      expect(how).toContain('class="legend');
      expect(html.slice(html.indexOf('class="notes"'), html.indexOf('id="sample"'))).not.toContain('class="legend');
    });

    // A note alone ("How much charge it still holds…") needs its section to make sense.
    it(`${locale}: names each note's section for screen readers`, () => {
      for (const [id, key] of [["machine", "section.system"], ["storage", "section.storage"], ["battery", "section.battery"]] as const) {
        const start = html.indexOf(`data-note-for="${id}"`);
        const note = html.slice(start, html.indexOf("</p>", start));
        expect(note, id).toMatch(new RegExp(`class="sr-only">${ui(key, locale)}`));
      }
      expect(html).not.toContain(`class="notes" aria-label="${h.noteLabel}"`);
    });

    it(`${locale}: keeps the notes out of the receipt`, () => {
      const start = html.indexOf('<article class="fettle-receipt');
      const receipt = decode(text(html.slice(start, html.indexOf("</article>", start))));
      for (const note of [h.trayStorage, h.trayBattery, h.trayMemory, h.trayWindows, h.trayStartup, h.trayGraphics, h.traySystem]) {
        expect(receipt).not.toContain(note);
      }
    });

    it(`${locale}: points three principles at their proof`, () => {
      const page = decode(text(html));
      for (const [id, heading] of [["plain", h.callPlainH], ["numbers", h.callNumbersH], ["unchecked", h.callUncheckedH]] as const) {
        expect(html.match(new RegExp(`data-callout="${id}"`, "g")) ?? [], id).toHaveLength(1);
        expect(page).toContain(heading);
      }
    });
  }

  // The minifier once folded animation-timeline into the shorthand, which Chromium rejects.
  it("ships scroll-driven motion that browsers accept", () => {
    for (const html of [built.en, built.it]) {
      expect(html).not.toMatch(/animation:[^;}]*view\(/);
      expect(html.match(/animation-timeline:view\(\)[;}]/g)?.length ?? 0).toBeGreaterThanOrEqual(3);
    }
  });

  // The pale mat focus ring once vanished on the download slip's paper.
  it("gives the paper slips the dark focus ring", () => {
    const themes = readFileSync(new URL("../../../packages/receipt-ui/src/themes.css", import.meta.url), "utf8");
    const rule = /:is\(([^)]*)\)\s*:focus-visible/.exec(themes)?.[1] ?? "";
    expect(rule).toContain(".slip");
  });

  it("anchors only where the browser supports it", () => {
    const anchored = supportsBlocks(site, /anchor-name/);
    const outside = site.replace(/@supports\s*\(anchor-name[^{]*\{(?:[^{}]*\{[^{}]*\})*\s*\}/g, "");
    expect(anchored).toContain("position-anchor");
    expect(outside).not.toMatch(/position-anchor|anchor\(/);
  });

  it("leaves the A4 sketch's note chips alone", () => {
    // The technicians' A4 sketch has <span class="sheet-chip note">; the margin notes must not style it.
    const bare = [...site.matchAll(/([^{}]*)\.note(?![-\w])/g)].map(([, before]) => before!.trim().split(/\s|,/).pop());
    for (const prefix of bare) expect(prefix).toBe(".sheet-chip");
  });

  it("is fully visible at rest", () => {
    for (const selector of [".margin-note", ".callout", ".end-slip", ".questions-slip", ".seal", ".sheet-back", ".sheet-mid", ".redact"]) {
      const rule = new RegExp(`(^|\\})\\s*\\${selector}\\s*\\{([^}]*)\\}`, "m").exec(site)?.[2] ?? "";
      expect(rule, selector).not.toMatch(/opacity:\s*0[;\s]/);
    }
  });
});
