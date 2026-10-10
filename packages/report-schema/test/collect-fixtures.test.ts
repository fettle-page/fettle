import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { generate, OUT } from "../../../crates/fettle-collect/tests/collect-fixtures.mjs";

describe("Rust collector parity fixtures", () => {
  it("are current with the JS converter (regenerate: node crates/fettle-collect/tests/collect-fixtures.mjs)", () => {
    const files = Object.entries(generate());
    expect(files.length).toBeGreaterThan(0);
    for (const [name, text] of files) expect(readFileSync(join(OUT, name), "utf8"), name).toBe(text);
  });
});
