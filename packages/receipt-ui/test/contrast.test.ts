import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const theme = readFileSync(new URL("../src/theme.css", import.meta.url), "utf8");
const token = (name: string) => {
  const hex = new RegExp(`--color-${name}:\\s*(#[0-9a-fA-F]{6})`).exec(theme)?.[1];
  if (!hex) throw new Error(`theme.css has no --color-${name}`);
  return hex;
};

/** WCAG 2.x relative luminance and contrast ratio. */
function luminance(hex: string) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}
const ratio = (a: string, b: string) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
};

describe("colour contrast on thermal paper (WCAG 2.2 AA)", () => {
  const paper = token("paper");

  it("pins the brand colours", () => {
    expect(token("mat").toLowerCase()).toBe("#1f6a45");
    expect(paper.toLowerCase()).toBe("#fbfaf6");
    expect(token("accent").toLowerCase()).toBe("#d9481e");
  });

  for (const name of ["ink", "ink-soft", "accent-ink", "mat"]) {
    it(`${name} text is at least 4.5:1`, () => {
      expect(ratio(token(name), paper)).toBeGreaterThanOrEqual(4.5);
    });
  }

  it("the stamp's small date uses the darker accent ink (4.5:1 at that size)", () => {
    const css = readFileSync(new URL("../src/receipt.css", import.meta.url), "utf8");
    expect(css).toMatch(/\[data-tone="some"\], \[data-tone="attention"\]\) \.stamp small\s*\{[^}]*color:\s*var\(--r-accent-ink\)/);
    expect(ratio(token("accent-ink"), paper)).toBeGreaterThanOrEqual(4.5);
  });

  it("the stamp accent is at least 3:1 (large text only)", () => {
    expect(ratio(token("accent"), paper)).toBeGreaterThanOrEqual(3);
  });

  it("paper text on the ACT tag background is at least 4.5:1", () => {
    expect(ratio(paper, token("accent-ink"))).toBeGreaterThanOrEqual(4.5);
  });
});
