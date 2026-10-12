import { env } from "cloudflare:workers";
import { parseReport } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { freshIp, share, tiredReport, upload } from "./helpers.ts";

// The app removes private details before it uploads. The server runs the same name-independent pass
// again over every upload before storing it, so a report from an old or modified app can't leave an
// email address, an IP, an account SID, a server name or a profile folder in a share link.
/** The stored report, as R2 holds it (uploads default to 30 days). */
const stored = async (id: string) => (await env.REPORTS.get(`30d/${id}`))!.text();

describe("the server's own privacy pass", () => {
  it("stores an upload with private details removed", async () => {
    const report = tiredReport();
    report.facts.startup.data.items = [
      { name: "OneDrive - Contoso (john.smith@contoso.com)", location: "run_key", enabled: true },
      { name: "Sync tool for 192.0.2.44", location: "run_key", enabled: true },
    ];
    report.facts.memory = {
      check: { status: "failed", detail: "Access denied for S-1-5-21-1111111111-2222222222-3333333333-1111 reading \\\\fileserver01\\share\\C:\\Users\\jsmith\\x" },
    };
    const { id } = await share({}, report);
    const text = await stored(id);

    for (const secret of ["john.smith@contoso.com", "192.0.2.44", "S-1-5-21-1111111111-2222222222-3333333333-1111", "fileserver01", "jsmith"]) {
      expect(text, secret).not.toContain(secret);
    }
    for (const placeholder of ["<email>", "<ip>", "<sid>", "<server>", "<user>"]) expect(text, placeholder).toContain(placeholder);
    // Still a whole, valid report.
    expect(parseReport(JSON.parse(text)).ok).toBe(true);
  });

  // Since 1.0.4 a failed check says why with a code, never Windows' text.
  it("keeps the code a 1.0.4 report gives for a failed check", async () => {
    const report = tiredReport();
    report.facts.memory = { check: { status: "failed", code: "timed_out" } };
    const { id } = await share({}, report);
    const memory = JSON.parse(await stored(id)).facts.memory;
    expect(memory.check.status).toBe("failed");
    expect(memory.check.code).toBe("timed_out");
    expect(memory.check.detail).toBeUndefined();
  });

  it("refuses a code it doesn't know (text can't ride in it)", async () => {
    const report = tiredReport();
    report.facts.memory = { check: { status: "failed", code: "denied for jsmith" } };
    const res = await upload({ report }, { ip: freshIp() });
    expect(res.status).toBe(400);
    expect((await res.json<any>()).error).toBe("invalid_report");
  });

  // Reports from Fettle 1.0.3 and earlier carry the error's text and no code: still accepted, still cleaned.
  it("still takes a 1.0.3 report's error text, cleaned", async () => {
    const report = tiredReport();
    report.facts.memory = { check: { status: "failed", detail: "no reply from john.smith@example.com at 10.0.0.7" } };
    const { id } = await share({}, report);
    const check = JSON.parse(await stored(id)).facts.memory.check;
    expect(check.detail).toBe("no reply from <email> at <ip>");
    expect(check.code ?? null).toBeNull();
  });

  it("leaves a report with nothing private exactly as it was", async () => {
    const report = tiredReport();
    const parsed = parseReport(report);
    if (!parsed.ok) throw new Error(parsed.error);
    const { id } = await share({}, report);
    expect(JSON.parse(await stored(id))).toEqual(parsed.report);
  });
});
