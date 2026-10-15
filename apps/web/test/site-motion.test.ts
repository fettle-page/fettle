import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// Website motion: paper-like, no bounce, off for people who ask for less motion.
const site = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** Text inside every at-rule block whose opening matches `at` (brace-balanced). */
function atBlocks(css: string, at: RegExp): string {
  let out = "";
  for (const m of css.matchAll(new RegExp(at.source + String.raw`\s*\{`, "g"))) {
    let depth = 1;
    let i = m.index! + m[0].length;
    const start = i;
    while (depth > 0 && i < css.length) {
      if (css[i] === "{") depth++;
      else if (css[i] === "}") depth--;
      i++;
    }
    out += css.slice(start, i - 1) + "\n";
  }
  return out;
}

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\>]/g, "\\$&");

/** The declarations of every rule whose selector is exactly `selector`, inside `css`, joined. */
const rule = (css: string, selector: string) =>
  [...css.matchAll(new RegExp(`(?:^|[}\\n])\\s*${escape(selector)}\\s*\\{([^}]*)\\}`, "g"))].map((m) => m[1]).join(";");

/** The body of `@keyframes name`. */
const frames = (css: string, name: string) => atBlocks(css, new RegExp(`@keyframes\\s+${name}`));

const reduced = atBlocks(site, /@media\s*\(prefers-reduced-motion:\s*reduce\)/);

describe("between pages", () => {
  it("pages opt in to view transitions", () => {
    expect(site).toMatch(/@view-transition\s*\{\s*navigation:\s*auto;?\s*\}/);
  });

  it("the ruler and nav stay, the page and footer are swapped", () => {
    expect(rule(site, ".ruler")).toContain("view-transition-name: ruler");
    expect(rule(site, ".nav")).toContain("view-transition-name: nav");
    expect(rule(site, "main")).toContain("view-transition-name: page");
    expect(rule(site, ".foot")).toContain("view-transition-name: foot");
  });

  it("the new page is laid down on the mat", () => {
    const inn = rule(site, "::view-transition-new(page), ::view-transition-new(foot)");
    expect(inn).toContain("animation-name: fettle-page-in");
    expect(inn).toContain("280ms");
    expect(inn).not.toContain("animation-delay"); // nothing to wait for: the old page is already gone
    expect(frames(site, "fettle-page-in")).toMatch(/from\s*\{[^}]*opacity:\s*0[^}]*\}/);
    expect(frames(site, "fettle-page-in")).toMatch(/from\s*\{[^}]*translateY\(16px\)/);
  });

  it("the old page goes in one frame, so a scrolled page never jumps back to its top", () => {
    // Its picture would be drawn where the new page starts, i.e. as if scrolled to the top.
    expect(rule(site, "::view-transition-old(page), ::view-transition-old(foot)")).toContain("display: none");
  });

  it("nothing travels from where it was on a scrolled page", () => {
    // A named part would otherwise slide from its old place (thousands of pixels up) to its new one.
    expect(rule(site, "::view-transition-group(*)")).toContain("animation: none");
  });

  it("the nav and ruler are drawn once while the page changes, so they don't flash brighter", () => {
    // With the group's animation off, the old and new pictures have no fade and both stay fully drawn,
    // added together: light parts went white for the length of the switch.
    expect(rule(site, "::view-transition-old(ruler), ::view-transition-old(nav)")).toContain("display: none");
  });

  it("the mat swaps in one frame, so its grid never shows twice", () => {
    expect(rule(site, "::view-transition-old(root)")).toContain("display: none");
    expect(rule(site, "::view-transition-new(root)")).toContain("animation: none");
  });

  it("reduced motion keeps only a fade", () => {
    const inn = rule(reduced, "::view-transition-new(page), ::view-transition-new(foot)");
    expect(inn).toContain("animation-name: fettle-fade-in");
    expect(inn).toContain("150ms");
    const props = [...frames(site, "fettle-fade-in").matchAll(/([a-z-]+)\s*:/g)].map(([, p]) => p);
    expect(props).toEqual(["opacity"]);
  });
});

