import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { createBrain } from "../src/core.ts";

// The wasm build aborts on a Rust panic (panic = "abort"), which reaches JavaScript as a trap and can
// leave the module half-way through. A long-lived Worker would otherwise keep using it for every later
// request, and the share page turns the error into a quiet 404.
const wasm = new WebAssembly.Module(readFileSync(new URL("../dist/fettle.wasm", import.meta.url)));
const CASES = new URL("../../../crates/fettle-receipt/tests/parity/cases/", import.meta.url);
const report = JSON.parse(readFileSync(new URL(readdirSync(CASES).sort()[0]!, CASES), "utf8")).report;

describe("a crash inside the brain", () => {
  it("is logged and thrown, and the next receipt comes from a fresh module", () => {
    let made = 0;
    let crash = true;
    const instantiate = (module: WebAssembly.Module) => {
      made++;
      const real = new WebAssembly.Instance(module, {}).exports as Record<string, (...a: number[]) => number>;
      // The first call traps as a panic would.
      const receipt = (...a: number[]) => {
        if (crash) {
          crash = false;
          throw new WebAssembly.RuntimeError("unreachable");
        }
        return real.receipt!(...a);
      };
      return { ...real, receipt } as never;
    };
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const brain = createBrain(wasm, instantiate);

    expect(() => brain.receiptFor(report, "en")).toThrow(WebAssembly.RuntimeError);
    expect(made).toBe(2);
    expect(logged).toHaveBeenCalledWith(expect.stringContaining("brain_crashed"));
    expect(brain.receiptFor(report, "it").model.locale).toBe("it");
    logged.mockRestore();
  });

  it("a report the brain refuses is not a crash: same module, nothing logged", () => {
    let made = 0;
    const brain = createBrain(wasm, (module) => (made++, new WebAssembly.Instance(module, {}).exports as never));
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => brain.receiptFor({ ...report, collected_at: "2026-13-40T99:00:00Z" }, "en")).toThrow(/scan time/);
    expect(made).toBe(1);
    expect(logged).not.toHaveBeenCalled();
    logged.mockRestore();
  });
});
