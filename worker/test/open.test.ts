import { describe, expect, it } from "vitest";
import staticHeaders from "../../apps/web/public/_headers?raw";
import { call } from "./helpers.ts";

// /open compiles the Rust receipt in the visitor's browser, which a CSP without 'wasm-unsafe-eval'
// blocks. Only that page gets it: every other page keeps the static site's policy.
const csp = async (path: string) => {
  const res = await call(path);
  expect(res.status, path).toBe(200);
  await res.text();
  return res.headers.get("content-security-policy") ?? "";
};
const scriptSrc = (policy: string) => /script-src ([^;]+)/.exec(policy)?.[1] ?? "";

describe("the open-a-report page's CSP", () => {
  it("allows compiling WebAssembly on /open and /it/open", async () => {
    for (const path of ["/open", "/it/open"]) {
      const policy = await csp(path);
      expect(scriptSrc(policy), path).toMatch(/^'self' 'wasm-unsafe-eval'( 'sha256-[A-Za-z0-9+/=]+')*$/);
      expect(policy, path).not.toContain("unsafe-inline");
      expect(policy, path).not.toMatch(/'unsafe-eval'/);
      // One policy, not two joined by a comma (the stricter would still block it).
      expect(policy, path).not.toContain(",");
    }
  });

  it("keeps the rest of the policy the static site's", async () => {
    const site = await csp("/faq");
    expect((await csp("/open")).replace(" 'wasm-unsafe-eval'", "")).toBe(site);
    expect(site).toContain(/Content-Security-Policy: default-src 'self';/.exec(staticHeaders)![0].slice("Content-Security-Policy: ".length));
  });

  it("doesn't allow it anywhere else", async () => {
    for (const path of ["/", "/it", "/faq", "/download", "/privacy"]) expect(await csp(path), path).not.toContain("wasm-unsafe-eval");
  });

  it("keeps the page's other security headers", async () => {
    const res = await call("/open");
    await res.text();
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    expect(res.headers.get("permissions-policy")).toBe(/Permissions-Policy: (.+)/.exec(staticHeaders)?.[1]?.trim());
  });
});
