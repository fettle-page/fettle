// The site, the share page and the receipt demo are plain CSS. Tailwind was only ever used for .sr-only,
// which the site's own CSS already defines, and for its reset, now @fettle/receipt-ui/reset.css.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { pages } from "./dist-helpers.ts";

const read = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

describe("Tailwind", () => {
  it("is no dependency of the site or the receipt package, nor in the lockfile", () => {
    for (const pkg of ["apps/web/package.json", "packages/receipt-ui/package.json"]) {
      const json = JSON.parse(read(pkg));
      const deps = Object.keys({ ...json.dependencies, ...json.devDependencies, ...json.peerDependencies });
      expect(deps.filter((d) => d.includes("tailwind")), pkg).toEqual([]);
    }
    const locked = Object.keys(JSON.parse(read("package-lock.json")).packages);
    expect(locked.filter((p) => p.includes("tailwind"))).toEqual([]);
  });

  it("isn't loaded by any build or stylesheet", () => {
    for (const file of ["apps/web/astro.config.mjs", "packages/receipt-ui/demo/vite.config.ts"]) expect(read(file), file).not.toMatch(/tailwind/i);
    for (const file of ["apps/web/src/styles/site.css", "apps/web/src/share/share.css", "packages/receipt-ui/demo/demo.css", "packages/receipt-ui/src/theme.css"]) {
      expect(read(file), file).not.toMatch(/@import "tailwindcss"|@source|@theme|@apply/);
    }
  });

  it("leaves nothing in the built pages", () => {
    for (const [file, html] of pages()) expect(html, file).not.toMatch(/tailwind|--tw-/i);
  });
});

describe(".sr-only", () => {
  // The standard visually-hidden pattern: a 1 px box, clipped away, out of the flow. Never display: none
  // or visibility: hidden, which would hide the text from screen readers too.
  const HIDES = ["position:absolute", "width:1px", "height:1px", "overflow:hidden", "clip-path:inset(50%)", "white-space:nowrap"];

  it("hides text from sight but not from screen readers, on every page that uses it", () => {
    const using = pages().filter(([, html]) => /class="[^"]*\bsr-only\b/.test(html));
    expect(using.length).toBeGreaterThan(10);
    for (const [file, html] of using) {
      // One rule, the site's own: not Tailwind's as well. (The receipt's scoped ".fettle-receipt .sr-only" isn't this.)
      const rules = [...html.matchAll(/(?:^|[{};])\s*\.sr-only\s*\{([^}]*)\}/g)].map(([, body]) => body!.replace(/\s+/g, ""));
      expect(rules, file).toHaveLength(1);
      const declarations = rules[0]!.split(";");
      for (const d of HIDES) expect(declarations, `${file}: ${d}`).toContain(d);
      expect(rules[0], file).not.toMatch(/display:none|visibility:hidden/);
    }
  });

  it("comes after a reset kept in a cascade layer, so the page's own rules win over the reset as before", () => {
    const [, html] = pages().find(([file]) => file === "faq.html")!;
    // The minifier may fold "@layer theme, base;" into the blocks' own order: theme first, then base.
    const theme = html.search(/@layer theme\s*[{,]/);
    const base = html.search(/@layer base\s*\{\s*\*\s*,/);
    expect(theme).toBeGreaterThan(-1);
    expect(base).toBeGreaterThan(theme);
  });
});
