import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// Scroll-linked motion: the page moves as you read down it, CSS only.
const site = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const built = [page("index.html"), page("it.html")];

/** Text inside every `@supports (animation-timeline: view())` block. */
function scrollBlocks(css: string): string {
  let out = "";
  for (const m of css.matchAll(/@supports\s*\(animation-timeline:\s*view\(\)\)\s*\{/g)) {
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

/** The declarations of the rule whose selector is exactly `selector`, inside `css`. */
const rule = (css: string, selector: string) =>
  new RegExp(`^\\s*${selector.replace(/[.*+?^${}()|[\]\\>]/g, "\\$&")}\\s*\\{([^}]*)\\}`, "m").exec(css)?.[1] ?? "";

describe("scroll-linked motion", () => {
  const scroll = scrollBlocks(site);

  for (const [selector, name] of [
    [".annotated-receipt :is(.kv > div, .steps li, .r-section h3, .line)", "fettle-print-in"],
    [".redact", "fettle-sweep"],
    [".sheet-back, .sheet-mid", "fettle-fan"],
    [".seal", "fettle-stamp-down"],
    [".questions-slip", "fettle-slip-in"],
    [".legend .tag", "fettle-tag-press"],
    [".privacy-slip", "fettle-slot-feed"],
    [".end-slip", "fettle-tear-off"],
    [".checks li::after", "fettle-tick-draw"],
  ] as const) {
    it(`${selector} moves with the scroll (${name})`, () => {
      const body = rule(scroll, selector);
      expect(body, selector).toContain(`animation-name: ${name}`);
      expect(body, selector).toMatch(/animation-timeline:\s*(view\(\)|--[\w-]+)/);
    });
  }

  it("the privacy slip's bars share one timeline, the slip's own", () => {
    expect(scroll).toMatch(/\.privacy-slip\s*\{[^}]*view-timeline-name:\s*--privacy-slip/);
    expect(rule(scroll, ".redact")).toContain("animation-timeline: --privacy-slip");
  });

  it("the stamp and the fan end exactly where the seal and the sheets rest", () => {
    expect(rule(site, ".seal")).toContain("rotate(-8deg)");
    expect(site).toMatch(/@keyframes fettle-stamp-down\s*\{[\s\S]*?to\s*\{[^}]*rotate\(-8deg\)/);
    // The fan starts as one squared pile: the back sheets begin at the front sheet's pose.
    const front = /\.sheet-front\s*\{[^}]*transform:\s*([^;]+);/.exec(site)?.[1];
    expect(front).toBe("rotate(1deg)");
    expect(site).toMatch(/@keyframes fettle-fan\s*\{\s*from\s*\{\s*transform:\s*rotate\(1deg\) translate\(0, 0\);/);
  });

  it("ships in a form browsers accept (no shorthand with a timeline)", () => {
    for (const html of built) {
      expect(html).not.toMatch(/animation:[^;}]*(view\(|--privacy-slip)/);
      expect(html).toContain("animation-timeline:--privacy-slip");
      expect(html.match(/animation-timeline:view\(\)[;}]/g)?.length ?? 0).toBeGreaterThanOrEqual(6);
    }
  });
});
