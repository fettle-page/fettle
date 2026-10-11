import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { page } from "./dist-helpers.ts";

// Each question is a heading inside its <summary>, so screen readers can jump from one to the next.
describe("the FAQ page", () => {
  for (const [locale, file] of [["en", "faq.html"], ["it", "it/faq.html"]] as const) {
    it(`${locale}: makes every question a heading under the page's h1`, () => {
      const html = page(file);
      const headings = [...html.matchAll(/<summary>\s*<h2 class="faq-q">([^<]+)<\/h2>\s*<\/summary>/g)].map((m) => m[1]);
      expect(headings).toHaveLength(faqs(locale).length);
      expect(html.match(/<h1[\s>]/g)).toHaveLength(1);
    });
  }
});
