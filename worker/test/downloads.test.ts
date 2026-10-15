import { env } from "cloudflare:workers";
import { createScheduledController } from "cloudflare:test";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import raw from "../wrangler.jsonc?raw";
import worker from "../src/index.ts";
import { ageDownloadCount, COUNT_KEY, downloadCount, refreshDownloadCount, resetDownloadCount } from "../src/downloads.ts";
import { keyFor, newId } from "../src/ids.ts";
import { call } from "./helpers.ts";

// The download count, as a section of the main download button on the home and download pages: GitHub's own total for the installer and the
// zip of every published release. The Worker asks GitHub once an hour on a schedule and keeps the answer in R2, and pages read
// it from there, so no visitor's page waits on GitHub. Visitors' browsers never contact GitHub, and nothing about who downloaded is stored.
const release = (draft: boolean, assets: [string, number][]) => ({ draft, assets: assets.map(([name, download_count]) => ({ name, download_count })) });
const RELEASES = [
  release(false, [["Fettle-Setup-1.0.0.exe", 5], ["Fettle-1.0.0-portable.zip", 3], ["SHA256SUMS.txt", 3]]),
  release(true, [["Fettle-Setup-1.0.1.exe", 100]]), // a draft isn't out yet
];
const github = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
let fetchSpy: ReturnType<typeof vi.spyOn>;
const answer = (r: () => Response | Promise<Response>) => fetchSpy.mockImplementation(async () => r());
const askedGitHub = () => fetchSpy.mock.calls.filter(([u]: unknown[]) => String(u).startsWith("https://api.github.com/")).length;
/** The count section: [whole tag, the number shown, the words for screen readers], or null. */
const line = (html: string) => {
  const m = /<span class="dl-count"([^>]*)>[\s\S]*?<span data-download-n[^>]*>([^<]*)<\/span><span class="sr-only" data-download-said>([^<]*)<\/span>/.exec(html);
  return m ? ([m[1]!, m[2]!, m[3]!] as const) : null;
};
const shown = async (path: string) => line(await (await call(path)).text())!.slice(1);
const hidden = (l: ReturnType<typeof line>) => /\shidden(\s|$)/.test(l![0]);
/** What the hourly run would have stored. */
const stored = (count: number) => env.REPORTS.put(COUNT_KEY, JSON.stringify({ count, at: new Date().toISOString() }));
const storedCount = async () => {
  const o = await env.REPORTS.get(COUNT_KEY);
  return o ? ((await o.json()) as { count: number }).count : null;
};
/** Runs the Worker's hourly job, as Cloudflare's Cron Trigger would. */
async function hourly(): Promise<void> {
  await worker.scheduled(createScheduledController({ cron: "17 * * * *", scheduledTime: Date.now() }), env);
}
// GitHub's slow answers are timers made inside the Worker's own request: Workers forbid settling
// one request's promise from another context, such as the test's.
const slowly = (ms: number, body: unknown) => answer(() => new Promise<Response>((r) => setTimeout(() => r(github(body)), ms)));
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

beforeEach(async () => {
  resetDownloadCount();
  await env.REPORTS.delete(COUNT_KEY);
  fetchSpy = vi.spyOn(globalThis, "fetch");
  answer(() => github(RELEASES));
});
afterEach(() => fetchSpy.mockRestore());

