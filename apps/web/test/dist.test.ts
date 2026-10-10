import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { DIST, pages } from "./dist-helpers.ts";

const THEME_JS = readFileSync(join(DIST, "theme.js"), "utf8").trim();

const SITE = "https://fettle.page";
// Pages that are not part of the public site map (the share shell and the 404).
const UTILITY = new Set(["share.html", "404.html"]);

describe("every built page", () => {
  const all = pages();

  it("exists", () => {
    expect(all.map(([f]) => f)).toEqual(expect.arrayContaining(["index.html", "it.html"]));
  });

  for (const [file, html] of all) {
    describe(file, () => {
      it("declares its language, title and description", () => {
        expect(html).toMatch(/<html lang="(en|it)"/);
        expect(html).toMatch(/<title>[^<]{3,}<\/title>/);
        if (!UTILITY.has(file)) expect(html).toMatch(/<meta name="description" content="[^"]{20,}"/);
      });

      if (!UTILITY.has(file)) {
        it("has a canonical URL and both language alternates", () => {
          expect(html).toMatch(new RegExp(`<link rel="canonical" href="${SITE}/[^"]*"`));
          for (const lang of ["en", "it", "x-default"]) expect(html).toMatch(new RegExp(`<link rel="alternate" hreflang="${lang}" href="${SITE}/[^"]*"`));
        });
      }

      it("has no inline script or style, so the strict CSP holds", () => {
        // Inline scripts: JSON data blocks, plus the one early theme script (allowed by hash, see inline-css.test.ts).
        const inlineScripts = [...html.matchAll(/<script(?![^>]*\bsrc=)([^>]*)>([\s\S]*?)<\/script>/g)]
          .filter(([, attrs]) => !/type="application\/(ld\+)?json"/.test(attrs!))
          .filter(([, , body]) => body!.trim() !== THEME_JS);
        expect(inlineScripts.map(([tag]) => tag.slice(0, 60))).toEqual([]);
        // <style> blocks are allowed only by hash (see inline-css.test.ts); style attributes never.
        expect(html).not.toMatch(/\sstyle="/);
      });

      it("loads nothing from other sites", () => {
        const urls = [...html.matchAll(/\s(?:src|href)="(https?:\/\/[^"]+)"/g)].map(([, u]) => u!);
        // A plain <a> link (the GitHub repo, the downloads) loads nothing until the visitor follows it.
        const allowed = (u: string, tag: string) => tag.startsWith("<a ") || (u.startsWith(SITE) && /rel="(canonical|alternate)"/.test(tag));
        const tags = [...html.matchAll(/<[^>]+\s(?:src|href)="https?:\/\/[^"]+"[^>]*>/g)].map(([t]) => t);
        expect(tags.filter((t) => !allowed(/="(https?:[^"]+)"/.exec(t)![1]!, t))).toEqual([]);
        expect(urls.length).toBeGreaterThanOrEqual(0);
      });
    });
  }
});
