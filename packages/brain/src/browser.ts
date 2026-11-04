// The brain in a visitor's browser (the website's /open page). Import it lazily: the wasm is fetched
// only when loadBrain() runs. Its address comes from the build (a hashed, long-cached file).
import url from "../dist/fettle.wasm?url";
import { createBrain, type Brain } from "./core.ts";

export const wasmUrl: string = url;

type Fetcher = (url: string) => Promise<Response>;

/** Fetches and compiles the receipt; `fetcher` is for tests. */
export async function loadBrain(fetcher: Fetcher = (u) => fetch(u)): Promise<Brain> {
  const res = await fetcher(wasmUrl);
  if (!res.ok) throw new Error(`couldn't load the receipt (${res.status})`);
  // Streaming compile needs the server to say application/wasm; otherwise compile the whole file.
  const streams = typeof WebAssembly.compileStreaming === "function" && (res.headers.get("content-type") ?? "").startsWith("application/wasm");
  const module = streams ? await WebAssembly.compileStreaming(res) : await WebAssembly.compile(await res.arrayBuffer());
  return createBrain(module);
}

export { BrainError, type Brain, type Zone } from "./core.ts";
