import { env } from "cloudflare:workers";
import { brain } from "@fettle/brain/worker";
import { parseReport } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { newId } from "../src/ids.ts";
import { hashPassword } from "../src/passwords.ts";
import { call, freshIp, share, tiredReport, untilFreshWindow, upload } from "./helpers.ts";

const unlock = (id: string, password: string, ip = freshIp(), locale?: unknown) =>
  call(`/api/reports/${id}/unlock`, {
    method: "POST",
    headers: { "content-type": "application/json", "cf-connecting-ip": ip },
    body: JSON.stringify({ password, locale }),
  });

/** The receipt a visitor sees, built the way the share page's server builds it. */
const modelOf = (locale: "en" | "it") => {
  const parsed = parseReport(tiredReport());
  if (!parsed.ok) throw new Error(parsed.error);
  return brain.receiptFor(parsed.report, locale, "UTC").model;
};

describe("password-protected reports", () => {
  it("store a password hash, never the password", async () => {
    const created = await share({ password: "hunter22" });
    expect(created.locked).toBe(true);
    const object = await env.REPORTS.get(`30d/${created.id}`);
    expect(object!.customMetadata!.pwSalt).toBeTruthy();
    expect(object!.customMetadata!.pwHash).toMatch(/^[0-9a-f]{64}$/);
    expect(JSON.stringify(object!.customMetadata)).not.toContain("hunter22");
    expect(await object!.text()).not.toContain("hunter22");
  });

  it("need 8 to 128 characters", async () => {
    for (const password of ["hunter2", "a".repeat(129), 12345678]) {
      const res = await upload({ password, report: tiredReport() }, { ip: freshIp() });
      expect(res.status, String(password)).toBe(400);
      expect(await res.json()).toEqual({ error: "invalid_request" });
    }
  });

  it("are off without a password", async () => {
    const created = await share();
    expect(created.locked).toBe(false);
    expect((await env.REPORTS.head(`30d/${created.id}`))!.customMetadata!.pwHash).toBeUndefined();
  });

  it("can't be read without the password", async () => {
    const { id } = await share({ password: "hunter22" });
    const res = await call(`/api/reports/${id}`);
    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: "password_required" });
  });

  it("open with the right password only", async () => {
    const { id, expiresAt } = await share({ password: "hunter22" });
    const ok = await unlock(id, "hunter22");
    expect(ok.status).toBe(200);
    const body = await ok.json<any>();
    // The finished receipt: the browser never gets, or builds from, the raw report.
    expect(body.model).toEqual(modelOf("en"));
    expect(body.report).toBeUndefined();
    expect(body.expiresAt).toBe(expiresAt);
    const wrong = await unlock(id, "hunter23");
    expect(wrong.status).toBe(403);
    expect(await wrong.json()).toEqual({ error: "wrong_password" });
  });

  it("a right password returns the receipt in the page's language", async () => {
    const { id } = await share({ password: "hunter22" });
    const body = await (await unlock(id, "hunter22", freshIp(), "it")).json<any>();
    expect(body.model).toEqual(modelOf("it"));
  });

  it("an odd language means English, not an error", async () => {
    const { id } = await share({ password: "hunter22" });
    // 2,000 characters: the body cap (4 KB) answers anything bigger with 413, as it always has.
    for (const locale of [undefined, "fr", "IT", "x".repeat(2000), 42, null, ["it"]]) {
      const res = await unlock(id, "hunter22", freshIp(), locale);
      expect(res.status, JSON.stringify(locale)).toBe(200);
      expect((await res.json<any>()).model.locale, JSON.stringify(locale)).toBe("en");
    }
  });

  it("a stored report the brain can't read is not found, never a 500", async () => {
    const id = newId("30d");
    const pw = await hashPassword("hunter22");
    await env.REPORTS.put(`30d/${id}`, JSON.stringify({ ...tiredReport(), collected_at: "2026-13-45T25:61:61Z" }), {
      customMetadata: { expiresAt: new Date(Date.now() + 86_400_000).toISOString(), deleteHash: "0".repeat(64), pwSalt: pw.salt, pwHash: pw.hash },
    });
    const res = await unlock(id, "hunter22");
    expect(res.status).toBe(404);
    expect(await res.json()).toEqual({ error: "not_found" });
  });

  it("answer 404 for unknown and expired reports", async () => {
    expect((await unlock(newId("30d"), "hunter22")).status).toBe(404);
    const id = newId("30d");
    const pw = await hashPassword("hunter22");
    await env.REPORTS.put(`30d/${id}`, JSON.stringify(tiredReport()), {
      customMetadata: { expiresAt: "2020-01-01T00:00:00.000Z", deleteHash: "0".repeat(64), pwSalt: pw.salt, pwHash: pw.hash },
    });
    expect((await unlock(id, "hunter22")).status).toBe(404);
  });

  it("cap guesses per report, whichever IPs they come from", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    await untilFreshWindow();
    for (let i = 0; i < 10; i++) expect((await unlock(id, `guess${i}`)).status).toBe(403);
    const limited = await unlock(id, "hunter22");
    expect(limited.status).toBe(429);
    expect(limited.headers.get("retry-after")).toBe("60");
    expect(await limited.json()).toEqual({ error: "rate_limited" });
  });

  it("don't count right passwords against the guess limit", async () => {
    const { id } = await share({ password: "hunter22" });
    for (let i = 0; i < 12; i++) expect((await unlock(id, "hunter22")).status, `unlock ${i}`).toBe(200);
  });

  it("cap a burst of parallel guesses at 10", async () => {
    const { id } = await share({ password: "hunter22" });
    await untilFreshWindow();
    const statuses = await Promise.all(Array.from({ length: 15 }, (_, i) => unlock(id, `guess${i}`).then((r) => r.status)));
    expect(statuses.filter((s) => s === 403).length).toBe(10);
    expect(statuses.filter((s) => s === 429).length).toBe(5);
  }, 30_000);

  it("cap unlock requests per IP, to bound the hashing cost", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    await untilFreshWindow();
    const ip = "203.0.113.50";
    for (let i = 0; i < 20; i++) expect((await unlock(id, "hunter22", ip)).status, `unlock ${i}`).toBe(200);
    expect((await unlock(id, "hunter22", ip)).status).toBe(429);
  });

  it("can still be deleted with just the delete token", async () => {
    const { id, deleteToken } = await share({ password: "hunter22" });
    const res = await call(`/api/reports/${id}`, { method: "DELETE", headers: { authorization: `Bearer ${deleteToken}` } });
    expect(res.status).toBe(204);
  });

  it("unlock only takes POST", async () => {
    const { id } = await share({ password: "hunter22" });
    const res = await call(`/api/reports/${id}/unlock`);
    expect(res.status).toBe(405);
    expect(res.headers.get("allow")).toBe("POST");
  });
});
