import { env } from "cloudflare:workers";
import { runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { budgetName, UPLOADS_PER_DAY, type UploadBudget } from "../src/budget.ts";
import { call, DAY, freshIp, tiredReport, untilFreshWindow, upload } from "./helpers.ts";

const MINUTE = 60_000;
const today = () => Math.floor(Date.now() / DAY) * DAY + 3_600_000;
let n = 0;
const fresh = () => env.UPLOAD_BUDGET.getByName(`budget-test-${Date.now()}-${n++}`);
const takes = (stub: DurableObjectStub<UploadBudget>, count: number, start: number) =>
  runInDurableObject(stub, async (budget: UploadBudget) => {
    const waits: number[] = [];
    for (let i = 0; i < count; i++) waits.push(await budget.take(start + i * MINUTE));
    return waits;
  });

describe("the daily upload budget", () => {
  it("allows a day's uploads per address, then says to come back after midnight UTC", async () => {
    const stub = fresh();
    const start = today();
    expect((await takes(stub, UPLOADS_PER_DAY, start)).every((w) => w === 0)).toBe(true);
    const at = start + UPLOADS_PER_DAY * MINUTE;
    expect(await takes(stub, 1, at)).toEqual([Math.ceil((Math.floor(at / DAY + 1) * DAY - at) / 1000)]);
    expect(await takes(stub, 1, start + DAY)).toEqual([0]);
  });

  it("clears itself once the day is over", async () => {
    const stub = fresh();
    await takes(stub, 3, today() - 2 * DAY);
    await runDurableObjectAlarm(stub);
    expect(await runInDurableObject(stub, async (_, state) => (await state.storage.list()).size)).toBe(0);
  });

  it("is named by a hash, never by the address", async () => {
    const name = await budgetName("198.51.100.200");
    expect(name).toMatch(/^[0-9a-f]{64}$/);
    expect(await budgetName("198.51.100.200")).toBe(name);
    expect(await budgetName("198.51.100.201")).not.toBe(name);
    expect(await budgetName("198.51.100.200", Date.now() + DAY)).not.toBe(name);
  });

  it("refuses an upload from an address that used up its day, and only that address", async () => {
    const ip = freshIp();
    await runInDurableObject(env.UPLOAD_BUDGET.getByName(await budgetName(ip)), (_, state) =>
      state.storage.put("used", { day: Math.floor(Date.now() / DAY), count: UPLOADS_PER_DAY }),
    );
    const refused = await upload({ report: tiredReport() }, { ip });
    expect(refused.status).toBe(429);
    expect(await refused.json()).toEqual({ error: "rate_limited" });
    expect(Number(refused.headers.get("retry-after"))).toBeGreaterThan(60);
    expect((await upload({ report: tiredReport() }, { ip: freshIp() })).status).toBe(201);
  });
});

describe("reading limits", () => {
  const from = (ip: string, init: RequestInit = {}) => ({ ...init, headers: { ...(init.headers as Record<string, string>), "cf-connecting-ip": ip } });

  it("cap share page views per address", { timeout: 60_000 }, async () => {
    await untilFreshWindow();
    const ip = freshIp();
    for (let i = 0; i < 60; i++) expect((await call("/r/7d-nothinghere00000", from(ip))).status, `view ${i}`).toBe(404);
    expect((await call("/r/7d-nothinghere00000", from(ip))).status).toBe(429);
    expect((await call("/r/7d-nothinghere00000", from(freshIp()))).status).toBe(404);
  });

  it("cap report reads through the API per address", { timeout: 60_000 }, async () => {
    await untilFreshWindow();
    const ip = freshIp();
    for (let i = 0; i < 60; i++) expect((await call("/api/reports/7d-nothinghere00000", from(ip))).status, `read ${i}`).toBe(404);
    expect((await call("/api/reports/7d-nothinghere00000", from(ip))).status).toBe(429);
  });

  it("cap deletes per address, before looking the report up", { timeout: 60_000 }, async () => {
    await untilFreshWindow();
    const ip = freshIp();
    const del = () => call("/api/reports/7d-nothinghere00000", from(ip, { method: "DELETE", headers: { authorization: "Bearer not-a-token" } }));
    for (let i = 0; i < 60; i++) expect((await del()).status, `delete ${i}`).toBe(404);
    expect((await del()).status).toBe(429);
  });

  it("check the unlock limit before looking the report up", { timeout: 60_000 }, async () => {
    await untilFreshWindow();
    const ip = freshIp();
    const unlock = () => call("/api/reports/7d-nothinghere00000/unlock", from(ip, { method: "POST", headers: { "content-type": "application/json" }, body: "{}" }));
    for (let i = 0; i < 20; i++) expect((await unlock()).status, `unlock ${i}`).toBe(404);
    expect((await unlock()).status).toBe(429);
  });
});
