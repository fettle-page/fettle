/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";

// The share Worker on the Rust brain: receipts and previews come from
// @fettle/brain (the Rust receipt as WebAssembly), not the TypeScript engine.
const sources = import.meta.glob("../src/**/*.ts", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

describe("the share Worker on the Rust brain", () => {
  it("has its sources to check", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(5);
  });

  it("doesn't import the TypeScript engine", () => {
    const engine = Object.entries(sources)
      .filter(([, text]) => /from "@fettle\/verdicts"|from "@fettle\/receipt-ui\/model"|buildReceiptModel/.test(text))
      .map(([path]) => path);
    expect(engine).toEqual([]);
  });
});
