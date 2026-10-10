import { describe, expect, it } from "vitest";
import { MAX_UPLOAD_BYTES, readCapped } from "../src/http.ts";

const post = (body: BodyInit, headers?: HeadersInit) =>
  new Request("http://example.com/api/reports", { method: "POST", body, headers });

describe("readCapped", () => {
  it("allows 256 KiB uploads", () => {
    expect(MAX_UPLOAD_BYTES).toBe(262144);
  });

  it("accepts a body of exactly the cap", async () => {
    const text = "a".repeat(100);
    expect(await readCapped(post(text), 100)).toEqual({ ok: true, text });
  });

  it("refuses one byte over", async () => {
    expect(await readCapped(post("a".repeat(101)), 100)).toEqual({ ok: false, reason: "too_large" });
  });

  it("trusts a declared Content-Length over the cap without reading", async () => {
    let pulled = false;
    const body = new ReadableStream({ pull(c) { pulled = true; c.enqueue(new Uint8Array(1)); c.close(); } }, { highWaterMark: 0 });
    const req = new Request("http://example.com/", { method: "POST", body, headers: { "content-length": "999999" }, duplex: "half" } as RequestInit);
    expect(await readCapped(req, 100)).toEqual({ ok: false, reason: "too_large" });
    expect(pulled).toBe(false);
  });

  it("stops an endless chunked upload and cancels it", async () => {
    let cancelled = false;
    const body = new ReadableStream({
      pull(c) { c.enqueue(new Uint8Array(64 * 1024)); },
      cancel() { cancelled = true; },
    });
    const req = new Request("http://example.com/", { method: "POST", body, duplex: "half" } as RequestInit);
    expect(await readCapped(req, 262144)).toEqual({ ok: false, reason: "too_large" });
    expect(cancelled).toBe(true);
  });

  it("counts bytes, not characters", async () => {
    const text = "è".repeat(1000); // 2000 bytes
    expect(await readCapped(post(text), 2000)).toEqual({ ok: true, text });
    expect(await readCapped(post(text), 1999)).toEqual({ ok: false, reason: "too_large" });
  });
});
