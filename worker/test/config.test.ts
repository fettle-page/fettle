import { describe, expect, it } from "vitest";
import raw from "../wrangler.jsonc?raw";
import pkg from "../package.json";

// JSONC: drop full-line comments before parsing.
const config = JSON.parse(raw.replace(/^\s*\/\/.*$/gm, ""));

describe("wrangler config", () => {
  it("doesn't let Workers Logs record visitors' IPs and headers", () => {
    expect(config.observability.logs.invocation_logs).toBe(false);
  });

  it("keeps traces off: they record R2 keys, which are the share links' IDs", () => {
    expect(config.observability.traces.enabled).toBe(false);
  });

  it("serves only on fettle.page, not on a workers.dev address or preview URLs", () => {
    expect(config.routes).toEqual([{ pattern: "fettle.page", custom_domain: true }]);
    expect(config.workers_dev).toBe(false);
    expect(config.preview_urls).toBe(false);
  });

  it("always rebuilds the site before deploying, so a stale build can't go out", () => {
    expect((pkg.scripts as Record<string, string>).deploy).toBe("npm run build:share && wrangler deploy");
  });

  it("keeps shared receipts in the EU (the bucket was created in R2's EU jurisdiction)", () => {
    expect(config.r2_buckets).toEqual([{ binding: "REPORTS", bucket_name: "fettle-reports", jurisdiction: "eu" }]);
  });

  it("gives the rate limiters their own IDs, not the placeholders (IDs are shared across the whole account)", () => {
    const ids = config.ratelimits.map((r: { namespace_id: string }) => r.namespace_id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) {
      expect(id).toMatch(/^\d+$/);
      expect(["1001", "1002"]).not.toContain(id);
    }
  });
});
