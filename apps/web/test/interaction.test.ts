import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// Website interaction: the page explains itself under the hand.
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

/** The declarations of every rule whose selector list contains `selector`, inside `css`, joined. */
const ruleWith = (css: string, selector: string) =>
  [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].filter(([, sel]) => sel!.split(",").map((s) => s.trim()).includes(selector)).map(([, , body]) => body).join(";");

const reduced = atBlocks(site, /@media\s*\(prefers-reduced-motion:\s*reduce\)/);
const LEVELS = ["fine", "info", "keep_an_eye", "act_soon", "couldnt_check"];

describe("self-healing cuts", () => {
  it("draw on a layer over the mat, under the content", () => {
    const layer = rule(site, ".cuts");
    for (const d of ["position: absolute", "inset: 0", "pointer-events: none", "z-index: 1"]) expect(layer, d).toContain(d);
    expect(rule(site, ".mat")).toContain("position: relative");
    expect(page("index.html")).toMatch(/<div class="mat"><svg class="cuts" aria-hidden="true" focusable="false"><\/svg>/);
  });

  it("heal by thinning and fading over 1.5 s", () => {
    expect(rule(site, ".cut path")).toMatch(/transition:[^;]*stroke-width 1500ms ease-in/);
    expect(rule(site, ".cut path")).toMatch(/transition:[^;]*opacity 1500ms ease-in/);
    const healing = rule(site, ".cut.healing path");
    expect(healing).toContain("stroke-width: 0");
    expect(healing).toContain("opacity: 0");
  });

  it("are hidden for people who ask for less motion", () => {
    expect(rule(reduced, ".cuts")).toContain("display: none");
  });
});

describe("two new moments on scroll", () => {
  const scroll = atBlocks(site, /@supports\s*\(animation-timeline:\s*view\(\)\)/);
  const frames = (name: string) => atBlocks(site, new RegExp(`@keyframes\\s+${name}`));

  it("the privacy slip feeds out of a slot", () => {
    const slip = rule(scroll, ".privacy-slip");
    expect(slip).toContain("animation-name: fettle-slot-feed");
    expect(slip).toContain("animation-timeline: --privacy-slip");
    expect(slip).toContain("animation-range: entry 20% cover 30%");
    // The paper slides its whole height while the clip's top edge moves with it, so the edge you see
    // stays on the slot line (6px above the slip): paper leaving a printer, not a slide plus a reveal.
    expect(frames("fettle-slot-feed")).toMatch(/from\s*\{[^}]*clip-path:\s*inset\(calc\(100% - 6px\) -40px -60px -40px\);[^}]*transform:\s*translateY\(-100%\) rotate\(-1\.2deg\)/);
    expect(frames("fettle-slot-feed")).toMatch(/to\s*\{[^}]*clip-path:\s*inset\(-6px -40px -60px -40px\);[^}]*transform:\s*rotate\(-1\.2deg\)/);
    expect(page("index.html")).toMatch(/<div class="slip-feed"><span class="slip-slot" aria-hidden="true"><\/span><figure class="slip privacy-slip"/);
    expect(rule(site, ".slip-slot")).toContain("background: var(--t-tape)");
  });

  it("the download slip comes off the roll from its top corner", () => {
    expect(rule(scroll, ".end-slip")).toContain("animation-name: fettle-tear-off");
    expect(rule(site, ".end-slip")).toContain("transform-origin: 100% 0");
    expect(frames("fettle-tear-off")).toMatch(/from\s*\{[^}]*opacity:\s*0;[^}]*transform:\s*translateY\(-28px\) rotate\(calc\(var\(--tilt\) - 3deg\)\)/);
  });

  it("both rest when printed and stay still for people who ask for less motion", () => {
    const print = atBlocks(site, /@media\s+print/);
    expect([...print.matchAll(/([^{}]+)\{\s*animation:\s*none;?\s*\}/g)].map((m) => m[1]).join(",")).toContain(".privacy-slip");
    expect(ruleWith(reduced, ".privacy-slip")).toContain("animation: none");
    expect(ruleWith(reduced, ".end-slip")).toContain("animation: none");
  });
});

