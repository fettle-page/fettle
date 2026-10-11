// The download count, as a section of the main download button on the home and download pages: GitHub's own total for the installer and the zip
// of every published release. Asked for at most once an hour (per isolate) and written into the page's
// empty line, so visitors' browsers never contact GitHub and nothing about who downloaded is kept.
// A page never waits on GitHub: only one visitor's request asks at a time (Workers can't share one
// request's promise with another, so the others just use what's known), an old count is shown while it
// is refreshed in the background, and a fresh isolate waits a moment at most, then leaves the line out.
import { release } from "@fettle/web/release";

/** The pages with a download count (apps/web DownloadCount.astro); wrangler.jsonc runs the Worker first on them. */
export const DOWNLOAD_PAGES = new Set(["/", "/it", "/download", "/it/download"]);

const HOUR = 3_600_000;
/** After a failed ask, try again in five minutes rather than an hour. */
const RETRY = 5 * 60_000;
/** The longest a page waits for a first answer, when the isolate has no count yet. */
const COLD_WAIT = 300;
/** Taken as abandoned after this long (the asking request may have been cancelled). */
const ASKING_MAX = 10_000;
type Known = { count: number | null; at: number };
let known: Known | undefined;
/** When a request started asking GitHub, or 0 when none is. */
let askedAt = 0;

/** For tests: forget the remembered count. */
export function resetDownloadCount(): void {
  known = undefined;
  askedAt = 0;
}

/** For tests: make the remembered count an hour old. */
export function ageDownloadCount(): void {
  if (known) known.at -= HOUR;
}

/** Asks GitHub and remembers the answer (or, on failure, when to try again). */
function refresh(env: Env): Promise<void> {
  askedAt = Date.now();
  return askGitHub(env)
    .then((fresh) => {
      const now = Date.now();
      known = fresh !== null ? { count: fresh, at: now } : { count: known?.count ?? null, at: now - HOUR + RETRY };
    })
    .finally(() => {
      askedAt = 0;
    });
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
    const res = await fetch(url, { headers, signal: AbortSignal.timeout(3000) });
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

/**
 * The count, from memory when it's under an hour old. An older one is shown as it is while it's
 * refreshed in the background; the last good one stays if GitHub fails meanwhile.
 */
export async function downloadCount(env: Env, ctx?: ExecutionContext): Promise<number | null> {
  if (known && Date.now() - known.at < HOUR) return known.count;
  // Another visitor's request is already asking: show what's known, without waiting.
  if (askedAt && Date.now() - askedAt < ASKING_MAX) return known?.count ?? null;
  const pending = refresh(env);
  ctx?.waitUntil(pending);
  if (known) return known.count;
  // A fresh isolate: wait a moment for the first answer, never longer.
  await Promise.race([pending, new Promise((r) => setTimeout(r, COLD_WAIT))]);
  // Read again: the answer may have arrived while this request waited.
  return (known as Known | undefined)?.count ?? null;
}

/** A home or download page with its download count filled in (or left hidden when there's no count). */
export async function withDownloadCount(request: Request, env: Env, ctx?: ExecutionContext): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return env.ASSETS.fetch(request);
  // A bare request: the visitor's conditional headers would otherwise get a 304 that keeps an old count.
  const page = await env.ASSETS.fetch(new Request(request.url, { method: request.method }));
  if (!page.ok || !(page.headers.get("content-type") ?? "").includes("text/html")) return page;
  const headers = new Headers(page.headers);
  headers.delete("etag");
  headers.delete("content-length");
  const count = request.method === "GET" ? await downloadCount(env, ctx) : null;
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
