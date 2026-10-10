import { readdirSync, readFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { generate } from "../scripts/gen-types.mjs";
import { ReceiptModelSchema, ReceiptSchema } from "../src/generated.ts";

// The receipt's TypeScript types are generated from the Rust structs (npm run gen:brain-types), the
// way report-schema is generated from fettle-facts: one hand-written definition, in Rust.
const CASES = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
const cases = readdirSync(CASES).sort().map((f) => JSON.parse(readFileSync(new URL(f, CASES), "utf8")));

describe("the receipt's types come from Rust", () => {
  // What `npm run gen:brain-types` would write now: the Rust schema, and the TypeScript made from it.
  let fresh = { schemaJson: "", code: "" };
  beforeAll(() => {
    fresh = generate();
  }, 300_000);

  it("every golden model and receipt fits the generated schema", () => {
    for (const c of cases) {
      expect(() => ReceiptSchema.parse(c.receipt), c.name).not.toThrow();
      for (const locale of ["en", "it"]) expect(() => ReceiptModelSchema.parse(c[locale]), `${c.name} ${locale}`).not.toThrow();
    }
  });

  it("the committed schema is current (run npm run gen:brain-types after changing the Rust structs)", () => {
    const committed = readFileSync(new URL("../schema/brain.schema.json", import.meta.url), "utf8");
    expect(JSON.parse(committed)).toEqual(JSON.parse(fresh.schemaJson));
  });

  it("the generated TypeScript is current too, untouched by hand", () => {
    // The types come from generated.ts, not the schema file: a hand edit there would slip past.
    expect(readFileSync(new URL("../src/generated.ts", import.meta.url), "utf8")).toBe(fresh.code);
  });

  it("a tone is one of the four the receipt knows", () => {
    expect(() => ReceiptModelSchema.shape.tone.parse("fine")).not.toThrow();
    expect(() => ReceiptModelSchema.shape.tone.parse("great")).toThrow();
  });
});
