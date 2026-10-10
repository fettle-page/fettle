import type { Report } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { deleteShared, shareReport } from "../src/index.ts";

const report: Report = { schema_version: 1, app_version: "0.1.0", collected_at: "2026-10-07T21:00:00Z", elevated: false, facts: {} };
const ORIGIN = "https://fettle.page";
const created = { id: "m0123456789abcdefg", url: `${ORIGIN}/r/m0123456789abcdefg`, deleteUrl: `${ORIGIN}/r/m0123456789abcdefg/delete#tok`, deleteToken: "tok", expiresAt: "2026-11-06T21:00:00.000Z", locked: false };

type Call = { url: string; init: RequestInit };
function fakeFetch(respond: (call: Call) => Response | Promise<Response>) {
  const calls: Call[] = [];
  const fn = async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const call = { url: String(input), init };
    calls.push(call);
    return respond(call);
  };
  return { fn: fn as typeof fetch, calls };
}
const json = (body: unknown, status = 200, headers: Record<string, string> = {}) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", ...headers } });

describe("shareReport", () => {
  it("posts the envelope as JSON and returns the links", async () => {
    const f = fakeFetch(() => json(created, 201));
    const result = await shareReport(ORIGIN, { report, expires: "30d" }, f.fn);
    expect(result).toEqual({ ok: true, shared: created });
    expect(f.calls[0]!.url).toBe(`${ORIGIN}/api/reports`);
    expect(f.calls[0]!.init.method).toBe("POST");
    expect(new Headers(f.calls[0]!.init.headers).get("content-type")).toBe("application/json");
    expect(JSON.parse(String(f.calls[0]!.init.body))).toEqual({ report, expires: "30d" });
  });

  it("sends the password only when there is one", async () => {
    const f = fakeFetch(() => json({ ...created, locked: true }, 201));
    await shareReport(ORIGIN, { report, password: "hunter22" }, f.fn);
    await shareReport(ORIGIN, { report, password: "" }, f.fn);
    expect(JSON.parse(String(f.calls[0]!.init.body)).password).toBe("hunter22");
    expect("password" in JSON.parse(String(f.calls[1]!.init.body))).toBe(false);
  });

  it("refuses a short or long password before sending anything", async () => {
    const f = fakeFetch(() => json(created, 201));
    expect(await shareReport(ORIGIN, { report, password: "short" }, f.fn)).toEqual({ ok: false, error: "password_too_short" });
    expect(await shareReport(ORIGIN, { report, password: "x".repeat(129) }, f.fn)).toEqual({ ok: false, error: "password_too_long" });
    expect(f.calls).toHaveLength(0);
  });

  it("refuses a report over 256 KB before sending it", async () => {
    const f = fakeFetch(() => json(created, 201));
    const big = { ...report, app_version: "x".repeat(262_144) };
    expect(await shareReport(ORIGIN, { report: big }, f.fn)).toEqual({ ok: false, error: "too_large" });
    expect(f.calls).toHaveLength(0);
  });

  it("passes on the server's error code and the wait after a rate limit", async () => {
    const limited = fakeFetch(() => json({ error: "rate_limited" }, 429, { "retry-after": "60" }));
    expect(await shareReport(ORIGIN, { report }, limited.fn)).toEqual({ ok: false, error: "rate_limited", retryAfter: 60 });
    const invalid = fakeFetch(() => json({ error: "invalid_report", detail: "facts: bad" }, 400));
    expect(await shareReport(ORIGIN, { report }, invalid.fn)).toEqual({ ok: false, error: "invalid_report", detail: "facts: bad" });
  });

  it("reports no connection and odd answers without throwing", async () => {
    const offline = fakeFetch(() => Promise.reject(new TypeError("fetch failed")));
    expect(await shareReport(ORIGIN, { report }, offline.fn)).toEqual({ ok: false, error: "network" });
    const html = fakeFetch(() => new Response("<h1>Bad gateway</h1>", { status: 502 }));
    expect(await shareReport(ORIGIN, { report }, html.fn)).toEqual({ ok: false, error: "bad_response" });
    const partial = fakeFetch(() => json({ id: "x" }, 201));
    expect(await shareReport(ORIGIN, { report }, partial.fn)).toEqual({ ok: false, error: "bad_response" });
  });

  it("accepts an origin with a trailing slash", async () => {
    const f = fakeFetch(() => json(created, 201));
    await shareReport(`${ORIGIN}/`, { report }, f.fn);
    expect(f.calls[0]!.url).toBe(`${ORIGIN}/api/reports`);
  });
});

describe("deleteShared", () => {
  it("sends the delete token as a bearer token", async () => {
    const f = fakeFetch(() => new Response(null, { status: 204 }));
    expect(await deleteShared(ORIGIN, created, f.fn)).toEqual({ ok: true });
    expect(f.calls[0]!.url).toBe(`${ORIGIN}/api/reports/${created.id}`);
    expect(f.calls[0]!.init.method).toBe("DELETE");
    expect(new Headers(f.calls[0]!.init.headers).get("authorization")).toBe("Bearer tok");
  });

  it("treats a link that is already gone as deleted", async () => {
    const f = fakeFetch(() => json({ error: "not_found" }, 404));
    expect(await deleteShared(ORIGIN, created, f.fn)).toEqual({ ok: true });
  });

  it("doesn't take someone else's 404 page for a deleted link", async () => {
    const f = fakeFetch(() => new Response("<h1>Not Found</h1>", { status: 404, headers: { "content-type": "text/html" } }));
    expect(await deleteShared(ORIGIN, created, f.fn)).toEqual({ ok: false, error: "bad_response" });
  });

  it("gives up on a stalled connection", async () => {
    const stalled = fakeFetch(({ init }) => new Promise<Response>((_, reject) => init.signal?.addEventListener("abort", () => reject(init.signal!.reason))));
    expect(await shareReport(ORIGIN, { report }, stalled.fn, { timeoutMs: 20 })).toEqual({ ok: false, error: "network" });
    expect(await deleteShared(ORIGIN, created, stalled.fn, { timeoutMs: 20 })).toEqual({ ok: false, error: "network" });
  });

  it("reports a wrong token", async () => {
    const f = fakeFetch(() => json({ error: "forbidden" }, 403));
    expect(await deleteShared(ORIGIN, created, f.fn)).toEqual({ ok: false, error: "forbidden" });
  });
});
