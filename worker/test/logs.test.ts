import { afterEach, describe, expect, it, vi } from "vitest";
import { logRef } from "../src/reports.ts";
import { call, tiredReport, upload } from "./helpers.ts";

const IP = "203.0.113.77";
const post = (path: string, body: unknown) =>
  call(path, { method: "POST", headers: { "content-type": "application/json", "cf-connecting-ip": IP }, body: JSON.stringify(body) });

afterEach(() => vi.restoreAllMocks());

describe("the Worker's logs", () => {
  // A whole link's life: upload, a wrong and a right password, a pass, the page, and delete.
  it("never hold a report ID, an IP address, a token, a password or a pass", async () => {
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((...args: unknown[]) => void lines.push(args.map(String).join(" ")));

    const created = await upload({ report: tiredReport(), password: "hunter22-secret" }, { ip: IP });
    expect(created.status).toBe(201);
    const { id, deleteToken } = await created.json<{ id: string; deleteToken: string }>();
    expect((await post(`/api/reports/${id}/unlock`, { password: "wrong-guess-1" })).status).toBe(403);
    const opened = await post(`/api/reports/${id}/unlock`, { password: "hunter22-secret" });
    const { pass } = await opened.json<{ pass: string }>();
    expect((await post(`/api/reports/${id}/unlock`, { pass })).status).toBe(200);
    await call(`/r/${id}`, { headers: { "cf-connecting-ip": IP } });
    const deleted = await call(`/api/reports/${id}`, { method: "DELETE", headers: { authorization: `Bearer ${deleteToken}`, "cf-connecting-ip": IP } });
    expect(deleted.status).toBe(204);

    const all = lines.join("\n");
    expect(lines.length).toBeGreaterThanOrEqual(4);
    for (const secret of [id, IP, deleteToken, pass, "hunter22-secret", "wrong-guess-1"]) expect(all, secret).not.toContain(secret);
    // Each line still says which link, as a short code made from its ID.
    expect(all).toContain(await logRef(id));
  });

  it("refer to a report by a short code that isn't its ID", async () => {
    const ref = await logRef("7d-AbCdEfGhIjKlMnOp");
    expect(ref).toMatch(/^[0-9a-f]{12}$/);
    expect(await logRef("7d-AbCdEfGhIjKlMnOp")).toBe(ref);
    expect(await logRef("7d-AbCdEfGhIjKlMnOq")).not.toBe(ref);
  });
});
