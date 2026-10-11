import { env } from "cloudflare:workers";
import staticHeaders from "../../apps/web/public/_headers?raw";
import { parseReport } from "@fettle/report-schema";
import { describe, expect, it } from "vitest";
import { newId } from "../src/ids.ts";
import { call, ORIGIN, share, tiredReport } from "./helpers.ts";

/** The share CSP: strict, with the shell's own inline stylesheet allowed by its hash. */
async function cspFor(html: string): Promise<string> {
  const css = /<style>([\s\S]*?)<\/style>/.exec(html)?.[1];
  if (css === undefined) throw new Error("the shell has no inline stylesheet");
  const js = /<script>([\s\S]*?)<\/script>/.exec(html)?.[1];
  if (js === undefined) throw new Error("the shell has no inline theme script");
  const sha = async (text: string) => btoa(String.fromCharCode(...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))));
  return `default-src 'self'; script-src 'self' 'sha256-${await sha(js)}'; style-src 'self' 'sha256-${await sha(css)}'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`;
}

/** What the server drew inside #root. */
function rootOf(html: string): string {
  return html.slice(html.indexOf('<div id="root">'), html.indexOf('<script id="fettle-share"'));
}

function payloadOf(html: string): any {
  const m = /<script id="fettle-share" type="application\/json">([\s\S]*?)<\/script>/.exec(html);
  if (!m) throw new Error("no payload script");
  return JSON.parse(m[1]!);
}

const meta = (html: string, property: string) =>
  new RegExp(`<meta property="${property}" content="([^"]*)">`).exec(html)?.[1];
const described = (html: string) => [...html.matchAll(/<meta name="description" content="([^"]*)">/g)].map((m) => m[1]);

async function expectShareHeaders(res: Response) {
  expect(res.headers.get("content-type")).toBe("text/html; charset=utf-8");
  expect(res.headers.get("content-security-policy")).toBe(await cspFor(await res.clone().text()));
  expect(res.headers.get("x-robots-tag")).toBe("noindex");
  expect(res.headers.get("referrer-policy")).toBe("no-referrer");
  expect(res.headers.get("x-content-type-options")).toBe("nosniff");
  // The same as the static site's (apps/web/public/_headers).
  expect(res.headers.get("permissions-policy")).toBe(/Permissions-Policy: (.+)/.exec(staticHeaders)?.[1]?.trim());
  expect(res.headers.get("cache-control")).toBe("no-store");
  expect(res.headers.get("etag")).toBeNull();
}

