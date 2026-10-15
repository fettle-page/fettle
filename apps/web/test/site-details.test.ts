import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page, pages } from "./dist-helpers.ts";

const src = (rel: string) => readFileSync(new URL(`../${rel}`, import.meta.url), "utf8");
const titleOf = (html: string) => html.match(/<title>([^<]*)<\/title>/)?.[1] ?? "";

describe("tab titles", () => {
  it("use an em dash between the name and the rest, never a middle dot", () => {
    expect(titleOf(page("index.html"))).toBe("Fettle — a free check-up for Windows PCs");
    expect(titleOf(page("it.html"))).toBe("Fettle — il controllo gratuito per PC Windows");
    expect(titleOf(page("faq.html"))).toBe("Questions — Fettle");
    expect(titleOf(page("it/faq.html"))).toBe("Domande — Fettle");
    for (const [file, html] of pages()) expect(titleOf(html), file).not.toContain("·");
  });
});

describe("selected text", () => {
  it("is marked with the brand's soft green highlighter on the site and the share page", () => {
    for (const file of ["src/styles/site.css", "src/share/share.css"]) {
      expect(src(file), file).toMatch(/::selection\s*\{[^}]*background:\s*var\(--color-soft\)[^}]*color:\s*var\(--color-ink\)/);
    }
  });
});

describe("footer theme picker", () => {
  it("shows each theme as a swatch on a tape strip, with the radio kept for keyboards and screen readers", () => {
    const html = page("index.html");
    const picker = html.slice(html.indexOf("data-theme-picker"), html.indexOf("</fieldset>"));
    for (const v of ["auto", "light", "dark", "oled"]) {
      expect(picker, v).toMatch(new RegExp(`<input[^>]*class="sr-only"[^>]*value="${v}"|<input[^>]*value="${v}"[^>]*class="sr-only"`));
      expect(picker, v).toContain(`class="swatch swatch-${v}"`);
    }
    const css = src("src/styles/site.css");
    expect(css).not.toMatch(/\.foot-theme input \{[^}]*accent-color/);
    expect(css).toMatch(/\.foot-theme label:has\(input:checked\)/);
    expect(css).toMatch(/\.foot-theme label:has\(input:focus-visible\)/);
  });
});

describe("keyboard focus on label-maker tapes", () => {
  // A tape's torn edges are a clip-path, which also cuts off the focus outline drawn outside it, so a
  // focused tape drops the torn edges and shows the ring (WCAG 2.4.7).
  const body = (css: string, selector: string) =>
    [...css.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^}]*)\}/g)].filter(([, sel]) => sel!.split(/,(?![^(]*\))/).some((s) => s.trim() === selector)).map(([, , b]) => b).join(";");
  it("shows the focus ring on the site's tapes", () => {
    expect(body(src("src/styles/site.css"), ":is(a, button).tape:focus-visible")).toMatch(/clip-path:\s*none/);
  });
  it("shows the focus ring on the share page's tapes", () => {
    const rule = body(src("src/share/share.css"), ".tape:focus-visible");
    expect(rule).toMatch(/clip-path:\s*none/);
    expect(rule).toMatch(/outline:\s*2px solid/);
  });
});

describe("the privacy slip", () => {
  // Each row's word agrees with its label in Italian: "MAC e IP" are plural.
  const rows = (file: string) => {
    const slip = /<figure class="slip privacy-slip"[^>]*>([\s\S]*?)<\/figure>/.exec(page(file))![1]!;
    return [...slip.matchAll(/<dt>([^<]*)<\/dt>\s*<dd>[\s\S]*?<span>([^<]*)<\/span>\s*<\/dd>/g)].map(([, label, word]) => [label!, word!]);
  };
  it("says hidden in the right number, in each language", () => {
    const it_ = rows("it.html");
    expect(it_.find(([label]) => label === "MAC e IP")?.[1]).toBe("nascosti");
    for (const [label, word] of it_.filter(([label]) => label !== "MAC e IP")) expect(word, label).toBe("nascosto");
    expect(rows("it.html").length).toBe(4);
    for (const [label, word] of rows("index.html")) expect(word, label).toBe("hidden");
  });
});
