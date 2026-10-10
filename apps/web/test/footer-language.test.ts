import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// The footer's language choice sits with the theme choice, as a pair of options with the current one
// marked, instead of trailing the page links (where "Italiano" wrapped onto a line of its own).
const block = (html: string, cls: string) => new RegExp(`<[^>]+class="${cls}"[^>]*>([\\s\\S]*?)</(nav|div)>`).exec(html)?.[1] ?? "";

describe("the footer's language choice", () => {
  for (const [file, locale, en, it_] of [
    ["index.html", "en", "/", "/it"],
    ["it.html", "it", "/", "/it"],
    ["faq.html", "en", "/faq", "/it/faq"],
    ["it/download.html", "it", "/download", "/it/download"],
  ] as const) {
    it(`${file}: English and Italiano, the current one marked, each pointing at this page`, () => {
      const html = page(file);
      const lang = block(html, "foot-lang");
      expect(lang, file).toContain(locale === "en" ? ">Language<" : ">Lingua<");
      expect(lang).toMatch(new RegExp(`<a href="${en}" hreflang="en" lang="en"${locale === "en" ? ' aria-current="true"' : ""}>English</a>`));
      expect(lang).toMatch(new RegExp(`<a href="${it_}" hreflang="it" lang="it"${locale === "it" ? ' aria-current="true"' : ""}>Italiano</a>`));
    });

    it(`${file}: the page links no longer end with a language`, () => {
      const links = block(page(file), "foot-links");
      expect(links).not.toBe("");
      expect(links).not.toContain("hreflang");
    });
  }

  it("sits in one row with the theme choice", () => {
    const html = page("index.html");
    expect(html).toMatch(/<div class="foot-prefs">\s*<fieldset class="foot-theme"[\s\S]*?<\/fieldset>\s*<nav class="foot-lang"/);
  });
});
