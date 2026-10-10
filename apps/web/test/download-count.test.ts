import { describe, expect, it } from "vitest";
import { page } from "./dist-helpers.ts";

// The download count: GitHub's own total for the installer and the zip, filled in by the Worker
// (worker/src/downloads.ts). The built pages carry an empty, hidden line with the wording, so a page
// served without the Worker, or when GitHub doesn't answer, shows nothing.
const placeholder = (html: string) => /<p class="dl-count"[^>]*>([^<]*)<\/p>/.exec(html);

describe("the download count's place on the site", () => {
  for (const [file, one, many] of [
    ["download.html", "Downloaded once", "Downloaded {n} times"],
    ["it/download.html", "Scaricato una volta", "Scaricato {n} volte"],
  ] as const) {
    it(`${file}: an empty, hidden line with the wording for the Worker`, () => {
      const m = placeholder(page(file));
      expect(m, file).not.toBeNull();
      const tag = m![0];
      expect(tag).toContain("data-download-count");
      expect(tag).toContain(`data-one="${one}"`);
      expect(tag).toContain(`data-many="${many}"`);
      expect(tag).toMatch(/\shidden[\s>]/);
      expect(m![1]).toBe("");
    });
  }

  it("is only on the download page", () => {
    for (const file of ["index.html", "it.html", "faq.html", "privacy.html", "changelog.html", "404.html"]) expect(placeholder(page(file)), file).toBeNull();
  });
});
