import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { home } from "../src/i18n/home.ts";
import { pages } from "../src/i18n/pages.ts";

// The mat text colours live in themes.css since the themes work; this pins the Dark (default) values.
const site = readFileSync(new URL("../../../packages/receipt-ui/src/themes.css", import.meta.url), "utf8");
const robots = readFileSync(new URL("../public/robots.txt", import.meta.url), "utf8");

/** WCAG contrast of a paper-warm text colour at `alpha` over the brightest mat green. */
function contrastOnMat(alpha: number): number {
  const fg = [0xf3, 0xf1, 0xea];
  const bg = [0x1f, 0x6a, 0x45];
  const mixed = fg.map((c, i) => c * alpha + bg[i]! * (1 - alpha));
  const lum = (rgb: number[]) => {
    const [r, g, b] = rgb.map((v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
  };
  const [a, b] = [lum(mixed), lum(bg)].sort((x, y) => y - x);
  return (a! + 0.05) / (b! + 0.05);
}

const alphaOf = (name: string) => Number(new RegExp(`--${name}:\\s*rgb\\(243 241 234 / ([0-9.]+)\\)`).exec(site)?.[1]);

describe("contrast, link previews and claims", () => {
  it("small text on the bright mat meets WCAG AA (4.5:1)", () => {
    for (const name of ["on-mat-soft", "on-mat-faint"]) {
      expect(alphaOf(name), name).toBeGreaterThan(0);
      expect(contrastOnMat(alphaOf(name)), name).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("lets link-preview bots fetch share pages (noindex keeps them out of search)", () => {
    expect(robots).not.toMatch(/Disallow:\s*\/r\//);
    expect(robots).not.toMatch(/Disallow:\s*\/share/);
  });

  it("only claims what is true today", () => {
    for (const locale of ["en", "it"] as const) {
      const all = JSON.stringify([faqs(locale), home(locale), pages(locale)]);
      expect(all).not.toMatch(/is applying|Android/);
      expect(all).not.toMatch(/A real receipt|Uno scontrino vero/);
    }
  });

  it("says the server counts by IP address and keeps passes only as hashes", () => {
    const privacy = JSON.stringify(pages("en").privacy.sections);
    expect(privacy).toMatch(/per IP address/);
    expect(privacy).toMatch(/pass/);
    expect(JSON.stringify(pages("it").privacy.sections)).not.toMatch(/anonimizzat|cifrat/);
  });
});