describe("under the hand", () => {
  const pointer = atBlocks(site, /@media\s*\(hover:\s*hover\)\s*and\s*\(pointer:\s*fine\)/);
  const LINKS = [".text-link", ".more-all", ".foot-links a"];

  it("paper lifts and its shadow deepens when you point at it", () => {
    expect(ruleWith(pointer, ".live:hover")).toContain("translate: 0 -3px");
    expect(ruleWith(pointer, ".live:hover")).toContain("drop-shadow(0 28px 26px");
    expect(ruleWith(pointer, ".slip:hover")).toContain("translate: 0 -3px");
    expect(ruleWith(pointer, ".slip:hover")).toContain("drop-shadow(0 26px 24px");
    // The front sheet is inert (the browser never sees the pointer over it), so its stack carries the hover.
    expect(ruleWith(pointer, ".tech-stack:hover .sheet-front")).toContain("translate: 0 -3px");
    expect(ruleWith(pointer, ".tech-stack:hover .sheet-front")).toContain("box-shadow: 0 30px 46px -18px");
    expect(ruleWith(pointer, ".sheet-front:hover")).toBe("");
    for (const s of [".live", ".slip", ".sheet-front"]) expect(ruleWith(site, s), s).toMatch(/transition:[^;]*translate 200ms/);
  });

  it("links get the pill band, which sweeps in from the left", () => {
    expect(ruleWith(pointer, ".more-slip li a:hover")).toContain("text-decoration: none");
    expect(ruleWith(pointer, ".more-all:hover")).toContain("text-decoration: none");
    for (const s of LINKS) {
      expect(ruleWith(site, s), s).toContain("--band-inset: 55% -4px -1px");
      const band = ruleWith(site, `${s}::before`);
      expect(band, s).toContain("transform: scaleX(0)");
      expect(band, s).toContain("transform-origin: left");
      expect(band, s).toMatch(/transition:[^;]*transform 220ms/);
      expect(ruleWith(pointer, `${s}:hover::before`), s).toContain("transform: scaleX(1)");
    }
    expect(ruleWith(site, ".text-link")).toContain("--band-color: color-mix(in srgb, var(--t-accent) 35%, transparent)");
  });

  it("a link that wraps gets one pill per line, never a block", () => {
    // "More questions" links can wrap (Italian, mid widths): an inline background, cloned on each line.
    const a = rule(site, ".more-slip li a");
    for (const d of ["display: inline", "box-decoration-break: clone", "-webkit-box-decoration-break: clone", "border-radius: 999px", "background-size: 0% 100%"]) expect(a, d).toContain(d);
    expect(a).toMatch(/background-image: linear-gradient\(var\(--color-soft\), var\(--color-soft\)\)/);
    expect(a).toMatch(/transition:[^;]*background-size 220ms/);
    expect(ruleWith(pointer, ".more-slip li a:hover")).toContain("background-size: 100% 100%");
    expect(rule(site, ".more-slip li")).toContain("padding: 8px 0");
    // The whole row is still the link: its ::before is free now, and stretches over the row.
    expect(rule(site, ".more-slip li")).toContain("position: relative");
    const row = rule(site, ".more-slip li a::before");
    for (const d of ['content: ""', "position: absolute", "inset: 0"]) expect(row, d).toContain(d);
    expect(ruleWith(reduced, ".more-slip li a")).toContain("transition: none");
  });

  it("hover lifts and sweeps stay off for people who ask for less motion", () => {
    expect(ruleWith(reduced, ".live:hover")).toContain("translate: none");
    expect(ruleWith(reduced, ".tech-stack:hover .sheet-front")).toContain("translate: none");
    for (const s of [".live", ".slip", ".sheet-front"]) expect(ruleWith(reduced, s), s).toContain("transition: none");
    for (const s of LINKS) expect(ruleWith(reduced, `${s}::before`), s).toContain("transition: none");
  });
});

describe("the text selection", () => {
  // Rounded ends were tried and taken out: they looked worse than the plain green band, which stays.
  it("is the plain green band, with no ends drawn over it", () => {
    for (const s of [".marks", ".mark-cap"]) expect(site, s).not.toContain(s);
    for (const f of ["index.html", "it.html", "faq.html"]) expect(page(f), f).not.toContain('class="marks"');
    expect(existsSync(new URL("../src/scripts/selection.ts", import.meta.url))).toBe(false);
  });
});

describe("printing", () => {
  const print = atBlocks(site, /@media\s+print/);
  it("shows the sample receipt plain, whatever is pinned or pointed at", () => {
    expect(rule(print, "#how[data-show] .annotated-receipt .line")).toContain("opacity: 1");
    expect(ruleWith(print, ".annotated-receipt .line::before")).toContain("display: none");
    expect(ruleWith(print, ".annotated-receipt .is-pointed::before")).toContain("display: none");
    expect(ruleWith(print, ".slip-slot")).toContain("display: none");
  });
});

describe("the footer theme spreads from the swatch", () => {
  it("is a circle that grows from the swatch to the farthest corner", () => {
    const spot = rule(site, 'html[data-vt="theme-spot"]::view-transition-new(root)');
    expect(spot).toContain("animation-name: fettle-spread");
    expect(spot).toContain("450ms");
    const frames = atBlocks(site, /@keyframes\s+fettle-spread/);
    expect(frames).toMatch(/from\s*\{\s*clip-path:\s*circle\(0 at var\(--vt-x\) var\(--vt-y\)\);?\s*\}/);
    expect(frames).toMatch(/to\s*\{\s*clip-path:\s*circle\(var\(--vt-r\) at var\(--vt-x\) var\(--vt-y\)\);?\s*\}/);
  });

  it("both theme changes make the page one sheet and keep the old theme underneath", () => {
    expect(rule(site, 'html[data-vt^="theme"] :is(.ruler, .nav, main, .foot)')).toContain("view-transition-name: none");
    expect(rule(site, 'html[data-vt^="theme"]::view-transition-old(root)')).toContain("display: block");
  });

  it("is switched off for people who ask for less motion", () => {
    expect(rule(reduced, 'html[data-vt="theme-spot"]::view-transition-new(root)')).toContain("animation: none");
  });
});

