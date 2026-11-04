import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST, page, text } from "./dist-helpers.ts";

// /open: a saved report opened in the browser, never uploaded.
const en = page("open.html");
const it_ = page("it/open.html");
const main = (html: string) => html.slice(html.indexOf('<main id="main">'), html.indexOf("</main>"));
const footLinks = (html: string) => /<nav class="foot-links"[^>]*>([\s\S]*?)<\/nav>/.exec(html)?.[1] ?? "";
/** The page's tags in order, attributes dropped: the two languages must be built from the same parts. */
const parts = (html: string) => [...main(html).matchAll(/<([a-z][a-z0-9]*)\b[^>]*>/g)].map((m) => m[1]).join(" ");

/** The page's own scripts as they load before anything is picked: static imports only, not import(). */
function upFront(html: string): Map<string, string> {
  const seen = new Map<string, string>();
  const queue = [...html.matchAll(/<script type="module" src="(\/_astro\/[^"]+\.js)"/g)].map((m) => m[1]!);
  while (queue.length) {
    const src = queue.shift()!;
    if (seen.has(src)) continue;
    const code = readFileSync(join(DIST, src), "utf8");
    seen.set(src, code);
    for (const m of code.matchAll(/(?:from\s*|import\s*)"\.\/([^"]+\.js)"/g)) queue.push(`/_astro/${m[1]}`);
  }
  return seen;
}

describe("/open", () => {
  it("is built in both languages, each switching to the other", () => {
    expect(en).toMatch(/<html lang="en"/);
    expect(it_).toMatch(/<html lang="it"/);
    expect(en).toMatch(/href="\/it\/open" hreflang="it"/);
    expect(it_).toMatch(/href="\/open" hreflang="en"/);
    expect(en).toContain('<link rel="canonical" href="https://fettle.page/open">');
  });

  it("is in the sitemap in both languages", () => {
    const sitemap = page("sitemap-0.xml");
    expect(sitemap).toContain("<loc>https://fettle.page/open</loc>");
    expect(sitemap).toContain("<loc>https://fettle.page/it/open</loc>");
  });

  it("has the same parts in each language", () => {
    expect(parts(it_)).toBe(parts(en));
  });

  it("has a file picker for .json files, a drop zone and a place for messages", () => {
    for (const html of [en, it_]) {
      expect(html).toMatch(/<input[^>]*type="file"[^>]*accept="\.json,application\/json"/);
      expect(html).toContain("data-open-drop");
      expect(html).toMatch(/role="status"/);
      expect(html).toContain("data-open-receipt");
    }
  });

  it("says plainly that the file stays on this computer", () => {
    expect(text(main(en))).toMatch(/stays on this computer/);
    expect(text(main(en))).toMatch(/isn't uploaded/);
    expect(text(main(it_))).toMatch(/resta su questo computer/);
    expect(text(main(it_))).toMatch(/non viene caricato/);
  });

  it("leaves no English sentence on the Italian page", () => {
    const sentences = [...main(en).matchAll(/<(?:p|li|h1|h2|span|label)[^>]*>([^<]{12,})</g)].map(([, s]) => text(s!).trim());
    expect(sentences.length).toBeGreaterThan(2);
    for (const s of sentences) expect(text(it_), s).not.toContain(s);
  });
});

describe("/open before a file is picked", () => {
  const code = [...upFront(en).values()].join("\n");

  it("loads no WebAssembly, no zod and no React", () => {
    expect(code).not.toContain("WebAssembly");
    expect(code).not.toContain(".wasm");
    expect(code).not.toContain("ZodError");
    expect(code).not.toContain("react.production");
    expect(code).not.toContain("__SECRET_INTERNALS");
  });

  it("stays light: under 12 KB of JavaScript", () => {
    const bytes = [...upFront(en).values()].reduce((n, c) => n + Buffer.byteLength(c), 0);
    expect(bytes).toBeLessThan(12_000);
  });

  it("loads the receipt later, from a hashed wasm file that is the brain's own", () => {
    const later = readdirSync(join(DIST, "_astro")).filter((f) => f.endsWith(".js")).map((f) => readFileSync(join(DIST, "_astro", f), "utf8"));
    const wasm = readdirSync(join(DIST, "_astro")).filter((f) => /^fettle\.[\w-]+\.wasm$/.test(f));
    expect(wasm).toHaveLength(1);
    expect(later.some((c) => c.includes(`/_astro/${wasm[0]}`) && c.includes("compileStreaming"))).toBe(true);
    const brainWasm = new URL("../../../packages/brain/dist/fettle.wasm", import.meta.url);
    expect(readFileSync(join(DIST, "_astro", wasm[0]!)).equals(readFileSync(brainWasm))).toBe(true);
  });
});

describe("links to /open", () => {
  it("are in the footer in each language", () => {
    expect(footLinks(page("index.html"))).toContain('<a href="/open">Open a saved report</a>');
    expect(footLinks(page("it.html"))).toContain('<a href="/it/open">Apri un report salvato</a>');
  });

  it("are in the FAQ answer about saving the report as JSON", () => {
    const answer = (html: string) => /<details class="faq-item" id="forum">([\s\S]*?)<\/details>/.exec(html)?.[1] ?? "";
    expect(answer(page("faq.html"))).toContain('href="/open"');
    expect(answer(page("it/faq.html"))).toContain('href="/it/open"');
  });

  it("the page exists where they point", () => {
    expect(existsSync(join(DIST, "open.html"))).toBe(true);
    expect(existsSync(join(DIST, "it/open.html"))).toBe(true);
  });
});