describe("hero opening", () => {
  it("the headline settles without ever being invisible (it's the largest paint)", () => {
    const h1 = rule(site, ".hero h1");
    expect(h1).toContain("fettle-settle");
    expect(h1).toContain("400ms");
    expect(frames(site, "fettle-settle")).not.toContain("opacity");
    expect(frames(site, "fettle-settle")).toMatch(/from\s*\{[^}]*translateY\(0\.12em\)/);
  });

  it("the knife is set down, then the buttons and small print arrive", () => {
    const knife = rule(site, ".knife");
    expect(knife).toContain("fettle-knife-down");
    expect(knife).toMatch(/450ms[^;]*80ms/);
    expect(rule(site, ".ctas")).toMatch(/fettle-rise[^;]*250ms[^;]*450ms/);
    expect(rule(site, ".fine")).toMatch(/fettle-rise[^;]*250ms[^;]*500ms/);
    expect(frames(site, "fettle-rise")).toMatch(/from\s*\{[^}]*translateY\(6px\)/);
  });

  it("an in-site arrival doesn't replay the hero", () => {
    const off = [...site.matchAll(/([^{}]*html\[data-arrival="page"\][^{}]*)\{\s*animation:\s*none;?\s*\}/g)].map((m) => m[1]).join(",");
    for (const s of [".hero h1", ".knife", ".ctas", ".fine", ".live", ".live dl > div", ".live-tag", ".live-end a::after"]) expect(off, s).toContain(s);
  });

  it("reduced motion switches the hero off too", () => {
    for (const s of [".hero h1", ".knife", ".ctas", ".fine"]) expect(reduced, s).toMatch(new RegExp(escape(s) + String.raw`[^{]*\{\s*animation:\s*none`));
  });
});

describe("theme switch", () => {
  it("the whole page is one sheet while the theme changes", () => {
    expect(rule(site, 'html[data-vt^="theme"] :is(.ruler, .nav, main, .foot)')).toContain("view-transition-name: none");
  });

  it("the new theme is drawn down over the old one", () => {
    expect(rule(site, 'html[data-vt^="theme"]::view-transition-old(root)')).toContain("animation: none");
    const wipe = rule(site, 'html[data-vt="theme"]::view-transition-new(root)');
    expect(wipe).toContain("animation-name: fettle-wipe");
    expect(wipe).toContain("380ms");
    expect(frames(site, "fettle-wipe")).toMatch(/from\s*\{\s*clip-path:\s*inset\(0 0 100% 0\);?\s*\}/);
    // An explicit end: clip-path can't blend into "none", so without it the wipe would flip in one step.
    expect(frames(site, "fettle-wipe")).toMatch(/to\s*\{\s*clip-path:\s*inset\(0\);?\s*\}/);
  });

  it("is switched off for people who ask for less motion", () => {
    expect(rule(reduced, 'html[data-vt="theme"]::view-transition-new(root)')).toContain("animation: none");
  });
});

describe("press feedback", () => {
  it("tapes, ghost buttons and the theme picker press in", () => {
    // Only tapes you can click press in; label tapes (v1.1, the sample's section heads) stay put.
    const tape = rule(site, ":is(a, button).tape:active");
    expect(tape).toContain("translate: 0 1px");
    // Pressing changes nothing else, so the current page's accent underline and the light tape's ring stay as they are.
    expect(tape).not.toContain("box-shadow");
    expect(rule(site, ".tape:active")).toBe("");
    expect(rule(site, ".btn-ghost:active")).toContain("translate: 0 2px");
    expect(rule(site, ".foot-theme label:active")).toContain("translate: 0 1px");
    expect(rule(site, ".tape")).toMatch(/translate 100ms/);
    expect(rule(site, ".btn")).toMatch(/translate 100ms/);
  });

  it("tapes draw no shadow outside their torn edges, where clip-path would hide it", () => {
    for (const sel of [".tape", '.tape[aria-current="page"]', ".tape.light", ".tape.quiet"]) {
      const shadows = /box-shadow:\s*([^;]+)/.exec(rule(site, sel))?.[1] ?? "";
      for (const part of shadows.split(/,(?![^(]*\))/).filter((x) => x.trim())) expect(part.trim(), sel).toMatch(/^inset /);
    }
  });

  it("hover lift is for mouse pointers only", () => {
    // Outside the mouse-pointer block there's no hover lift, so a tap on a phone doesn't leave a tape lifted.
    const pointer = atBlocks(site, /@media\s*\(hover:\s*hover\)\s*and\s*\(pointer:\s*fine\)/);
    for (const s of [".tape:hover", ".btn:hover"]) {
      expect(pointer, s).toMatch(new RegExp(escape(s) + String.raw`\s*\{[^}]*translateY\(-2px\)`));
      // Every nav tape lifts: the hover beats the tapes' own angle rules (.tapes li:nth-child(n) .tape).
      if (s === ".tape:hover") expect(pointer).toMatch(/\.tapes li \.tape:hover/);
      expect(site.replace(/@media\s*\(hover:\s*hover\)\s*and\s*\(pointer:\s*fine\)\s*\{[\s\S]*?\n\}/g, ""), s).not.toMatch(new RegExp(String.raw`(^|[}\n])\s*` + escape(s) + String.raw`\s*\{`));
    }
  });

  it("nothing presses in for people who ask for less motion", () => {
    expect(rule(reduced, ":is(a, button).tape:active, .btn-ghost:active, .foot-theme label:active")).toContain("translate: none");
  });
});

