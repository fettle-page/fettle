// The download count, as a section of the main download button on the home and download pages: GitHub's own total for the installer and the zip
// of every published release, written into the page's empty line, so visitors' browsers never contact GitHub and nothing about who
// downloaded is kept.
// The Worker asks GitHub once an hour on a schedule (wrangler.jsonc's Cron Trigger) and keeps the answer in one small R2 object, which
// every isolate in every data centre can read. A page reads the count from its isolate's memory, or else from that object, and never
// waits on GitHub: Cloudflare starts fresh isolates all the time, and GitHub often takes longer to answer than a page should wait.
// Before the first hourly run (or if someone deletes it) the object is missing; then the first page that finds nothing asks GitHub in
// the background (only one request at a time per isolate: Workers can't share one request's promise with another) and stores the answer.
import { release } from "@fettle/web/release";

/** The pages with a download count (apps/web DownloadCount.astro); wrangler.jsonc runs the Worker first on them. */
export const DOWNLOAD_PAGES = new Set(["/", "/it", "/download", "/it/download"]);

/** The stored count, in the reports bucket but outside the 7d/, 30d/ and 1y/ prefixes, which reports and lifecycle rules use. */
export const COUNT_KEY = "site/download-count.json";

/** How long an isolate uses the count it read before reading the stored one again (in the background). */
const FRESH = 10 * 60_000;
/** The longest a page waits for the stored count, when the isolate has none yet. */
const READ_WAIT = 1000;
/** After a failed ask, try again in five minutes rather than on the next visit. */
const RETRY = 5 * 60_000;
/** The longest GitHub gets to answer. */
const GITHUB_WAIT = 10_000;
/** Taken as abandoned after this long (the request doing it may have been cancelled): longer than GitHub's wait plus the write. */
const BUSY_MAX = GITHUB_WAIT + 5_000;
type Known = { count: number; at: number };
let known: Known | undefined;
/** When a request started reading the stored count in the background, or 0 when none is. */
let readingAt = 0;
/** When a request started asking GitHub, or 0 when none is. */
let askedAt = 0;
/** When asking GitHub last failed, or 0. */
let failedAt = 0;

/** For tests: forget the remembered count. */
export function resetDownloadCount(): void {
  known = undefined;
  readingAt = askedAt = failedAt = 0;
}

/** For tests: make the remembered count old enough to be read again. */
export function ageDownloadCount(): void {
  if (known) known.at -= FRESH;
}

const busy = (since: number) => since !== 0 && Date.now() - since < BUSY_MAX;

/**
 * The stored count: a number, null when nothing is stored, or undefined when it couldn't be read
 * (an unreadable object counts as nothing stored, so it gets replaced).
 */
async function readStored(env: Env): Promise<number | null | undefined> {
  try {
    const object = await env.REPORTS.get(COUNT_KEY);
    if (!object) return null;
    const { count } = (await object.json()) as { count?: unknown };
    return typeof count === "number" && Number.isSafeInteger(count) && count >= 0 ? count : null;
  } catch (e) {
    if (e instanceof SyntaxError) return null;
    console.log(JSON.stringify({ event: "download_count_read_failed", message: String(e).slice(0, 120) }));
    return undefined;
  }
}

function remember(count: number | null | undefined): void {
  if (typeof count === "number") known = { count, at: Date.now() };
}

/**
 * Asks GitHub and stores the count for every isolate. Run hourly by the Cron Trigger, and by a page
 * that found nothing stored. When GitHub fails, the stored count stays as it was and this says false.
 */
export async function refreshDownloadCount(env: Env): Promise<boolean> {
  const count = await askGitHub(env);
  if (count === null) {
    failedAt = Date.now();
    return false;
  }
  remember(count);
  await env.REPORTS.put(COUNT_KEY, JSON.stringify({ count, at: new Date().toISOString() }), { httpMetadata: { contentType: "application/json" } });
  return true;
}

function releasesUrl(): string | null {
  const m = release.repo ? /^https:\/\/github\.com\/([^/]+)\/([^/]+)$/.exec(release.repo) : null;
  return m ? `https://api.github.com/repos/${m[1]}/${m[2]}/releases?per_page=100` : null;
}

