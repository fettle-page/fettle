import { describe, expectTypeOf, it } from "vitest";
import type { SharePayload as Real } from "../../src/share/payload.ts";
import type { SharePayload as Declared } from "../../src/share/render-types.d.ts";

describe("render-types.d.ts", () => {
  it("declares exactly the payload the page reads (checked by tsc)", () => {
    expectTypeOf<Declared>().toEqualTypeOf<Real>();
  });
});
