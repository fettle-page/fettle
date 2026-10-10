import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// The download count: GitHub's own total for the installer and the zip, filled in by the Worker
// (worker/src/downloads.ts) as a section of the main download button, like a star count on a button.
// The built pages carry it empty and hidden, with its wording, so a page served without the Worker,
// or when GitHub doesn't answer, shows the plain button.
const button = (html: string) => /<a class="btn btn-primary dl-btn" href="([^"]*)"[^>]*>([\s\S]*?)<\/a>/.exec(html);

describe("the download count's place on the site", () => {
  for (const [file, href, label, one, many] of [
    ["index.html", "/download", "Download for Windows", "1 download", "{n} downloads"],
    ["it.html", "/it/download", "Scarica per Windows", "1 download", "{n} download"],
    ["download.html", "https://github.com/", "Download the installer", "1 download", "{n} downloads"],
    ["it/download.html", "https://github.com/", "Scarica l&#39;installer", "1 download", "{n} download"],
  ] as const) {
    it(`${file}: the main download button has an empty, hidden count section`, () => {
      const m = button(page(file));
      expect(m, file).not.toBeNull();
      expect(m![1]).toContain(href);
      const inner = m![2]!;
      expect(inner).toContain(`<span class="dl-label">${label}</span>`);
      const count = /<span class="dl-count"([^>]*)>([\s\S]*)<\/span>$/.exec(inner);
      expect(count, file).not.toBeNull();
      const [, attrs, body] = count!;
      expect(attrs).toContain("data-download-count");
      expect(attrs).toContain(`data-one="${one}"`);
      expect(attrs).toContain(`data-many="${many}"`);
      expect(attrs).toMatch(/\shidden(\s|$)/);
      expect(body).toMatch(/<svg[^>]*aria-hidden="true"/);
      expect(body).toMatch(/<span data-download-n aria-hidden="true"><\/span>/);
      expect(body).toMatch(/<span class="sr-only" data-download-said><\/span>/);
    });
  }

  it("is only on the home and download pages, and the tape and the small line are gone", () => {
    for (const file of ["faq.html", "privacy.html", "changelog.html", "404.html"]) expect(button(page(file)), file).toBeNull();
    for (const file of ["index.html", "download.html"]) {
      expect(page(file), file).not.toMatch(/class="dl-tape"|class="dl-cta"|<p class="dl-count"/);
    }
  });
});
