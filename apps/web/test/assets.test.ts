import { existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST, pages } from "./dist-helpers.ts";

describe("preview images and icons", () => {
  it("ship with the site", () => {
    for (const file of ["og/site.png", "og/share.png", "apple-touch-icon.png", "favicon.svg", "robots.txt", "_headers", "sitemap-index.xml"]) {
      expect(existsSync(join(DIST, file)), file).toBe(true);
    }
  });

  it("are referenced by every public page", () => {
    for (const [file, html] of pages()) {
      if (file === "share.html" || file === "404.html") continue;
      expect(html, file).toContain('<meta property="og:image" content="https://fettle.page/og/site.png">');
    }
  });
});

describe("the share shell", () => {
  it("keeps the Worker's contract: noindex, one empty payload slot, one module script", async () => {
    const { page } = await import("./dist-helpers.ts");
    const html = page("share.html");
    expect(html).toContain('<meta name="robots" content="noindex">');
    expect(html.match(/id="fettle-share"/g)).toHaveLength(1);
    expect(html).toMatch(/<script id="fettle-share" type="application\/json"><\/script>/);
    expect(html.match(/<script type="module" src="[^"]+">/g)).toHaveLength(1);
    expect(html).not.toContain("class=\"nav\"");
  });
});

describe("theme switching", () => {
  it("every page applies a stored theme before its stylesheet, and offers the switch only with JavaScript", async () => {
    const { pages } = await import("./dist-helpers.ts");
    for (const [file, html] of pages()) {
      const script = html.search(/<script>[^<]*fettle-theme/);
      expect(script, file).toBeGreaterThan(0);
      expect(script, file).toBeLessThan(html.indexOf("<style>"));
      expect(html.slice(0, html.indexOf("</head>")), file).toContain('<meta name="theme-color" content="#E8EDE7" media="(prefers-color-scheme: light)">');
      if (file === "share.html") continue;
      expect(html, file).toMatch(/<button[^>]*data-theme-toggle[^>]*hidden/);
      expect(html, file).toMatch(/<fieldset[^>]*data-theme-picker[^>]*hidden/);
    }
  });
});
