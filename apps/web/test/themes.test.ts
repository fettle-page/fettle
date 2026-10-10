import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../../../packages/receipt-ui/src/themes.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** Declarations (`--name: value`) inside the first block whose selector matches `selector` exactly. */
function block(selector: string, source = css): Record<string, string> {
  const at = source.indexOf(`${selector} {`);
  if (at < 0) throw new Error(`no block for ${selector}`);
  const body = source.slice(source.indexOf("{", at) + 1, source.indexOf("}", at));
  return Object.fromEntries([...body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map(([, k, v]) => [k!, v!.trim()]));
}

function mediaBody(query: string): string {
  const at = css.indexOf(`@media ${query}`);
  if (at < 0) throw new Error(`no @media ${query}`);
  let depth = 0;
  let i = css.indexOf("{", at);
  const start = i + 1;
  do {
    if (css[i] === "{") depth++;
    else if (css[i] === "}") depth--;
    i++;
  } while (depth > 0);
  return css.slice(start, i - 1);
}

type RGB = [number, number, number];
const hex = (h: string): RGB => [1, 3, 5].map((i) => Number.parseInt(h.slice(i, i + 2), 16)) as RGB;
/** "#RRGGBB" or "rgb(r g b / a)" over a background. */
function solid(value: string, over: RGB): RGB {
  if (value.startsWith("#")) return hex(value);
  const m = /rgb\((\d+) (\d+) (\d+) \/ ([\d.]+)\)/.exec(value);
  if (!m) throw new Error(`can't read colour ${value}`);
  const a = Number(m[4]);
  return [1, 2, 3].map((i, k) => Number(m[i]) * a + over[k]! * (1 - a)) as RGB;
}
function contrast(a: RGB, b: RGB): number {
  const lum = (c: RGB) => {
    const [r, g, bl] = c.map((v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r! + 0.7152 * g! + 0.0722 * bl!;
  };
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x! + 0.05) / (y! + 0.05);
}

const themes = {
  dark: block(":root"),
  light: block('[data-theme="light"]'),
  oled: block('[data-theme="oled"]'),
};

describe("theme tokens", () => {
  for (const [name, t] of Object.entries(themes)) {
    describe(name, () => {
      const mat = hex(t["--t-mat"]!);

      it("keeps text on the mat readable (WCAG AA)", () => {
        for (const key of ["--on-mat", "--on-mat-soft", "--on-mat-faint", "--t-accent"]) {
          expect(contrast(solid(t[key]!, mat), mat), `${name} ${key}`).toBeGreaterThanOrEqual(4.5);
        }
      });

      it("keeps ink and tags on its paper readable (WCAG AA)", () => {
        const paper = hex(t["--color-paper"]!);
        for (const key of ["--color-ink", "--color-ink-soft", "--color-accent-ink", "--color-mat"]) {
          expect(contrast(hex(t[key]!), paper), `${name} ${key}`).toBeGreaterThanOrEqual(4.5);
        }
      });

      it("keeps the tape labels and buttons readable", () => {
        expect(contrast(hex(t["--t-tape-ink"]!), hex(t["--t-tape"]!)), `${name} tape`).toBeGreaterThanOrEqual(4.5);
        expect(contrast(hex(t["--t-btn-ink"]!), hex(t["--t-btn"]!)), `${name} button`).toBeGreaterThanOrEqual(4.5);
      });
    });
  }

  it("uses true black for OLED", () => {
    expect(themes.oled["--t-mat"]).toBe("#000000");
    expect(themes.oled["--t-page"]).toBe("#000000");
  });

  it("dims the paper in Dark and OLED, never in Light", () => {
    expect(themes.light["--color-paper"]).toBe("#FBFAF6");
    expect(themes.dark["--color-paper"]).toBe("#EDEBE4");
    expect(themes.oled["--color-paper"]).toBe("#CFCCC3");
  });

  it("follows the device with the same Light tokens when nobody has chosen", () => {
    const media = mediaBody("(prefers-color-scheme: light)");
    expect(block(":root:not([data-theme])", media)).toEqual(themes.light);
  });

  it("always prints on white paper", () => {
    const print = mediaBody("print");
    expect(print).toMatch(/--color-paper:\s*#FFFFFF/i);
    expect(print).toMatch(/--t-mat:\s*#FFFFFF/i);
  });
});

describe("paper surfaces in dark themes", () => {
  const view = readFileSync(new URL("../../../packages/receipt-ui/src/view.css", import.meta.url), "utf8");
  const receipt = readFileSync(new URL("../../../packages/receipt-ui/src/receipt.css", import.meta.url), "utf8");

  it("keep light native controls (checkboxes, radios) on paper", () => {
    const raw = readFileSync(new URL("../../../packages/receipt-ui/src/themes.css", import.meta.url), "utf8");
    const rule = /\*\/\s*([^{}]+)\{\s*color-scheme:\s*light;\s*\}/.exec(raw.slice(raw.lastIndexOf("/* Paper is always light")));
    expect(rule, "a paper color-scheme rule").not.toBeNull();
    for (const sel of [".fettle-receipt", ".fettle-bench", ".card", ".doc-sheet", ".faq-item", ".foot-slip"]) expect(rule![1], sel).toContain(sel);
  });

  it("draw rules and tab borders from the paper and ink, so they show on dimmed paper", () => {
    expect(view).toMatch(/--b-rule:\s*color-mix\(/);
    expect(view).toMatch(/\[role="tab"\] \{[^}]*border: 1px solid color-mix\(/);
    expect(receipt).toMatch(/--r-rule:\s*color-mix\(/);
  });
});

describe("themes on paper and in print", () => {
  it("prints text on the mat in dark ink, whatever the theme", () => {
    const print = mediaBody("print");
    expect(print).toMatch(/--on-mat:\s*#000000/i);
    expect(print).toMatch(/--on-mat-soft:\s*#000000/i);
    expect(print).toMatch(/--on-mat-faint:\s*#3A3A3A/i);
    expect(print).toMatch(/--t-accent:\s*#1F6A45/i);
  });

  it("gives focus rings on paper the dark green, in every theme", () => {
    const rule = /:is\(([^)]*)\)\s*:focus-visible:focus-visible\s*\{\s*outline-color:\s*var\(--color-mat\);\s*\}/.exec(css);
    expect(rule, "paper focus rule").not.toBeNull();
    for (const sel of [".fettle-receipt", ".fettle-bench", ".card", ".doc-sheet", ".faq-item", ".foot-slip"]) expect(rule![1], sel).toContain(sel);
  });
});
