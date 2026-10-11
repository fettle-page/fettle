import { env } from "cloudflare:workers";
import { runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { call, DAY, freshIp, share, untilFreshWindow } from "./helpers.ts";

const unlock = (id: string, body: Record<string, unknown>, ip = freshIp()) =>
  call(`/api/reports/${id}/unlock`, {
    method: "POST",
    headers: { "content-type": "application/json", "cf-connecting-ip": ip },
    body: JSON.stringify(body),
  });

async function opened(id: string) {
  const res = await unlock(id, { password: "hunter22" });
  expect(res.status).toBe(200);
  return res.json<{ pass: string; passExpiresAt: string; model: unknown }>();
}

describe("unlock passes", () => {
  it("come with a right password and last 24 hours", async () => {
    const { id } = await share({ password: "hunter22" });
    const body = await opened(id);
    expect(body.pass).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(Math.abs(Date.parse(body.passExpiresAt) - (Date.now() + DAY))).toBeLessThan(5000);
  });

  it("open the report without the password", async () => {
    const { id } = await share({ password: "hunter22" });
    const { pass, model } = await opened(id);
    const res = await unlock(id, { pass });
    expect(res.status).toBe(200);
    const body = await res.json<any>();
    expect(body.model).toEqual(model);
    // The same pass comes back, renewed.
    expect(body.pass).toBe(pass);
  });

  it("keep working while the guess limit is used up", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    const { pass } = await opened(id);
    await untilFreshWindow();
    for (let i = 0; i < 10; i++) await unlock(id, { password: `wrong-${i}xx` });
    expect((await unlock(id, { password: "hunter22" })).status).toBe(429);
    expect((await unlock(id, { pass })).status).toBe(200);
  });

  it("only open the report they were issued for", async () => {
    const a = await share({ password: "hunter22" });
    const b = await share({ password: "hunter22" });
    const { pass } = await opened(a.id);
    const res = await unlock(b.id, { pass });
    expect(res.status).toBe(403);
    expect(await res.json()).toEqual({ error: "invalid_pass" });
    expect((await unlock(a.id, { pass: "x".repeat(43) })).status).toBe(403);
  });

  it("aren't counted as password guesses", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    await untilFreshWindow();
    for (let i = 0; i < 12; i++) expect((await unlock(id, { pass: `bad-pass-${i}` })).status).toBe(403);
    expect((await unlock(id, { password: "hunter22" })).status).toBe(200);
  });

  it("are stored only as hashes, and cleared once they expire", async () => {
    const { id } = await share({ password: "hunter22" });
    const { pass } = await opened(id);
    const stub = env.UNLOCK_GUARD.getByName(id);
    const stored = await runInDurableObject(stub, async (_, state) => [...(await state.storage.list()).keys()]);
    expect(stored.some((k) => k.startsWith("pass:"))).toBe(true);
    expect(stored.join(" ")).not.toContain(pass);

    await runInDurableObject(stub, async (_, state) => {
      for (const k of (await state.storage.list({ prefix: "pass:" })).keys()) await state.storage.put(k, Date.now() - 1);
    });
    await runDurableObjectAlarm(stub);
    expect((await unlock(id, { pass })).status).toBe(403);
  });

  it("are renewed for another 24 hours each time they're used", async () => {
    const { id } = await share({ password: "hunter22" });
    const { pass } = await opened(id);
    await runInDurableObject(env.UNLOCK_GUARD.getByName(id), async (_, state) => {
      for (const k of (await state.storage.list({ prefix: "pass:" })).keys()) await state.storage.put(k, Date.now() + 60_000);
    });
    const res = await unlock(id, { pass });
    expect(res.status).toBe(200);
    const body = await res.json<{ pass: string; passExpiresAt: string }>();
    expect(body.pass).toBe(pass);
    expect(Math.abs(Date.parse(body.passExpiresAt) - (Date.now() + DAY))).toBeLessThan(5000);
  });

  it("need a password or a pass", async () => {
    const { id } = await share({ password: "hunter22" });
    expect((await unlock(id, {})).status).toBe(400);
  });
});