/** Installer and zip downloads of the published releases, or null when GitHub doesn't say. */
async function askGitHub(env: Env): Promise<number | null> {
  const url = releasesUrl();
  if (!url) return null;
  const headers: Record<string, string> = { "user-agent": "fettle.page", accept: "application/vnd.github+json" };
  // Read-only and optional: without it GitHub's anonymous limit applies, which Cloudflare's shared addresses often hit.
  if (env.GITHUB_TOKEN) headers.authorization = `Bearer ${env.GITHUB_TOKEN}`;
  try {
    const res = await fetch(url, { headers, signal: AbortSignal.timeout(GITHUB_WAIT) });
    if (!res.ok) {
      console.log(JSON.stringify({ event: "download_count_failed", status: res.status }));
      return null;
    }
    const list: unknown = await res.json();
    if (!Array.isArray(list)) return null;
    let count = 0;
    for (const r of list) {
      if (!r || typeof r !== "object" || (r as { draft?: unknown }).draft) continue;
      for (const asset of ((r as { assets?: unknown }).assets as { name?: unknown; download_count?: unknown }[] | undefined) ?? []) {
        if (typeof asset?.name === "string" && /\.(exe|zip)$/i.test(asset.name) && typeof asset.download_count === "number") count += asset.download_count;
      }
    }
    return count;
  } catch (e) {
    console.log(JSON.stringify({ event: "download_count_failed", message: String(e).slice(0, 120) }));
    return null;
  }
}

/** With nothing stored yet, asks GitHub in the background, at most one request at a time and not right after a failure. */
function fillInBackground(env: Env, ctx?: ExecutionContext): void {
  if (busy(askedAt) || (failedAt && Date.now() - failedAt < RETRY)) return;
  askedAt = Date.now();
  const asking = refreshDownloadCount(env)
    .catch((e) => console.log(JSON.stringify({ event: "download_count_store_failed", message: String(e).slice(0, 120) })))
    .finally(() => {
      askedAt = 0;
    });
  ctx?.waitUntil(asking);
}

/**
 * The count, from memory when it was read in the last ten minutes. An older one is shown as it is
 * while the stored one is read again in the background. A fresh isolate reads the stored count,
 * waiting a moment at most. Null when no count is known: the line stays out.
 */
export async function downloadCount(env: Env, ctx?: ExecutionContext): Promise<number | null> {
  if (known) {
    if (Date.now() - known.at >= FRESH && !busy(readingAt)) {
      readingAt = Date.now();
      const reading = readStored(env)
        .then((count) => {
          // Nothing readable: keep the count in memory, and look again in ten minutes rather than on every visit.
          if (typeof count === "number") remember(count);
          else if (known) known.at = Date.now();
        })
        .finally(() => {
          readingAt = 0;
        });
      ctx?.waitUntil(reading);
    }
    return known.count;
  }
  const reading = readStored(env);
  ctx?.waitUntil(reading.then(remember));
  const stored = await Promise.race([reading, new Promise<"slow">((r) => setTimeout(() => r("slow"), READ_WAIT))]);
  if (typeof stored === "number") remember(stored);
  // Nothing stored, and no other request has filled it in meanwhile.
  else if (stored === null && !known) fillInBackground(env, ctx);
  // Read again: another request may have filled it in while this one waited.
  return (known as Known | undefined)?.count ?? null;
}

/** A home or download page with its download count filled in (or left hidden when there's no count). */
export async function withDownloadCount(request: Request, env: Env, ctx?: ExecutionContext): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return env.ASSETS.fetch(request);
  // Started first, so reading the stored count overlaps fetching the page.
  const counting = request.method === "GET" ? downloadCount(env, ctx) : Promise.resolve(null);
  // A bare request: the visitor's conditional headers would otherwise get a 304 that keeps an old count.
  const page = await env.ASSETS.fetch(new Request(request.url, { method: request.method }));
  if (!page.ok || !(page.headers.get("content-type") ?? "").includes("text/html")) return page;
  const headers = new Headers(page.headers);
  headers.delete("etag");
  headers.delete("content-length");
  const count = await counting;
  const plain = new Response(page.body, { status: page.status, headers });
  if (count === null) return plain;
  // The section shows the number; screen readers get the whole phrase ("1,234 downloads").
  let shown = "";
  let said = "";
  return new HTMLRewriter()
    .on("[data-download-count]", {
      element(el) {
        const intl = el.getAttribute("data-locale") === "it" ? "it-IT" : "en-GB";
        shown = new Intl.NumberFormat(intl, { useGrouping: "always" }).format(count);
        said = (count === 1 ? el.getAttribute("data-one") : el.getAttribute("data-many")?.replace("{n}", shown)) ?? "";
        if (said) el.removeAttribute("hidden");
      },
    })
    .on("[data-download-count] [data-download-n]", {
      element(el) {
        if (said) el.setInnerContent(shown);
      },
    })
    .on("[data-download-count] [data-download-said]", {
      element(el) {
        if (said) el.setInnerContent(said);
      },
    })
    .transform(plain);
}
