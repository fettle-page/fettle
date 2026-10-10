import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// The download count: GitHub's own total for the installer and the zip, filled in by the Worker
// (worker/src/downloads.ts) on a strip of label tape stuck to the main download button. The built
// pages carry the tape empty and hidden, with its wording, so a page served without the Worker, or
// when GitHub doesn't answer, shows nothing.
const tape = (html: string) => /<span class="dl-tape"[^>]*>([^<]*)<\/span>/.exec(html);

describe("the download count's place on the site", () => {
  for (const [file, href, one, many] of [
    ["index.html", "/download", "1 download", "{n} downloads"],
    ["it.html", "/it/download", "1 download", "{n} download"],
    ["download.html", "https://github.com/", "1 download", "{n} downloads"],
    ["it/download.html", "https://github.com/", "1 download", "{n} download"],
  ] as const) {
    it(`${file}: an empty, hidden tape on the download button, with the wording for the Worker`, () => {
      const html = page(file);
      const m = tape(html);
      expect(m, file).not.toBeNull();
      const tag = m![0];
      expect(tag).toContain("data-download-count");
      expect(tag).toContain(`data-one="${one}"`);
      expect(tag).toContain(`data-many="${many}"`);
      expect(tag).toMatch(/\shidden[\s>]/);
      expect(m![1]).toBe("");
      // Stuck to the main download button, right after it.
      const button = `<span class="dl-cta"><a class="btn btn-primary" href="${href}`;
      const at = html.indexOf(button);
      expect(at, `${file}: the button sits in a .dl-cta`).toBeGreaterThan(-1);
      expect(html.slice(at).search(/<span class="dl-tape"/)).toBeLessThan(html.slice(at).indexOf("</span>") + 1 + 400);
    });
  }

  it("is only on the home and download pages, and the old small line is gone", () => {
    for (const file of ["faq.html", "privacy.html", "changelog.html", "404.html"]) expect(tape(page(file)), file).toBeNull();
    for (const file of ["index.html", "download.html"]) expect(page(file), file).not.toContain('<p class="dl-count"');
  });
});
