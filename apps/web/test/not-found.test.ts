import { describe, expect, it } from "vitest";
import { pages } from "../src/i18n/pages.ts";
import { t } from "../src/i18n/strings.ts";
import { page } from "./dist-helpers.ts";

// The 404: no empty card, but a misprinted receipt with the ways on. Each language has its own page
// (404.html, it/404.html); Cloudflare serves the nearest one, so a missing /it/ address gets the
// Italian page with the Italian nav and footer.
const files = { en: "404.html", it: "it/404.html" } as const;

describe("the 404 pages", () => {
  for (const locale of ["en", "it"] as const) {
    const html = page(files[locale]);

    it(`${locale}: a receipt slip with the ?? tag and the ways on`, () => {
      const c = pages(locale).notFound;
      expect(html).toContain(c.h1.replace("'", "&#39;"));
      expect(html).toMatch(/class="tag tag-couldnt_check"[^>]*>\?\?</);
      expect(c.links.map((l) => l.href)).toEqual(locale === "en" ? ["/", "/download", "/faq", "/changelog"] : ["/it", "/it/download", "/it/faq", "/it/changelog"]);
      for (const link of c.links) expect(html, link.href).toContain(`href="${link.href}"`);
    });

    it(`${locale}: the page, its nav and its footer are all in that language`, () => {
      expect(html).toMatch(new RegExp(`<html[^>]*lang="${locale}"`));
      expect(html).toContain(`aria-label="${t(locale, "nav.label")}"`);
      expect(html).toContain(`>${t(locale, "nav.download")}<`);
      expect(html).toContain(`<title>${pages(locale).notFound.title.replace("'", "&#39;")}</title>`);
      // One language per page: no hidden copy of the other, and no script swapping them.
      expect(html).not.toContain("data-nf");
    });

    it(`${locale}: marks no nav link as the current page`, () => {
      expect(html).not.toContain('aria-current="page"');
    });
  }

  it("has no empty card any more", () => {
    // An element with the card's class (the inlined stylesheet names it on every page).
    expect(page("404.html")).not.toMatch(/class="([^"]* )?doc-sheet[ "]/);
  });
});
