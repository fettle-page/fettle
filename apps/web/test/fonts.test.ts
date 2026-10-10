import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page, pages } from "./dist-helpers.ts";

const pub = (f: string) => new URL(`../public/fonts/${f}`, import.meta.url);
const FACES = ["bricolage-display-800.woff2", "bricolage-text-500.woff2", "bricolage-text-800.woff2", "fettle-mono-400.woff2", "fettle-mono-600.woff2"];

describe("fonts", () => {
  it("ship as small trimmed files, with the OFL licence next to them", () => {
    for (const f of FACES) {
      expect(existsSync(pub(f)), f).toBe(true);
      expect(statSync(pub(f)).size, f).toBeLessThan(30_000);
    }
    expect(readFileSync(pub("OFL-bricolage-grotesque.txt"), "utf8")).toContain("SIL OPEN FONT LICENSE");
    // IBM's own licence text, including the reserved name that made the "Fettle Mono" rename necessary.
    expect(readFileSync(pub("OFL-ibm-plex-mono.txt"), "utf8")).toContain('Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"');
  });

  it("are kept for a year, so a return visit doesn't show the fallback font first", () => {
    // Measured 2026-10-09: with max-age=0 all five went back to the network on every return visit and
    // arrived after the first paint on a slow phone connection, so the text swapped fonts in front of you.
    const headers = readFileSync(new URL("../public/_headers", import.meta.url), "utf8");
    expect(headers).toMatch(/^\/fonts\/\*\n {2}Cache-Control: public, max-age=31536000, immutable$/m);
  });

  it("change name whenever they change, as browsers keep them for a year", () => {
    // A changed font under an old name would never reach anyone who has the old one cached. If you
    // rebuild a font, give it a new file name (and update fonts.css, the preloads and this list).
    const PINNED: Record<string, string> = {
      "bricolage-display-800.woff2": "10c7049d8cd8c945",
      "bricolage-text-500.woff2": "13590eb1b831105e",
      "bricolage-text-800.woff2": "5fe2fc862df7e565",
      "fettle-mono-400.woff2": "8c45ffc015f9ab17",
      "fettle-mono-600.woff2": "9ce95efcdf9bc052",
    };
    for (const f of FACES) expect(createHash("sha256").update(readFileSync(pub(f))).digest("hex").slice(0, 16), f).toBe(PINNED[f]);
  });

  it("no longer load the full Fontsource files", () => {
    for (const css of ["../src/styles/site.css", "../src/share/share.css"]) {
      const text = readFileSync(new URL(css, import.meta.url), "utf8");
      expect(text, css).not.toContain("@fontsource-variable/bricolage-grotesque");
      expect(text, css).not.toContain("@fontsource/ibm-plex-mono");
    }
  });

  it("preload the headline face and the receipt title face on site pages", () => {
    for (const [file, html] of pages()) {
      if (file === "share.html") continue;
      const preloads = [...html.matchAll(/<link rel="preload"[^>]*>/g)].map(([t]) => t);
      expect(preloads, file).toEqual([
        '<link rel="preload" href="/fonts/bricolage-display-800.woff2" as="font" type="font/woff2" crossorigin>',
        '<link rel="preload" href="/fonts/fettle-mono-600.woff2" as="font" type="font/woff2" crossorigin>',
      ]);
    }
    expect(page("index.html")).not.toContain("bricolage-grotesque-latin-opsz");
  });
});
