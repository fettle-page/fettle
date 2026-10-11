import { describe, expect, it } from "vitest";
import raw from "../wrangler.jsonc?raw";
import { call } from "./helpers.ts";

describe("the website through the Worker", () => {
  it("serves the landing page", async () => {
    const res = await call("/");
    expect(res.status).toBe(200);
    expect(await res.text()).toContain("fine fettle");
  });

  it("shows the site's own 404 page for unknown paths", async () => {
    const res = await call("/nope");
    expect(res.status).toBe(404);
    expect(await res.text()).toContain("This page isn");
  });

  it("shows the Italian 404 for unknown paths under /it/", async () => {
    const res = await call("/it/nulla/qui");
    expect(res.status).toBe(404);
    const html = await res.text();
    expect(html).toMatch(/<html[^>]*lang="it"/);
    expect(html).not.toContain("This page isn");
  });
});

describe("addresses with a trailing slash or .html", () => {
  // Permanent (301), so search engines settle on the clean address; the static files alone would answer 307.
  it("redirect for good to the clean address, keeping the query", async () => {
    const cases: [string, string][] = [
      ["/it/", "/it"],
      ["/download/", "/download"],
      ["/download.html", "/download"],
      ["/it/faq.html", "/it/faq"],
      ["/index.html", "/"],
      ["/it/index.html", "/it"],
      ["/faq/?from=forum", "/faq?from=forum"],
    ];
    for (const [from, to] of cases) {
      const res = await call(from, { redirect: "manual" });
      expect(res.status, from).toBe(301);
      expect(new URL(res.headers.get("location")!).pathname + new URL(res.headers.get("location")!).search, from).toBe(to);
    }
  });

  // Never an open redirect: "//host/" and its look-alikes must stay on this site.
  it("never send a visitor to another site", async () => {
    // Backslashes as the browser would send them: "/\evil" and "\\evil" (doubled here for JavaScript).
    const tricks = ["//evil.example/", "/\\evil.example/", "\\\\evil.example/", "//evil.example/x.html", "///evil.example/", "/%2F%2Fevil.example/", "/%5Cevil.example/", "/%5C%5Cevil.example/x.html", "/.//evil.example/"];
    for (const trick of tricks) {
      const res = await call(trick, { redirect: "manual" });
      const location = res.headers.get("location");
      if (location !== null) expect(new URL(location, "http://example.com/").origin, trick).toBe("http://example.com");
    }
  });

  it("clean a doubled slash to a path on this site", async () => {
    const res = await call("//download/", { redirect: "manual" });
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("http://example.com/download");
  });

  it("run the Worker first, so it can answer before the static files do", () => {
    const config = JSON.parse(raw.replace(/^\s*\/\/.*$/gm, ""));
    expect(config.assets.run_worker_first).toEqual(expect.arrayContaining(["/*/", "/*.html"]));
  });
});
