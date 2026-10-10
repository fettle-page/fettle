import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST, pages } from "./dist-helpers.ts";

const headers = readFileSync(join(DIST, "_headers"), "utf8");
const styleSrc = /style-src ([^;\n]+)/.exec(headers)?.[1] ?? "";
const scriptSrc = /script-src ([^;\n]+)/.exec(headers)?.[1] ?? "";
const themeJs = readFileSync(join(DIST, "theme.js"), "utf8").trim();
const hashOf = (css: string) => `'sha256-${createHash("sha256").update(css, "utf8").digest("base64")}'`;

describe("stylesheets are inlined, and the CSP allows exactly them", () => {
  for (const [file, html] of pages()) {
    it(file, () => {
      expect(html, "no render-blocking stylesheet request").not.toMatch(/<link rel="stylesheet"/);
      const styles = [...html.matchAll(/<style>([\s\S]*?)<\/style>/g)].map(([, css]) => css!);
      expect(styles, "one stylesheet per page").toHaveLength(1);
      expect(styleSrc, "hash listed in _headers").toContain(hashOf(styles[0]!));
      // The early theme script is inlined (no blocking request), allowed by hash, and runs before any style.
      expect(html, "no blocking theme request").not.toContain('<script src="/theme.js">');
      const inline = html.indexOf(`<script>${themeJs}</script>`);
      expect(inline, "inlined theme script").toBeGreaterThan(0);
      expect(inline).toBeLessThan(html.indexOf("<style>"));
      expect(scriptSrc).toContain(hashOf(themeJs));
    });
  }

  it("keeps style-src and script-src strict: self plus hashes, no unsafe-inline", () => {
    expect(styleSrc).toMatch(/^'self'( 'sha256-[A-Za-z0-9+/=]+')+$/);
    expect(scriptSrc).toMatch(/^'self' 'sha256-[A-Za-z0-9+/=]+'$/);
    expect(headers).not.toContain("unsafe-inline");
  });
});
