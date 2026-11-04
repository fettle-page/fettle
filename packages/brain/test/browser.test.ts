import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { loadBrain, wasmUrl } from "../src/browser.ts";

// The browser loader, run under Node with the real wasm bytes standing in for the network.
const bytes = readFileSync(new URL("../dist/fettle.wasm", import.meta.url));
const sample = JSON.parse(readFileSync(new URL("../../../apps/web/src/content/sample-report.json", import.meta.url), "utf8"));
const served = (type: string, status = 200) => async (url: string) => {
  asked.push(url);
  return new Response(status === 200 ? bytes : null, { status, headers: { "content-type": type } });
};
let asked: string[] = [];

describe("the brain in the browser", () => {
  it("compiles the wasm as it streams in and draws a receipt in the visitor's own time", async () => {
    asked = [];
    const brain = await loadBrain(served("application/wasm"));
    expect(asked).toEqual([wasmUrl]);
    const { model } = brain.receiptFor(sample, "en", { offsetMinutes: 60 });
    expect(model.locale).toBe("en");
    expect(model.sections.length).toBeGreaterThan(0);
    // collected_at is 09:12 at +02:00: 08:12 one hour east of UTC, and no "UTC" on it.
    expect(model.meta.when).toBe("07/10/2026 08:12");
  });

  it("falls back to compiling the whole file when the server sends another content type", async () => {
    const brain = await loadBrain(served("application/octet-stream"));
    expect(brain.receiptFor(sample, "it", { offsetMinutes: 60 }).model.locale).toBe("it");
  });

  it("refuses a failed download instead of compiling an error page", async () => {
    await expect(loadBrain(served("text/html", 404))).rejects.toThrow(/404/);
  });

  it("gets its address from the build, as a hashed file", () => {
    expect(wasmUrl).toMatch(/fettle.*\.wasm/);
  });
});
