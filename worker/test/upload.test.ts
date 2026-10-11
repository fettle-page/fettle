import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { hashToken } from "../src/tokens.ts";
import { call, DAY, freshIp, ORIGIN, tiredReport, untilFreshWindow, upload } from "./helpers.ts";

const near = (iso: string, ms: number) => Math.abs(Date.parse(iso) - (Date.now() + ms)) < 5000;

describe("POST /api/reports", () => {
  it("stores a report for 30 days by default and returns its links", async () => {
    const res = await upload({ report: tiredReport() }, { ip: freshIp() });
    expect(res.status).toBe(201);
    const body = await res.json<any>();
    expect(body.id).toMatch(/^m[0-9a-z]{16}$/);
    expect(near(body.expiresAt, 30 * DAY)).toBe(true);
    expect(body.url).toBe(`${ORIGIN}/r/${body.id}`);
    expect(body.deleteUrl).toBe(`${body.url}/delete#${body.deleteToken}`);
    const head = await env.REPORTS.head(`30d/${body.id}`);
    expect(head).not.toBeNull();
    expect(head!.customMetadata!.deleteHash).toBe(await hashToken(body.deleteToken));
    expect(head!.customMetadata!.deleteHash).not.toBe(body.deleteToken);
    expect(head!.customMetadata!.expiresAt).toBe(body.expiresAt);
  });

  it("honours the expiry choice", async () => {
    const week = await (await upload({ expires: "7d", report: tiredReport() }, { ip: freshIp() })).json<any>();
    expect(week.id[0]).toBe("w");
    expect(await env.REPORTS.head(`7d/${week.id}`)).not.toBeNull();
    const year = await (await upload({ expires: "1y", report: tiredReport() }, { ip: freshIp() })).json<any>();
    expect(year.id[0]).toBe("y");
    expect(near(year.expiresAt, 365 * DAY)).toBe(true);
    expect(await env.REPORTS.head(`1y/${year.id}`)).not.toBeNull();
    const bad = await upload({ expires: "2d", report: tiredReport() }, { ip: freshIp() });
    expect(bad.status).toBe(400);
    expect(await bad.json()).toEqual({ error: "invalid_request" });
  });

  it("strips fields the report format doesn't know", async () => {
    const report = tiredReport();
    report.hostname = "DESKTOP-ME";
    report.facts.system.data.serial = "ABC123";
    const res = await upload({ report }, { ip: freshIp() });
    expect(res.status).toBe(201);
    const { id } = await res.json<any>();
    const stored = await (await env.REPORTS.get(`30d/${id}`))!.text();
    expect(stored.includes("hostname")).toBe(false);
    expect(stored.includes("DESKTOP-ME")).toBe(false);
    expect(stored.includes("ABC123")).toBe(false);
  });

  it("refuses what isn't a report", async () => {
    const ip = freshIp();
    const plain = await upload("{}", { ip, contentType: "text/plain" });
    expect(plain.status).toBe(415);
    expect(await plain.json()).toEqual({ error: "unsupported_media_type" });
    const broken = await upload("{", { ip: freshIp() });
    expect(broken.status).toBe(400);
    expect(await broken.json()).toEqual({ error: "invalid_json" });
    const notArray = await upload([], { ip: freshIp() });
    expect(notArray.status).toBe(400);
    expect(await notArray.json()).toEqual({ error: "invalid_request" });
    const partial = await upload({ report: { schema_version: 1 } }, { ip: freshIp() });
    expect(partial.status).toBe(400);
    const body = await partial.json<any>();
    expect(body.error).toBe("invalid_report");
    expect(body.detail.length).toBeGreaterThan(0);
    const impossible = await upload({ report: { ...tiredReport(), collected_at: "2026-13-45T25:61:61Z" } }, { ip: freshIp() });
    expect(impossible.status).toBe(400);
    expect((await impossible.json<any>()).error).toBe("invalid_report");
    const newer = await upload({ report: { ...tiredReport(), schema_version: 2 } }, { ip: freshIp() });
    expect(newer.status).toBe(400);
    expect((await newer.json<any>()).error).toBe("invalid_report");
  });

  it("accepts exactly 64 KiB and refuses one byte more", async () => {
    const base = JSON.stringify({ report: tiredReport() });
    const pad = (n: number) => base + " ".repeat(n - new TextEncoder().encode(base).byteLength);
    expect((await upload(pad(65536), { ip: freshIp() })).status).toBe(201);
    const over = await upload(pad(65537), { ip: freshIp() });
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "too_large" });
  });

  // The largest real report is about 4 KB (5 KB with 26 startup apps). This one is far past any real
  // PC, with every name at its length limit, and still has to fit.
  it("takes a report with every list long and every name at its limit", async () => {
    const report = tiredReport();
    const name = "N".repeat(128);
    const f = report.facts;
    f.startup.data.items = Array.from({ length: 100 }, () => ({ name, location: "run_key", enabled: true }));
    f.storage.data.disks = Array.from({ length: 16 }, () => ({ ...f.storage.data.disks[0], model: name }));
    f.graphics.data.adapters = Array.from({ length: 8 }, () => ({ ...f.graphics.data.adapters[0], name }));
    f.memory.data.modules = Array.from({ length: 16 }, () => ({ ...f.memory.data.modules[0], part_number: name, manufacturer: name, bank_label: name, device_locator: name }));
    f.system.data.displays = Array.from({ length: 8 }, () => ({ name }));
    Object.assign(f.system.data, { cpu_name: name, board: name });
    const body = JSON.stringify({ report });
    expect(new TextEncoder().encode(body).byteLength).toBeGreaterThan(30_000);
    expect((await upload(body, { ip: freshIp() })).status).toBe(201);
  });

  it("refuses an endless chunked upload", async () => {
    const body = new ReadableStream({ pull(c) { c.enqueue(new TextEncoder().encode(" ".repeat(65536))); } });
    const res = await call("/api/reports", {
      method: "POST", body, headers: { "content-type": "application/json", "cf-connecting-ip": freshIp() }, duplex: "half",
    } as RequestInit);
    expect(res.status).toBe(413);
  });

  it("allows 10 uploads a minute per IP", { timeout: 30_000 }, async () => {
    await untilFreshWindow();
    const ip = "203.0.113.7";
    for (let i = 0; i < 10; i++) expect((await upload({ report: tiredReport() }, { ip })).status).toBe(201);
    const limited = await upload({ report: tiredReport() }, { ip });
    expect(limited.status).toBe(429);
    expect(limited.headers.get("retry-after")).toBe("60");
    expect(await limited.json()).toEqual({ error: "rate_limited" });
    expect((await upload({ report: tiredReport() }, { ip: "203.0.113.8" })).status).toBe(201);
  });

  it("still limits uploads with no client IP", { timeout: 30_000 }, async () => {
    await untilFreshWindow();
    const statuses: number[] = [];
    for (let i = 0; i < 11; i++) statuses.push((await upload({ report: tiredReport() })).status);
    expect(statuses.slice(0, 10).every((s) => s === 201)).toBe(true);
    expect(statuses[10]).toBe(429);
  });

  it("only takes POST", async () => {
    const res = await call("/api/reports");
    expect(res.status).toBe(405);
    expect(res.headers.get("allow")).toBe("POST");
  });
});
