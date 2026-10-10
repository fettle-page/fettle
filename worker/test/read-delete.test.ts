import { env } from "cloudflare:workers";
import { parseReport } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { newId } from "../src/ids.ts";
import { hashToken, newDeleteToken } from "../src/tokens.ts";
import { call, share, tiredReport } from "./helpers.ts";

const bearer = (t: string) => ({ authorization: `Bearer ${t}` });

/** Stores a report directly in R2 with an expiry that has already passed. */
async function expired(token = newDeleteToken()) {
  const id = newId("30d");
  const key = `30d/${id}`;
  await env.REPORTS.put(key, JSON.stringify(tiredReport()), {
    customMetadata: { expiresAt: "2020-01-01T00:00:00.000Z", deleteHash: await hashToken(token) },
  });
  return { id, key, token };
}

describe("GET /api/reports/:id", () => {
  it("returns the stored report and its expiry", async () => {
    const created = await share();
    const res = await call(`/api/reports/${created.id}`);
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe("no-store");
    const body = await res.json<any>();
    const parsed = parseReport(tiredReport());
    expect(parsed.ok && body.report).toEqual(parsed.ok && parsed.report);
    expect(body.expiresAt).toBe(created.expiresAt);
  });

  it("answers 404 for unknown and malformed ids", async () => {
    const unknown = await call(`/api/reports/${newId("30d")}`);
    expect(unknown.status).toBe(404);
    expect(await unknown.json()).toEqual({ error: "not_found" });
    expect((await call("/api/reports/NOT-AN-ID")).status).toBe(404);
  });

  it("hides and removes a report past its expiry, before R2's own clean-up", async () => {
    const { id, key } = await expired();
    expect((await call(`/api/reports/${id}`)).status).toBe(404);
    expect(await env.REPORTS.head(key)).toBeNull();
  });
});

describe("DELETE /api/reports/:id", () => {
  it("needs the delete token", async () => {
    const { id, deleteToken } = await share();
    const none = await call(`/api/reports/${id}`, { method: "DELETE" });
    expect(none.status).toBe(401);
    expect(await none.json()).toEqual({ error: "unauthorized" });
    const wrong = await call(`/api/reports/${id}`, { method: "DELETE", headers: bearer("wrong") });
    expect(wrong.status).toBe(403);
    expect(await wrong.json()).toEqual({ error: "forbidden" });
    expect(await env.REPORTS.head(`30d/${id}`)).not.toBeNull();

    const ok = await call(`/api/reports/${id}`, { method: "DELETE", headers: bearer(deleteToken) });
    expect(ok.status).toBe(204);
    expect((await call(`/api/reports/${id}`)).status).toBe(404);
    expect(await env.REPORTS.head(`30d/${id}`)).toBeNull();
    expect((await call(`/api/reports/${id}`, { method: "DELETE", headers: bearer(deleteToken) })).status).toBe(404);
  });

  it("treats an expired report as gone", async () => {
    const { id, token } = await expired();
    expect((await call(`/api/reports/${id}`, { method: "DELETE", headers: bearer(token) })).status).toBe(404);
  });

  it("only takes GET and DELETE", async () => {
    const { id } = await share();
    const res = await call(`/api/reports/${id}`, { method: "PATCH" });
    expect(res.status).toBe(405);
    expect(res.headers.get("allow")).toBe("GET, DELETE");
  });
});