describe("GET /r/:id", () => {
  it("renders a live report with a preview card", async () => {
    const { id, expiresAt } = await share();
    const res = await call(`/r/${id}`);
    expect(res.status).toBe(200);
    await expectShareHeaders(res);
    const html = await res.text();
    expect(meta(html, "og:title")).toBe("Fettle receipt: Needs attention.");
    expect(meta(html, "og:description")).toMatch(/to act on soon/);
    // One plain description for search engines and link previews, the same text as the card.
    expect(described(html)).toEqual([meta(html, "og:description")]);
    expect(meta(html, "og:url")).toBe(`${ORIGIN}/r/${id}`);
    expect(meta(html, "og:type")).toBe("website");
    expect(meta(html, "og:site_name")).toBe("Fettle");
    expect(meta(html, "og:image")).toBe(`${ORIGIN}/og/share.png`);
    expect(meta(html, "og:image:width")).toBe("1200");
    expect(meta(html, "og:image:height")).toBe("630");
    expect(html).toContain('<meta name="twitter:card" content="summary_large_image">');
    expect(html).toContain("<title>Fettle receipt: Needs attention.</title>");
    // The receipt arrives already drawn, from a model the server built once (the browser reuses it).
    const payload = payloadOf(html);
    expect(payload).toMatchObject({ kind: "report", id, expiresAt, locale: "en" });
    expect(payload.report).toBeUndefined();
    expect(payload.model.summary).toBe("Needs attention.");
    const drawn = rootOf(html);
    expect(drawn).toContain('class="fettle-receipt"');
    expect(drawn).toContain("Needs attention.");
    expect(html, "no style attributes: the CSP would block them").not.toMatch(/\sstyle="/);
    void parseReport;
  });

  it("stamps the scan time in UTC and says so (the reader's zone is unknown until the page runs)", async () => {
    const report = tiredReport();
    report.collected_at = "2026-10-07T14:32:00+02:00";
    const { id } = await share({}, report);
    const html = await (await call(`/r/${id}`)).text();
    expect(rootOf(html)).toContain('class="when">07/10/2026 12:32 UTC<');
    expect(payloadOf(html).model.meta.when).toBe("07/10/2026 12:32 UTC");
  });

  it("picks the first language Fettle speaks from the whole Accept-Language list", async () => {
    const html = await (await call(`/r/${newId("30d")}`, { headers: { "accept-language": "de-DE,de;q=0.9,it;q=0.8,en;q=0.7" } })).text();
    expect(payloadOf(html).locale).toBe("it");
  });

  it("takes only whole language tags, so 'items' is no Italian", async () => {
    const html = await (await call(`/r/${newId("30d")}?lang=items`, { headers: { "accept-language": "items, en;q=0.5" } })).text();
    expect(payloadOf(html).locale).toBe("en");
    const notEnglish = await (await call(`/r/${newId("30d")}`, { headers: { "accept-language": "en;q=0, it;q=0.3" } })).text();
    expect(payloadOf(notEnglish).locale).toBe("it");
  });

  it("draws the receipt in the language asked for", async () => {
    const { id } = await share();
    const html = await (await call(`/r/${id}?lang=it`)).text();
    expect(html).toContain('<html lang="it">');
    expect(payloadOf(html).locale).toBe("it");
    expect(rootOf(html)).toContain("Ha bisogno di attenzioni.");
  });

  it("writes the preview in the reader's language", async () => {
    const { id } = await share();
    const html = await (await call(`/r/${id}`, { headers: { "accept-language": "it-IT,it;q=0.9" } })).text();
    expect(meta(html, "og:title")).toMatch(/^Scontrino Fettle: /);
  });

  it("keeps hostile report text inert", async () => {
    const report = tiredReport();
    const evil = "</script><script>alert(1)</script> \"><img src=x onerror=alert(2)>";
    report.facts.startup.data.items[0].name = evil;
    const { id } = await share({}, report);
    const html = await (await call(`/r/${id}`)).text();
    expect(html).not.toContain("</script><script>alert(1)");
    expect(html).not.toContain("<img src=x");
    expect(JSON.stringify(payloadOf(html).model)).toContain(JSON.stringify(evil).slice(1, -1));
  });

  it("gives nothing away for a password-protected report", async () => {
    const report = tiredReport();
    report.facts.startup.data.items[0].name = "SecretToolXYZ";
    const { id, expiresAt } = await share({ password: "hunter22" }, report);
    const res = await call(`/r/${id}`);
    expect(res.status).toBe(200);
    const html = await res.text();
    expect(payloadOf(html)).toEqual({ kind: "locked", id, expiresAt, locale: "en" });
    expect(meta(html, "og:title")).toBe("Fettle receipt (password protected)");
    expect(meta(html, "og:description")).toBe("Open the link and enter the password to see it.");
    expect(described(html)).toEqual(["Open the link and enter the password to see it."]);
    expect(meta(html, "og:image")).toBe(`${ORIGIN}/og/share.png`);
    expect(html).not.toContain("SecretToolXYZ");
    expect(html).not.toContain("Needs attention");
  });

  it("draws the 'isn't here' and password pages on the server too (readable without JavaScript)", async () => {
    const missing = await (await call(`/r/${newId("30d")}`)).text();
    expect(rootOf(missing)).toContain("This receipt isn");
    const { id } = await share({ password: "hunter22" });
    const locked = await (await call(`/r/${id}`)).text();
    expect(rootOf(locked)).toContain("This receipt is password protected.");
    expect(rootOf(locked)).toContain('type="password"');
  });

  it("leaves the delete page to the browser (its token is in the #fragment the server never sees)", async () => {
    const html = await (await call(`/r/${newId("30d")}/delete`)).text();
    expect(rootOf(html).replace(/<\/?div[^>]*>/g, "").trim()).toBe("");
  });

  it("says a missing report isn't here", async () => {
    const res = await call(`/r/${newId("30d")}`);
    expect(res.status).toBe(404);
    await expectShareHeaders(res);
    const html = await res.text();
    expect(payloadOf(html)).toEqual({ kind: "missing", locale: "en" });
    expect(meta(html, "og:title")).toBeUndefined();
  });

  it("treats an expired report as missing and removes it", async () => {
    const id = newId("30d");
    await env.REPORTS.put(`30d/${id}`, JSON.stringify(tiredReport()), {
      customMetadata: { expiresAt: "2020-01-01T00:00:00.000Z", deleteHash: "0".repeat(64) },
    });
    const res = await call(`/r/${id}`);
    expect(res.status).toBe(404);
    expect(payloadOf(await res.text())).toEqual({ kind: "missing", locale: "en" });
    expect(await env.REPORTS.head(`30d/${id}`)).toBeNull();
  });

  it("shows a stored report the verdicts can't read as missing, not as an error", async () => {
    const id = newId("30d");
    await env.REPORTS.put(`30d/${id}`, JSON.stringify({ ...tiredReport(), collected_at: "2026-13-45T25:61:61Z" }), {
      customMetadata: { expiresAt: "2099-01-01T00:00:00.000Z", deleteHash: "0".repeat(64) },
    });
    const res = await call(`/r/${id}`);
    expect(res.status).toBe(404);
    await expectShareHeaders(res);
    expect(payloadOf(await res.text())).toEqual({ kind: "missing", locale: "en" });
  });

  it("answers 404 for other paths and 405 for other methods", async () => {
    for (const path of ["/r/", "/r", `/r/${newId("30d")}/extra`, "/r/NOT-AN-ID"]) {
      const res = await call(path);
      expect(res.status, path).toBe(404);
      expect(payloadOf(await res.text()), path).toEqual({ kind: "missing", locale: "en" });
    }
    const post = await call(`/r/${newId("30d")}`, { method: "POST" });
    expect(post.status).toBe(405);
    expect(post.headers.get("allow")).toBe("GET, HEAD");
  });
});

describe("GET /r/:id/delete", () => {
  it("serves the delete page without reading the report", async () => {
    const id = newId("30d");
    const res = await call(`/r/${id}/delete`);
    expect(res.status).toBe(200);
    await expectShareHeaders(res);
    const html = await res.text();
    expect(payloadOf(html)).toEqual({ kind: "delete", id, locale: "en" });
    expect(meta(html, "og:title")).toBeUndefined();
  });
});
