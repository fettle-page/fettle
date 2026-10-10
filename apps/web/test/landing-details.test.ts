import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { home } from "../src/i18n/home.ts";
import { page } from "./dist-helpers.ts";

// Small details of the landing page: layout, motion, wording and reading order.
const strip = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, "");
const site = strip(readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8"));
const themes = strip(readFileSync(new URL("../../../packages/receipt-ui/src/themes.css", import.meta.url), "utf8"));
const built = [page("index.html"), page("it.html")];

describe("landing page details", () => {
  it("breakpoints leave no gap at fractional widths", () => {
    expect(site).not.toMatch(/(max|min)-width:\s*(899|900|1199|1200)px/);
    expect(site).toMatch(/@media \(width < 900px\)/);
    expect(site).toMatch(/@media \(width < 1200px\)/);
  });

  it("leader lines don't look like the mat's grid", () => {
    for (const selector of [".margin-note::after", ".callout::before"]) {
      const rule = new RegExp(`^\\s*\\${selector}\\s*\\{([^}]*)\\}`, "m").exec(site)?.[1] ?? "";
      expect(rule, selector).toMatch(/background:\s*var\(--t-accent\)/);
    }
  });

  it("principle tapes don't lift like links", () => {
    expect(site).toMatch(/\.callout \.tape:hover\s*\{[^}]*transform:\s*none/);
  });

  it("nothing overshoots: no easing curve goes past its end", () => {
    for (const [, args] of site.matchAll(/cubic-bezier\(([^)]*)\)/g)) {
      const [, y1, , y2] = args!.split(",").map(Number);
      expect(Math.max(y1!, y2!), args).toBeLessThanOrEqual(1);
    }
    expect(site).not.toContain("--spring");
  });

  it("the A4 printout ends in a fade inside the sheet, clear of the stamp", () => {
    const col = /\.ps-col\s*\{([^}]*)\}/.exec(site)?.[1] ?? "";
    expect(col).toMatch(/height:\s*\d+cqi/);
    expect(col).toMatch(/overflow:\s*hidden/);
  });

  it("screen readers don't read the More questions arrows aloud", () => {
    expect(site).toMatch(/\.more-slip li a::after\s*\{[^}]*content:\s*" →"\s*\/\s*""/);
  });

  it("the old parts-tray colours are gone", () => {
    expect(themes).not.toMatch(/--t-(tray|bin):/);
  });

  for (const html of built) {
    const lang = html.includes('<html lang="it"') ? "it" : "en";

    it(`${lang}: reads the principles in the order they appear on screen`, () => {
      const order = [...html.matchAll(/data-callout="([a-z]+)"/g)].map(([, id]) => id);
      expect(order).toEqual(["plain", "unchecked", "numbers"]);
    });

    it(`${lang}: the receipt still has what the principles point at`, () => {
      const start = html.indexOf('<article class="fettle-receipt');
      const receipt = html.slice(start, html.indexOf("</article>", start));
      expect(receipt).toMatch(/class="r-summary"[\s\S]*?<h2/);
      const storage = receipt.slice(receipt.indexOf('data-section="storage"'));
      expect(storage.slice(0, storage.indexOf("</section>"))).toContain('class="fold"');
      expect(receipt).toContain('data-level="couldnt_check"');
    });
  }

  it("says each idea once: the intro doesn't repeat the principles", () => {
    expect(home("en").sampleP).not.toMatch(/details|what it means/i);
    expect(home("it").sampleP).not.toMatch(/dettagli|cosa significa/i);
  });
});
