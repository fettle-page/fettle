import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { newId } from "../src/ids.ts";
import { hashPassword } from "../src/passwords.ts";
import { dayKey, rateKey, unlockReport } from "../src/reports.ts";
import { call, freshIp, share, tiredReport, untilFreshWindow, upload } from "./helpers.ts";

describe("rateKey", () => {
  it("keeps IPv4 addresses as they are", () => {
    expect(rateKey("203.0.113.7")).toBe("203.0.113.7");
    expect(rateKey("unknown")).toBe("unknown");
    expect(rateKey("::ffff:203.0.113.7")).toBe("203.0.113.7");
  });

  it("groups IPv6 addresses by their /64, since one host usually owns the whole block", () => {
    expect(rateKey("2001:db8:1:2:aaaa::1")).toBe("2001:db8:1:2::/64");
    expect(rateKey("2001:0DB8:0001:0002:bbbb:cccc:dddd:eeee")).toBe("2001:db8:1:2::/64");
    expect(rateKey("2001:db8::1")).toBe("2001:db8:0:0::/64");
    expect(rateKey("::1")).toBe("0:0:0:0::/64");
  });
});

describe("dayKey", () => {
  it("keeps IPv4 addresses as they are", () => {
    expect(dayKey("203.0.113.7")).toBe("203.0.113.7");
    expect(dayKey("::ffff:203.0.113.7")).toBe("203.0.113.7");
  });

  // The daily caps count a whole /48: a free tunnel broker hands one person a /48 of 65,536 /64s.
  it("groups IPv6 addresses by their /48", () => {
    expect(dayKey("2001:db8:1:2:aaaa::1")).toBe("2001:db8:1::/48");
    expect(dayKey("2001:db8:1:ffff::2")).toBe("2001:db8:1::/48");
    expect(dayKey("2001:db8::1")).toBe("2001:db8:0::/48");
    expect(dayKey("2001:db8:2::1")).not.toBe(dayKey("2001:db8:1::1"));
  });
});

describe("upload limit with IPv6", () => {
  it("counts a whole /64 as one client", { timeout: 30_000 }, async () => {
    await untilFreshWindow();
    for (let i = 0; i < 10; i++) {
      expect((await upload({ report: tiredReport() }, { ip: `2001:db8:77:1::${(i + 1).toString(16)}` })).status).toBe(201);
    }
    expect((await upload({ report: tiredReport() }, { ip: "2001:db8:77:1:ffff::99" })).status).toBe(429);
    expect((await upload({ report: tiredReport() }, { ip: "2001:db8:77:2::1" })).status).toBe(201);
  });
});

describe("damaged objects", () => {
  it("read as missing, not as a server error", async () => {
    const id = newId("30d");
    await env.REPORTS.put(`30d/${id}`, "not json {", { customMetadata: { expiresAt: "2099-01-01T00:00:00.000Z", deleteHash: "0".repeat(64) } });
    expect((await call(`/api/reports/${id}`)).status).toBe(404);
    expect((await call(`/r/${id}`)).status).toBe(404);
  });
});

describe("unlock cost", () => {
  /** env with a REPORTS binding that counts body reads. */
  function counting() {
    const reads = { gets: 0 };
    const bucket = {
      head: (key: string) => env.REPORTS.head(key),
      get: (key: string) => {
        reads.gets++;
        return env.REPORTS.get(key);
      },
      put: (...args: Parameters<R2Bucket["put"]>) => env.REPORTS.put(...args),
      delete: (keys: string | string[]) => env.REPORTS.delete(keys),
    } as unknown as R2Bucket;
    return { env: { ...env, REPORTS: bucket } as Env, reads };
  }
  const unlockRequest = (id: string, password: string, ip = freshIp()) =>
    new Request(`http://example.com/api/reports/${id}/unlock`, {
      method: "POST",
      headers: { "content-type": "application/json", "cf-connecting-ip": ip },
      body: JSON.stringify({ password }),
    });

  it("doesn't read the report for a wrong or throttled guess", { timeout: 30_000 }, async () => {
    const { id } = await share({ password: "hunter22" });
    const { env: e, reads } = counting();
    await untilFreshWindow();
    expect((await unlockReport(unlockRequest(id, "nope-nope"), id, e)).status).toBe(403);
    for (let i = 0; i < 9; i++) await unlockReport(unlockRequest(id, `guess-${i}x`), id, e);
    expect((await unlockReport(unlockRequest(id, "hunter22"), id, e)).status).toBe(429);
    expect(reads.gets).toBe(0);
  });

  it("reads it once the password is right", async () => {
    const { id } = await share({ password: "hunter22" });
    const { env: e, reads } = counting();
    const res = await unlockReport(unlockRequest(id, "hunter22"), id, e);
    expect(res.status).toBe(200);
    expect(reads.gets).toBe(1);
  });

  it("still answers 404 for an expired locked report and removes it", async () => {
    const id = newId("30d");
    const pw = await hashPassword("hunter22");
    await env.REPORTS.put(`30d/${id}`, JSON.stringify(tiredReport()), {
      customMetadata: { expiresAt: "2020-01-01T00:00:00.000Z", deleteHash: "0".repeat(64), pwSalt: pw.salt, pwHash: pw.hash },
    });
    const { env: e } = counting();
    expect((await unlockReport(unlockRequest(id, "hunter22"), id, e)).status).toBe(404);
    expect(await env.REPORTS.head(`30d/${id}`)).toBeNull();
  });
});

describe("HEAD", () => {
  it("returns a share page's status and headers without a body", async () => {
    const { id } = await share();
    const res = await call(`/r/${id}`, { method: "HEAD" });
    expect(res.status).toBe(200);
    expect(res.headers.get("x-robots-tag")).toBe("noindex");
    expect(await res.text()).toBe("");
  });

  it("isn't an API method", async () => {
    const { id } = await share();
    const res = await call(`/api/reports/${id}`, { method: "HEAD" });
    expect(res.status).toBe(405);
  });
});