describe("the legend explains the receipt", () => {
  for (const [file, show, none] of [
    ["index.html", "Show these lines on the sample receipt", "Not on this sample."],
    ["it.html", "Mostra queste righe sullo scontrino di esempio", "Non c&#39;è in questo esempio."],
  ] as const) {
    it(`${file}: each tag is a button that controls the sample`, () => {
      const html = page(file);
      const picks = [...html.matchAll(/<button[^>]*class="legend-pick"[^>]*>/g)].map((m) => m[0]);
      expect(picks).toHaveLength(5);
      for (const [i, b] of picks.entries()) {
        expect(b).toContain(`data-level="${LEVELS[i]}"`);
        expect(b).toContain('aria-pressed="false"');
        expect(b).toContain('aria-controls="sample"');
        expect(b).toContain('type="button"');
        // Without JavaScript they do nothing, so they start switched off; the legend script switches them on.
        expect(b).toMatch(/\sdisabled[\s>]/);
      }
      expect(html.split(show).length - 1).toBe(5);
      expect([...html.matchAll(/<span class="legend-none" hidden>([^<]*)<\/span>/g)].map((m) => m[1])).toEqual(Array(5).fill(none));
      // Read after the level name as its own sentence; a dash can be read out as "dash".
      // Hidden until the script runs: a button that can't show anything shouldn't offer to.
      expect(html.split(`<span class="sr-only" data-legend-show hidden>. ${show}</span>`).length - 1).toBe(5);
      expect(html).not.toContain(` — ${show}`);
    });
  }

  it("other lines fade while a level is shown", () => {
    const fade = rule(site, "#how[data-show] .annotated-receipt .line");
    expect(fade).toContain("opacity: 0.3");
    expect(rule(site, ".annotated-receipt .line")).toMatch(/transition:[^;]*opacity 160ms/);
  });

  it("matching lines get the pill band", () => {
    const band = ruleWith(site, ".annotated-receipt .line::before");
    expect(site).not.toMatch(/\.band\b/); // one shared band, no unused class
    expect(band).toContain("border-radius: var(--band-radius, 999px)");
    // A pill on one line; on a several-line receipt line a full pill becomes a blob, so a 1em radius.
    expect(rule(site, ".annotated-receipt .line")).toContain("--band-radius: 1em");
    expect(band).toContain("inset: var(--band-inset, -2px -8px)");
    expect(band).toContain("background: var(--band-color, var(--color-soft))");
    expect(band).toContain("opacity: 0");
    expect(band).toMatch(/transition:[^;]*opacity 160ms/);
    for (const l of LEVELS) {
      expect(ruleWith(site, `#how[data-show="${l}"] .annotated-receipt .line[data-level="${l}"]`), l).toContain("opacity: 1");
      expect(ruleWith(site, `#how[data-show="${l}"] .annotated-receipt .line[data-level="${l}"]::before`), l).toContain("opacity: var(--band-opacity, 1)");
    }
  });

  it("the pinned tag is outlined", () => {
    // Pinned and focused look different: a pin is a solid ring with a soft-green halo, focus a dashed ring.
    const pinned = ruleWith(site, '.legend-pick[aria-pressed="true"] .tag');
    expect(pinned).toMatch(/outline: 2px solid/);
    expect(pinned).toContain("box-shadow: 0 0 0 4px var(--color-soft)");
    expect(ruleWith(site, ".legend-pick:focus-visible .tag")).toMatch(/outline: 2px dashed/);
    // A pinned tag that gets keyboard focus shows it too: its ring turns dashed.
    expect(ruleWith(site, '.legend-pick[aria-pressed="true"]:focus-visible .tag')).toContain("outline-style: dashed");
  });

  it("pointing marks a receipt part with a lighter band, and its note and callout step forward", () => {
    for (const s of [".annotated-receipt [data-section]", ".annotated-receipt .r-head", ".annotated-receipt .r-summary h2", ".annotated-receipt .fold"]) {
      expect(ruleWith(site, s), s).toContain("position: relative");
    }
    expect(ruleWith(site, ".annotated-receipt .is-pointed")).toContain("--band-opacity: 0.55");
    expect(ruleWith(site, ".annotated-receipt .is-pointed::before")).toContain("opacity: var(--band-opacity, 1)");
    expect(rule(site, ".margin-note.is-pointed")).toContain("color: var(--on-mat)");
    expect(rule(site, ".callout.is-pointed .tape")).toContain("translate: 0 -1px");
    expect(rule(site, ".margin-note")).toMatch(/transition:[^;]*color 160ms/);
  });

  it("fades and bands switch instantly for people who ask for less motion", () => {
    expect(ruleWith(reduced, ".margin-note")).toContain("transition: none");
    expect(ruleWith(reduced, ".annotated-receipt .line")).toContain("transition: none");
    expect(ruleWith(reduced, ".annotated-receipt .line::before")).toContain("transition: none");
  });
});
