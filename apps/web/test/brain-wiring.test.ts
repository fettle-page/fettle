import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// The website on the Rust brain: receipts come from @fettle/brain (the Rust
// receipt as WebAssembly), and copy from the crates' own JSON. receipt-ui only draws.
const SRC = fileURLToPath(new URL("../src/", import.meta.url));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? files(p) : /\.(astro|ts|tsx)$/.test(n) ? [p] : [];
  });
}

/** Files that import the TypeScript engine: @fettle/verdicts, or receipt-ui's model and copy (its `stamp`, the browser's local time, stays). */
function usingTheEngine(skip: (rel: string) => boolean): string[] {
  return files(SRC)
    .map((f) => [relative(SRC, f).replaceAll("\\", "/"), readFileSync(f, "utf8")] as const)
    .filter(([rel]) => !skip(rel))
    .filter(([, text]) => /from "@fettle\/verdicts"/.test(text) || /import \{[^}]*\b(buildReceiptModel|ui|toMarkdown)\b[^}]*\} from "@fettle\/receipt-ui/.test(text) || /@fettle\/receipt-ui\/model/.test(text))
    .map(([rel]) => rel);
}

describe("the website on the Rust brain", () => {
  it("neither the site build nor the share page uses the TypeScript engine", () => {
    expect(usingTheEngine(() => false)).toEqual([]);
  });

  it("the TypeScript engine is gone: one brain, in Rust", () => {
    const root = new URL("../../../", import.meta.url);
    // The package itself, not the folder: an old checkout can keep an untracked node_modules there.
    for (const part of ["packages/verdicts/package.json", "packages/verdicts/src"]) expect(existsSync(new URL(part, root)), part).toBe(false);
    for (const pkg of ["apps/web", "worker", "packages/receipt-ui", "packages/brain", "packages/share-client", "packages/report-schema"]) {
      const json = JSON.parse(readFileSync(new URL(`${pkg}/package.json`, root), "utf8"));
      expect({ ...json.dependencies, ...json.devDependencies }, pkg).not.toHaveProperty("@fettle/verdicts");
    }
    expect(existsSync(new URL("packages/receipt-ui/src/model.ts", root))).toBe(false);
  });
});
