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