describe("the download count", () => {
  it("counts the installer and the zip of published releases, not the checksums or drafts", async () => {
    await refreshDownloadCount(env);
    expect(await storedCount()).toBe(8);
  });

  it("asks GitHub as fettle.page, with the read-only token when there is one", async () => {
    await refreshDownloadCount({ ...env, GITHUB_TOKEN: "t0ken" });
    const [url, init] = fetchSpy.mock.calls[0]! as [string, RequestInit];
    expect(url).toBe("https://api.github.com/repos/fettle-page/fettle/releases?per_page=100");
    const headers = new Headers(init.headers);
    expect(headers.get("user-agent")).toBe("fettle.page");
    expect(headers.get("authorization")).toBe("Bearer t0ken");
    await refreshDownloadCount({ ...env, GITHUB_TOKEN: undefined });
    expect(new Headers((fetchSpy.mock.calls[1]! as [string, RequestInit])[1].headers).get("authorization")).toBeNull();
  });

  it("shows on the home page's download button, in each language", async () => {
    await stored(8);
    const en = line(await (await call("/")).text());
    expect(hidden(en)).toBe(false);
    expect(en![1]).toBe("8");
    expect(en![2]).toBe("8 downloads");
    expect(line(await (await call("/it")).text())![2]).toBe("8 download");
  });

  it("shows on the download pages, with each language's thousands separator", async () => {
    await stored(1234);
    expect(await shown("/download")).toEqual(["1,234", "1,234 downloads"]);
    expect(await shown("/it/download")).toEqual(["1.234", "1.234 download"]);
  });

  it("says 1 download, not 1 downloads", async () => {
    await stored(1);
    expect(await shown("/")).toEqual(["1", "1 download"]);
    expect(await shown("/it/download")).toEqual(["1", "1 download"]);
  });

  it("shows the count on a fresh isolate's first page, even when GitHub is slow", async () => {
    await stored(8);
    slowly(1500, RELEASES);
    const started = Date.now();
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    expect(Date.now() - started).toBeLessThan(1200);
    expect(askedGitHub()).toBe(0);
  });

  it("never asks GitHub on a visitor's request once the count is stored", async () => {
    await stored(8);
    await call("/");
    await call("/it/download");
    resetDownloadCount();
    await call("/download");
    expect(askedGitHub()).toBe(0);
  });

  it("keeps the count in memory, and reads it again in the background when it's old", async () => {
    await stored(8);
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    await stored(20);
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    ageDownloadCount();
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    await vi.waitFor(async () => expect(await shown("/")).toEqual(["20", "20 downloads"]));
  });

  it("keeps the count in memory when the stored one goes missing", async () => {
    await stored(8);
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    await env.REPORTS.delete(COUNT_KEY);
    ageDownloadCount();
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
    await pause(100);
    expect(await shown("/download")).toEqual(["8", "8 downloads"]);
    expect(askedGitHub()).toBe(0);
  });

  it("the hourly run asks GitHub and stores the count for every isolate", async () => {
    await hourly();
    expect(askedGitHub()).toBe(1);
    expect(await storedCount()).toBe(8);
    resetDownloadCount();
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
  });

  it("the hourly run keeps the last count when GitHub fails, and is marked failed", async () => {
    await stored(5);
    answer(() => github({ message: "bad credentials" }, 401));
    await expect(hourly()).rejects.toThrow(/GitHub/);
    expect(await storedCount()).toBe(5);
  });

  it("is set to run hourly", () => {
    const config = JSON.parse(raw.replace(/^\s*\/\/.*$/gm, ""));
    expect(config.triggers).toEqual({ crons: ["17 * * * *"] });
  });

  it("keeps the count where no report or lifecycle rule can reach it", () => {
    // Reports live under 7d/, 30d/ and 1y/, and only those prefixes have lifecycle rules (README).
    expect(COUNT_KEY).toMatch(/^site\//);
    for (const ttl of ["7d", "30d", "1y"] as const) {
      for (let i = 0; i < 50; i++) expect(keyFor(newId(ttl))).toMatch(new RegExp(`^${ttl}/`));
    }
  });

  it("before the first hourly run, the first visitor's page doesn't wait and the count is fetched in the background", async () => {
    slowly(1500, RELEASES);
    const started = Date.now();
    const first = line(await (await call("/")).text());
    expect(Date.now() - started).toBeLessThan(1200);
    expect(hidden(first)).toBe(true);
    await vi.waitFor(async () => expect(await storedCount()).toBe(8), { timeout: 5000 });
    resetDownloadCount();
    expect(await shown("/")).toEqual(["8", "8 downloads"]);
  });

  it("before the first hourly run, visitors who arrive together ask GitHub once", async () => {
    slowly(300, RELEASES);
    await Promise.all([call("/"), call("/download"), call("/it")]);
    await pause(600);
    expect(askedGitHub()).toBe(1);
  });

  it("leaves the line out when nothing is stored and GitHub doesn't answer, and the page still works", async () => {
    for (const fail of [() => github({ message: "rate limited" }, 403), () => github({}, 500), () => { throw new TypeError("network"); }, () => github({ not: "a list" })]) {
      resetDownloadCount();
      answer(fail);
      const res = await call("/");
      expect(res.status).toBe(200);
      const l = line(await res.text());
      expect(hidden(l)).toBe(true);
      expect([l![1], l![2]]).toEqual(["", ""]);
      await pause(50);
      expect(await storedCount()).toBeNull();
    }
  });

  it("after a failed ask, waits before asking GitHub again", async () => {
    answer(() => github({}, 500));
    await call("/");
    await pause(50);
    await call("/download");
    await pause(50);
    expect(askedGitHub()).toBe(1);
  });

  it("leaves the line out when the stored count is unreadable", async () => {
    answer(() => github({}, 500));
    await env.REPORTS.put(COUNT_KEY, "not json {");
    const res = await call("/");
    expect(res.status).toBe(200);
    expect(hidden(line(await res.text()))).toBe(true);
  });

  it("gives up on a slow read of the stored count rather than keep the page waiting", async () => {
    const slowBucket = { get: () => new Promise((r) => setTimeout(() => r(null), 3000)) } as unknown as R2Bucket;
    const started = Date.now();
    expect(await downloadCount({ ...env, REPORTS: slowBucket })).toBeNull();
    expect(Date.now() - started).toBeLessThan(1500);
  });

  it("sends no ETag, so a browser can't keep showing an old count", async () => {
    await stored(8);
    const res = await call("/download");
    expect(res.headers.get("etag")).toBeNull();
    expect(res.headers.get("content-type")).toContain("text/html");
  });

  it("runs the Worker first only on the four pages that show it (and the API, share pages and redirects)", () => {
    const config = JSON.parse(raw.replace(/^\s*\/\/.*$/gm, ""));
    expect(config.assets.run_worker_first).toEqual(["/api/*", "/r/*", "/", "/it", "/download", "/it/download", "/*/", "/*.html"]);
  });
});
