import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST, page } from "./dist-helpers.ts";

// The share page's browser code draws a finished receipt. It carries no
// verdict engine, no verdict wording and no zod: the server builds the receipt with the Rust brain.
function scriptsOf(html: string): string[] {
  const seen = new Set<string>();
  const queue = [...html.matchAll(/<script type="module" src="(\/_astro\/[^"]+\.js)"/g)].map((m) => m[1]!);
  while (queue.length) {
    const src = queue.shift()!;
    if (seen.has(src)) continue;
    seen.add(src);
    const code = readFileSync(join(DIST, src), "utf8");
    // Imports between chunks: `import … from "./x.js"`, `import "./x.js"`, and loaded later, `import("./x.js")`.
    for (const m of code.matchAll(/(?:from|import)\s*\(?\s*"\.\/([^"]+\.js)"/g)) queue.push(`/_astro/${m[1]}`);
  }
  return [...seen].map((src) => readFileSync(join(DIST, src), "utf8"));
}

const verdictsEn = JSON.parse(readFileSync(new URL("../../../crates/fettle-verdicts/copy/en.json", import.meta.url), "utf8")) as Record<string, string>;

describe("the share page's browser code", () => {
  const code = scriptsOf(page("share.html")).join("\n");

  it("loads", () => {
    expect(existsSync(join(DIST, "share.html"))).toBe(true);
    expect(code.length).toBeGreaterThan(1000);
  });

  it("never loads the Rust receipt (D5: the server sends the receipt finished)", () => {
    expect(code).not.toContain("WebAssembly");
    expect(code).not.toContain(".wasm");
  });

  it("carries no zod", () => {
    expect(code).not.toContain("ZodError");
  });

  it("carries no verdict wording (the server sends it already written)", () => {
    // A verdict line in its own words, without its {param} slots.
    const line = verdictsEn["storage.no_health"]!.replace(/\{\w+\}/g, "");
    expect(line.length).toBeGreaterThan(20);
    expect(code).not.toContain(line.trim());
  });
});
