import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import { newId } from "../src/ids.ts";
import { hashPassword, hashPasswordV1, passwordMatches } from "../src/passwords.ts";
import { createReport, unlockReport } from "../src/reports.ts";
import { freshIp, ORIGIN, share, tiredReport, untilFreshWindow } from "./helpers.ts";

// Share passwords since 1.0.4: hashed with the PASSWORD_PEPPER secret (version 2). Links made before
// keep their old hash (no version) and still open. Without the secret, nothing that needs it works.
const PEPPER = (env as Env).PASSWORD_PEPPER!;
const withoutPepper = new Proxy(env, { get: (target, key) => (key === "PASSWORD_PEPPER" ? undefined : Reflect.get(target, key)) });

const post = (path: string, body: unknown, ip = freshIp()) =>
  new Request(ORIGIN + path, { method: "POST", headers: { "content-type": "application/json", "cf-connecting-ip": ip }, body: JSON.stringify(body) });
const unlock = (id: string, body: unknown, on: Env = env) => unlockReport(post(`/api/reports/${id}/unlock`, body), id, on);

/** Stores the tired report under a new id with the given password metadata. */
async function stored(pw: { salt: string; hash: string }, pwV?: string): Promise<string> {
  const id = newId("30d");
  const meta: Record<string, string> = { expiresAt: new Date(Date.now() + 86_400_000).toISOString(), deleteHash: "0".repeat(64), pwSalt: pw.salt, pwHash: pw.hash };
  if (pwV !== undefined) meta.pwV = pwV;
  await env.REPORTS.put(`30d/${id}`, JSON.stringify(tiredReport()), { customMetadata: meta });
  return id;
}

/** Runs `fn` and returns what it logged with console.error. */
async function errorsOf<T>(fn: () => Promise<T>): Promise<{ result: T; errors: string }> {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => void lines.push(args.map(String).join(" ")));
  try {
    return { result: await fn(), errors: lines.join("\n") };
  } finally {
    spy.mockRestore();
  }
}

describe("the password pepper", () => {
  it("is set for the tests, and isn't the address pepper", () => {
    expect(PEPPER.length).toBeGreaterThanOrEqual(16);
    expect(PEPPER).not.toBe((env as Env).ADDRESS_PEPPER);
  });

  it("goes into every new password hash, marked version 2", async () => {
    const { id } = await share({ password: "hunter22" });
    const meta = (await env.REPORTS.head(`30d/${id}`))!.customMetadata!;
    expect(meta.pwV).toBe("2");
    const kept = { salt: meta.pwSalt!, hash: meta.pwHash! };
    expect(await passwordMatches("hunter22", { ...kept, version: "2" }, PEPPER)).toBe(true);
    // Not the old scheme: without the pepper the hash can't be checked.
    expect(await passwordMatches("hunter22", kept, undefined)).toBe(false);
  });

  it("opens a version 2 link with the right password only", async () => {
    const id = await stored(await hashPassword("hunter22", PEPPER), "2");
    expect((await unlock(id, { password: "hunter22" })).status).toBe(200);
    const wrong = await unlock(id, { password: "hunter23" });
    expect(wrong.status).toBe(403);
    expect(await wrong.json()).toEqual({ error: "wrong_password" });
  });

  it("still opens a link made before 1.0.4, with or without the secret", async () => {
    const id = await stored(await hashPasswordV1("hunter22"));
    expect((await unlock(id, { password: "hunter22" })).status).toBe(200);
    expect((await unlock(id, { password: "hunter22" }, withoutPepper)).status).toBe(200);
    expect((await unlock(id, { password: "hunter23" }, withoutPepper)).status).toBe(403);
  });

  it("refuses a password version it doesn't know, even with the right password", async () => {
    const v2 = await hashPassword("hunter22", PEPPER);
    const v1 = await hashPasswordV1("hunter22");
    for (const [pw, pwV] of [[v2, "3"], [v2, "1"], [v2, ""], [v1, "2x"]] as const) {
      const id = await stored(pw, pwV);
      const { result: res, errors } = await errorsOf(() => unlock(id, { password: "hunter22" }));
      expect(res.status, JSON.stringify(pwV)).toBe(404);
      expect(await res.json()).toEqual({ error: "not_found" });
      expect(errors).toContain("password_version_unknown");
    }
  });
});

describe("without the PASSWORD_PEPPER secret", () => {
  it("refuses a password-protected upload, loudly, before it counts against the day", async () => {
    const request = post("/api/reports", { password: "hunter22", report: tiredReport() });
    const { result: res, errors } = await errorsOf(() => createReport(request, withoutPepper));
    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({ error: "unavailable" });
    expect(errors).toContain("password_pepper_missing");
    expect(errors).toContain("PASSWORD_PEPPER");
  });

  it("still takes uploads without a password", async () => {
    const res = await createReport(post("/api/reports", { report: tiredReport() }), withoutPepper);
    expect(res.status).toBe(201);
    expect((await res.json<{ locked: boolean }>()).locked).toBe(false);
  });

  it("refuses to check a version 2 password, loudly, without counting it as a guess", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    await untilFreshWindow();
    // More than the 10 guesses a minute a link allows: none of them may count, as no password was checked.
    for (let i = 0; i < 12; i++) {
      const { result: res, errors } = await errorsOf(() => unlock(id, { password: "hunter22" }, withoutPepper));
      expect(res.status).toBe(503);
      expect(await res.json()).toEqual({ error: "unavailable" });
      expect(errors).toContain("password_pepper_missing");
    }
    expect((await unlock(id, { password: "hunter22" })).status).toBe(200);
  });

  it("still opens a version 2 link with a pass from an earlier right password", async () => {
    const { id } = await share({ password: "hunter22" });
    const { pass } = await (await unlock(id, { password: "hunter22" })).json<{ pass: string }>();
    expect((await unlock(id, { pass }, withoutPepper)).status).toBe(200);
  });
});
