import { describe, expect, it } from "vitest";
import { explainBuildFailure } from "../scripts/build.mjs";

describe("building the brain", () => {
  it("a missing wasm target is explained, not dumped", () => {
    const stderr = "error[E0463]: can't find crate for `core`\n  = note: the `wasm32-unknown-unknown` target may not be installed";
    expect(explainBuildFailure(stderr)).toContain("rustup target add wasm32-unknown-unknown");
  });

  it("other failures pass through", () => {
    expect(explainBuildFailure("error: something else")).toBe("error: something else");
  });
});