describe("legend and ticks", () => {
  const scroll = atBlocks(site, /@supports\s*\(animation-timeline:\s*view\(\)\)/);

  it("run on their own timelines", () => {
    expect(rule(scroll, ".legend")).toContain("view-timeline-name: --legend");
    expect(rule(scroll, ".checks")).toContain("view-timeline-name: --checks");
    expect(rule(scroll, ".legend .tag")).toContain("animation-timeline: --legend");
    expect(rule(scroll, ".checks li::after")).toContain("animation-timeline: --checks");
  });

  it("the five tags are pressed on one after another", () => {
    for (let n = 1; n <= 5; n++) {
      const [a, b] = [10 + 3 * (n - 1), 22 + 3 * (n - 1)];
      expect(rule(scroll, `.legend dl > div:nth-child(${n}) .tag`), `tag ${n}`).toContain(`animation-range: cover ${a}% cover ${b}%`);
    }
    expect(frames(site, "fettle-tag-press")).toMatch(/from\s*\{[^}]*opacity:\s*0;[^}]*transform:\s*translateY\(-3px\) scale\(1\.06\)/);
  });

  it("the three ticks are drawn, short stroke first", () => {
    for (let n = 1; n <= 3; n++) {
      const [a, b] = [15 + 6 * (n - 1), 27 + 6 * (n - 1)];
      expect(rule(scroll, `.checks li:nth-child(${n})::after`), `tick ${n}`).toContain(`animation-range: cover ${a}% cover ${b}%`);
    }
    expect(frames(site, "fettle-tick-draw")).toMatch(/from\s*\{\s*clip-path:\s*inset\(0 100% 0 0\);?\s*\}/);
    expect(frames(site, "fettle-tick-draw")).toMatch(/to\s*\{\s*clip-path:\s*inset\(0\);?\s*\}/); // clip-path can't blend into "none"
  });

  it("are switched off for people who ask for less motion", () => {
    expect(reduced).toMatch(/\.legend \.tag, \.checks li::after\s*\{\s*animation:\s*none/);
  });

  it("print shows every scroll-linked element at rest", () => {
    const print = atBlocks(site, /@media\s+print/);
    const off = [...print.matchAll(/([^{}]+)\{\s*animation:\s*none;?\s*\}/g)].map((m) => m[1]).join(",");
    for (const s of [".legend .tag", ".checks li::after", ".redact", ".seal", ".sheet-back", ".sheet-mid", ".questions-slip", ".end-slip", ".margin-note", ".callout", ".annotated-receipt :is(.kv > div, .steps li, .r-section h3, .line)"]) {
      expect(off, s).toContain(s);
    }
  });
});

describe("FAQ answers", () => {
  const sized = atBlocks(site, /@supports\s*\(interpolate-size:\s*allow-keywords\)/);

  it("ease open and shut where the browser can animate to auto", () => {
    expect(rule(sized, ":root")).toContain("interpolate-size: allow-keywords");
    const content = rule(sized, ".faq-item::details-content");
    expect(content).toContain("block-size: 0");
    expect(content).toContain("overflow: clip");
    expect(content).toMatch(/block-size 220ms/);
    expect(content).toMatch(/content-visibility 220ms allow-discrete/);
    expect(rule(sized, ".faq-item[open]::details-content")).toContain("block-size: auto");
  });

  it("the answer's words come in as it opens, only where the answer itself eases open", () => {
    const answer = rule(sized, ".faq-answer");
    expect(atBlocks(site, /@starting-style/)).toBe(atBlocks(sized, /@starting-style/));
    expect(answer).toMatch(/opacity 180ms/);
    expect(answer).toMatch(/translate 180ms/);
    const start = rule(atBlocks(site, /@starting-style/), ".faq-item[open] .faq-answer");
    expect(start).toContain("opacity: 0");
    expect(start).toContain("translate: 0 4px");
  });

  it("snap open as before for people who ask for less motion", () => {
    expect(rule(reduced, ".faq-item::details-content, .faq-answer")).toContain("transition: none");
  });
});
