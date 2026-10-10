import { describe, expect, it } from "vitest";
import { pages } from "../src/i18n/pages.ts";
import { page } from "./dist-helpers.ts";

// The 404: no empty card, but a misprinted receipt with the ways on. One page
// serves every missing address, so it carries both languages; an /it/ address shows the Italian.
const html = page("404.html");
const section = (locale: string) => new RegExp(`<section[^>]*data-nf="${locale}"[^>]*>([\\s\\S]*?)</section>`).exec(html);

describe("the 404 page", () => {
  it("has no empty card any more", () => {
    // An element with the card's class (the inlined stylesheet names it on every page).
    expect(html).not.toMatch(/class="([^"]* )?doc-sheet[ "]/);
  });

  for (const locale of ["en", "it"] as const) {
    it(`${locale}: a receipt slip with the ?? tag and the ways on`, () => {
      const c = pages(locale).notFound;
      const s = section(locale);
      expect(s, locale).not.toBeNull();
      const body = s![1]!;
      expect(body).toContain(c.h1.replace("'", "&#39;"));
      expect(body).toMatch(/class="tag tag-couldnt_check"[^>]*>\?\?</);
      expect(c.links.map((l) => l.href)).toEqual(locale === "en" ? ["/", "/download", "/faq", "/changelog"] : ["/it", "/it/download", "/it/faq", "/it/changelog"]);
      for (const link of c.links) expect(body, link.href).toContain(`href="${link.href}"`);
    });
  }

  it("shows English without JavaScript, the Italian waiting hidden", () => {
    expect(section("en")![0]).not.toMatch(/^<section[^>]*\shidden/);
    expect(section("it")![0]).toMatch(/^<section[^>]*\shidden/);
    expect(section("it")![0]).toContain('lang="it"');
  });
});
