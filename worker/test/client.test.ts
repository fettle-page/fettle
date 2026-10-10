// The app's share client, run against this Worker: if either side changes the contract, this fails.
import * as client from "@fettle/share-client";
import type { Report } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { MAX_UPLOAD_BYTES } from "../src/http.ts";
import { PASSWORD_MAX, PASSWORD_MIN } from "../src/passwords.ts";
import { call, freshIp, ORIGIN, tiredReport } from "./helpers.ts";

/** fetch, routed into the Worker from one fresh IP (so tests never share a rate-limit bucket). */
function viaWorker(): typeof fetch {
  const ip = freshIp();
  return (async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const headers = new Headers(init.headers);
    headers.set("cf-connecting-ip", ip);
    const url = new URL(String(input));
    return call(url.pathname, { ...init, headers });
  }) as typeof fetch;
}

const report = () => tiredReport() as Report;

describe("share client against the Worker", () => {
  it("uploads, opens and deletes a report", async () => {
    const result = await client.shareReport(ORIGIN, { report: report(), expires: "7d" }, viaWorker());
    if (!result.ok) throw new Error(result.error);
    const { shared } = result;
    expect(shared.url).toBe(`${ORIGIN}/r/${shared.id}`);
    expect(shared.locked).toBe(false);
    expect((await call(`/api/reports/${shared.id}`)).status).toBe(200);

    expect(await client.deleteShared(ORIGIN, shared, viaWorker())).toEqual({ ok: true });
    expect((await call(`/api/reports/${shared.id}`)).status).toBe(404);
    // Deleting again is still fine: the link is gone either way.
    expect(await client.deleteShared(ORIGIN, shared, viaWorker())).toEqual({ ok: true });
  });

  it("locks a report with a password", async () => {
    const result = await client.shareReport(ORIGIN, { report: report(), password: "hunter22" }, viaWorker());
    if (!result.ok) throw new Error(result.error);
    expect(result.shared.locked).toBe(true);
    const res = await call(`/api/reports/${result.shared.id}`);
    expect(await res.json()).toMatchObject({ error: "password_required" });
  });

  it("refuses a wrong delete token", async () => {
    const result = await client.shareReport(ORIGIN, { report: report() }, viaWorker());
    if (!result.ok) throw new Error(result.error);
    expect(await client.deleteShared(ORIGIN, { id: result.shared.id, deleteToken: "nope" }, viaWorker())).toEqual({ ok: false, error: "forbidden" });
  });

  it("passes on the Worker's verdict on a broken report", async () => {
    const broken = { ...report(), schema_version: 999 };
    const result = await client.shareReport(ORIGIN, { report: broken }, viaWorker());
    expect(result).toMatchObject({ ok: false, error: "invalid_report" });
  });

  it("uses the same limits as the Worker", () => {
    expect([client.PASSWORD_MIN, client.PASSWORD_MAX, client.MAX_UPLOAD_BYTES]).toEqual([PASSWORD_MIN, PASSWORD_MAX, MAX_UPLOAD_BYTES]);
  });
});
