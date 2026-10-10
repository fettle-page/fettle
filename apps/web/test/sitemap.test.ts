import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST } from "./dist-helpers.ts";

// Search Console and other tools look for /sitemap.xml first; the sitemap integration writes
// /sitemap-index.xml. The alias must stay the same file, byte for byte.
describe("/sitemap.xml", () => {
  const read = (name: string) => readFileSync(join(DIST, name));

  it("is in the built site, identical to sitemap-index.xml", () => {
    expect(existsSync(join(DIST, "sitemap.xml"))).toBe(true);
    expect(read("sitemap.xml").equals(read("sitemap-index.xml"))).toBe(true);
  });

  it("points at the page list, and isn't listed as a page itself", () => {
    expect(read("sitemap.xml").toString("utf8")).toContain("<loc>https://fettle.page/sitemap-0.xml</loc>");
    expect(read("sitemap-0.xml").toString("utf8")).not.toMatch(/sitemap\.xml/);
  });

  it("leaves robots.txt naming one sitemap", () => {
    const lines = read("robots.txt").toString("utf8").split(/\r?\n/).filter((l) => /^sitemap:/i.test(l));
    expect(lines).toEqual(["Sitemap: https://fettle.page/sitemap-index.xml"]);
  });
});
