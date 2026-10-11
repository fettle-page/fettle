import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import raw from "../wrangler.jsonc?raw";
import { ageDownloadCount, downloadCount, resetDownloadCount } from "../src/downloads.ts";
import { call } from "./helpers.ts";

// The download count, as a section of the main download button on the home and download pages: GitHub's own total for the installer and the
// zip of every published release, asked for at most once an hour, filled in by the Worker. Visitors'
// browsers never contact GitHub, and nothing about who downloaded is stored.
const release = (draft: boolean, assets: [string, number][]) => ({ draft, assets: assets.map(([name, download_count]) => ({ name, download_count })) });
const RELEASES = [
  release(false, [["Fettle-Setup-1.0.0.exe", 5], ["Fettle-1.0.0-portable.zip", 3], ["SHA256SUMS.txt", 3]]),
  release(true, [["Fettle-Setup-1.0.1.exe", 100]]), // a draft isn't out yet
];
const github = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
let fetchSpy: ReturnType<typeof vi.spyOn>;
const answer = (r: () => Response | Promise<Response>) => fetchSpy.mockImplementation(async () => r());
/** The count section: [whole tag, the number shown, the words for screen readers], or null. */
const line = (html: string) => {
  const m = /<span class="dl-count"([^>]*)>[\s\S]*?<span data-download-n[^>]*>([^<]*)<\/span><span class="sr-only" data-download-said>([^<]*)<\/span>/.exec(html);
  return m ? ([m[1]!, m[2]!, m[3]!] as const) : null;
};

beforeEach(() => {
  resetDownloadCount();
  fetchSpy = vi.spyOn(globalThis, "fetch");
  answer(() => github(RELEASES));
});
afterEach(() => fetchSpy.mockRestore());

describe("the download count", () => {
  it("counts the installer and the zip of published releases, not the checksums or drafts", async () => {
    expect(await downloadCount({} as Env)).toBe(8);
  });

  it("asks GitHub as fettle.page, with the read-only token when there is one", async () => {
    await downloadCount({ GITHUB_TOKEN: "t0ken" } as Env);
    const [url, init] = fetchSpy.mock.calls[0]! as [string, RequestInit];
    expect(url).toBe("https://api.github.com/repos/fettle-page/fettle/releases?per_page=100");
    const headers = new Headers(init.headers);
    expect(headers.get("user-agent")).toBe("fettle.page");
    expect(headers.get("authorization")).toBe("Bearer t0ken");
    resetDownloadCount();
    await downloadCount({} as Env);
    expect(new Headers((fetchSpy.mock.calls[1]! as [string, RequestInit])[1].headers).get("authorization")).toBeNull();
  });

  it("shows on the home page's download button, in each language", async () => {
    const en = line(await (await call("/")).text());
    expect(en![0]).not.toMatch(/\shidden(\s|$)/);
    expect(en![1]).toBe("8");
    expect(en![2]).toBe("8 downloads");
    expect(line(await (await call("/it")).text())![2]).toBe("8 download");
  });

  it("shows on the download pages, with each language's thousands separator", async () => {
    answer(() => github([release(false, [["Fettle-Setup-1.0.0.exe", 1000], ["Fettle-1.0.0-portable.zip", 234]])]));
    const en = line(await (await call("/download")).text())!;
    expect([en[1], en[2]]).toEqual(["1,234", "1,234 downloads"]);
    const it_ = line(await (await call("/it/download")).text())!;
    expect([it_[1], it_[2]]).toEqual(["1.234", "1.234 download"]);
  });

  it("says 1 download, not 1 downloads", async () => {
    answer(() => github([release(false, [["Fettle-Setup-1.0.0.exe", 1]])]));
    expect(line(await (await call("/")).text())!.slice(1)).toEqual(["1", "1 download"]);
    expect(line(await (await call("/it/download")).text())!.slice(1)).toEqual(["1", "1 download"]);
  });

  it("asks GitHub once, not on every visit", async () => {
    await call("/");
    await call("/it/download");
    await call("/download");
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it("shares one GitHub request between visitors who arrive together", async () => {
    answer(() => new Promise((r) => setTimeout(() => r(github(RELEASES)), 100)));
    await Promise.all([call("/"), call("/download"), call("/it")]);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  // GitHub's slow answers are timers made inside the Worker's own request: Workers forbid settling
  // one request's promise from another context, such as the test's.
  const slowly = (ms: number, body: unknown) => answer(() => new Promise<Response>((r) => setTimeout(() => r(github(body)), ms)));
  const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

  it("never keeps a page waiting on a slow GitHub", async () => {
    slowly(1500, RELEASES);
    const started = Date.now();
    const first = await call("/");
    expect(Date.now() - started).toBeLessThan(1200);
    expect(line(await first.text())![0]).toMatch(/\shidden(\s|$)/);
    await pause(1800);
    expect(line(await (await call("/")).text())!.slice(1)).toEqual(["8", "8 downloads"]);
  });

  it("shows an hour-old count straight away and refreshes it in the background", async () => {
    await call("/");
    ageDownloadCount();
    slowly(1500, [release(false, [["Fettle-Setup-1.0.0.exe", 20]])]);
    const started = Date.now();
    expect(line(await (await call("/")).text())!.slice(1)).toEqual(["8", "8 downloads"]);
    expect(Date.now() - started).toBeLessThan(1200);
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    await pause(1800);
    expect(line(await (await call("/")).text())!.slice(1)).toEqual(["20", "20 downloads"]);
  });

  it("leaves the line out when GitHub doesn't answer, and the page still works", async () => {
    for (const fail of [() => github({ message: "rate limited" }, 403), () => github({}, 500), () => { throw new TypeError("network"); }, () => github({ not: "a list" })]) {
      resetDownloadCount();
      answer(fail);
      const res = await call("/");
      expect(res.status).toBe(200);
      const l = line(await res.text());
      expect(l![0]).toMatch(/\shidden(\s|$)/);
      expect([l![1], l![2]]).toEqual(["", ""]);
    }
  });

  it("sends no ETag, so a browser can't keep showing an old count", async () => {
    const res = await call("/download");
    expect(res.headers.get("etag")).toBeNull();
    expect(res.headers.get("content-type")).toContain("text/html");
  });

  it("runs the Worker first only on the four pages that show it", () => {
    const config = JSON.parse(raw.replace(/^\s*\/\/.*$/gm, ""));
    expect(config.assets.run_worker_first).toEqual(["/api/*", "/r/*", "/", "/it", "/download", "/it/download"]);
  });
});
