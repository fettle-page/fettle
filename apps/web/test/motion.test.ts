import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const site = read("../src/styles/site.css");
const share = read("../src/share/share.css");
const receipt = read("../../../packages/receipt-ui/src/receipt.css");

/** Every @keyframes block, as [name, body]. */
const named = (css: string) => [...css.matchAll(/@keyframes\s+([\w-]+)\s*\{((?:[^{}]*\{[^{}]*\})*)\s*\}/g)].map(([, name, body]) => [name!, body!] as const);
/** The bodies of every @keyframes block. */
const keyframes = (css: string) => named(css).map(([, body]) => body);
const CLIPPED = ["fettle-wipe", "fettle-tick-draw", "fettle-spread", "fettle-slot-feed"];

/** Text inside every `prefers-reduced-motion: reduce` block. */
function reducedBlocks(css: string): string {
  let out = "";
  for (const m of css.matchAll(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{/g)) {
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

/** Selectors of top-level rules that declare an animation (not "none"). */
function animated(css: string): string[] {
  const outside = css.replace(/@media[^{]*\{(?:[^{}]*\{[^{}]*\})*\s*\}/g, "").replace(/@keyframes[^{]*\{(?:[^{}]*\{[^{}]*\})*\s*\}/g, "");
  return [...outside.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .filter(([, , body]) => /(^|;|\s)animation(-name)?\s*:(?!\s*none)/.test(body!)) // "animation: none" isn't one
    .map(([, sel]) => sel!.trim());
}

describe("motion", () => {
  it("animates only transform and opacity", () => {
    for (const [name, body] of [...named(site), ...named(receipt), ...named(share)]) {
      const props = [...body.matchAll(/([a-z-]+)\s*:/g)].map(([, p]) => p);
      // Two exceptions: the theme wipe clips a page snapshot, and the
      // technicians' ticks are drawn by clipping an 8 × 5 px mark; round 2 adds the theme spread and the
      // privacy slip's slot. clip-path only in these.
      const allowed = CLIPPED.includes(name) ? ["clip-path", "transform", "opacity"] : ["transform", "opacity"];
      expect(props.filter((p) => !allowed.includes(p!)), name).toEqual([]);
    }
  });

  it("has something to animate on the landing page", () => {
    expect(keyframes(site).length).toBeGreaterThanOrEqual(2);
    expect(animated(site).length).toBeGreaterThanOrEqual(2);
  });

  it("switches every animation off for people who ask for less motion", () => {
    const reduced = reducedBlocks(site);
    for (const selector of animated(site)) expect(reduced, selector).toContain(selector);
  });

  it("finishes printing a receipt within about 600 ms, however long it is", () => {
    expect(receipt).toMatch(/animation-delay:\s*min\(calc\(var\(--i, 0\) \* 40ms\), 560ms\)/);
  });
});
