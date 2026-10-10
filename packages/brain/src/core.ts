// Talks to the Rust receipt (crates/fettle-wasm): JSON in, JSON out, through the module's own memory.
import type { Locale } from "./locale.ts";
import type { Receipt, ReceiptModel } from "./types.ts";

/** "UTC" for anything drawn on a server; an offset (minutes east) for a viewer's own clock. */
export type Zone = "UTC" | { offsetMinutes: number };

export interface Brain {
  receiptFor(report: unknown, locale: Locale, zone?: Zone): { receipt: Receipt; model: ReceiptModel };
  markdownFor(report: unknown, locale: Locale, zone?: Zone): string;
  /** The module's memory (a test checks it settles). */
  readonly memory: WebAssembly.Memory;
}

/** The brain refused the report: unreadable, or a scan time that doesn't exist. */
export class BrainError extends Error {
  override name = "BrainError";
}

type Exports = {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  dealloc(ptr: number, cap: number): void;
  receipt(ptr: number, len: number, locale: number, zoneKind: number, offsetMinutes: number, withMarkdown: number): number;
};

const instantiateOnce = (module: WebAssembly.Module) => new WebAssembly.Instance(module, {}).exports as unknown as Exports;

/** `instantiate` is for tests (a module that crashes on purpose). */
export function createBrain(module: WebAssembly.Module, instantiate: (module: WebAssembly.Module) => Exports = instantiateOnce): Brain {
  let wasm = instantiate(module);
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  function call(report: unknown, locale: Locale, zone: Zone, withMarkdown: boolean) {
    const input = encoder.encode(JSON.stringify(report));
    const offset = zone === "UTC" ? 0 : zone.offsetMinutes;
    let ptr = 0;
    let pair: number;
    try {
      ptr = wasm.alloc(input.length);
      new Uint8Array(wasm.memory.buffer, ptr, input.length).set(input);
      pair = wasm.receipt(ptr, input.length, locale === "it" ? 1 : 0, zone === "UTC" ? 0 : 1, offset, withMarkdown ? 1 : 0);
    } catch (e) {
      // A trap (a Rust panic aborts) can leave the module half-way through: drop it and start a fresh
      // one, so later requests in this isolate aren't served by a broken module. Our own log line,
      // with no report data; the caller still shows "isn't here".
      console.error(JSON.stringify({ event: "brain_crashed", message: String(e).slice(0, 200) }));
      wasm = instantiate(module);
      throw e;
    }
    wasm.dealloc(ptr, input.length);
    // Fresh views each time: the memory may have grown during the call.
    const [outPtr = 0, outLen = 0] = new Uint32Array(wasm.memory.buffer, pair, 2);
    const json = decoder.decode(new Uint8Array(wasm.memory.buffer, outPtr, outLen));
    wasm.dealloc(outPtr, outLen);
    const out = JSON.parse(json);
    if (typeof out.error === "string") throw new BrainError(out.error);
    return out as { receipt: Receipt; model: ReceiptModel; markdown?: string };
  }

  return {
    receiptFor(report, locale, zone = "UTC") {
      const { receipt, model } = call(report, locale, zone, false);
      return { receipt, model };
    },
    markdownFor: (report, locale, zone = "UTC") => call(report, locale, zone, true).markdown ?? "",
    get memory() {
      return wasm.memory;
    },
  };
}
