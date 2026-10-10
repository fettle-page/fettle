// The download count, on a tape stuck to the main download button of the home and download pages: GitHub's own total for the installer and the zip
// of every published release. Asked for at most once an hour (per isolate) and written into the page's
// empty line, so visitors' browsers never contact GitHub and nothing about who downloaded is kept.
import { release } from "@fettle/web/release";

/** The pages with a download count tape (apps/web DownloadCount.astro); wrangler.jsonc runs the Worker first on them. */
export const DOWNLOAD_PAGES = new Set(["/", "/it", "/download", "/it/download"]);

const HOUR = 3_600_000;
/** After a failed ask, try again in five minutes rather than an hour. */
const RETRY = 5 * 60_000;
let known: { count: number | null; at: number } | undefined;

/** For tests: forget the remembered count. */
export function resetDownloadCount(): void {
  known = undefined;
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

/** The count, from memory when it's under an hour old; the last good one if GitHub fails meanwhile. */
export async function downloadCount(env: Env): Promise<number | null> {
  const now = Date.now();
  if (known && now - known.at < HOUR) return known.count;
  const fresh = await askGitHub(env);
  known = fresh !== null ? { count: fresh, at: now } : { count: known?.count ?? null, at: now - HOUR + RETRY };
  return known.count;
}

/** A home or download page with its count tape filled in (or left hidden when there's no count). */
export async function withDownloadCount(request: Request, env: Env): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return env.ASSETS.fetch(request);
  // A bare request: the visitor's conditional headers would otherwise get a 304 that keeps an old count.
  const page = await env.ASSETS.fetch(new Request(request.url, { method: request.method }));
  if (!page.ok || !(page.headers.get("content-type") ?? "").includes("text/html")) return page;
  const headers = new Headers(page.headers);
  headers.delete("etag");
  headers.delete("content-length");
  const count = request.method === "GET" ? await downloadCount(env) : null;
  const plain = new Response(page.body, { status: page.status, headers });
  if (count === null) return plain;
  return new HTMLRewriter()
    .on("[data-download-count]", {
      element(el) {
        const intl = el.getAttribute("data-locale") === "it" ? "it-IT" : "en-GB";
        const n = new Intl.NumberFormat(intl, { useGrouping: "always" }).format(count);
        const text = count === 1 ? el.getAttribute("data-one") : el.getAttribute("data-many")?.replace("{n}", n);
        if (!text) return;
        el.setInnerContent(text);
        el.removeAttribute("hidden");
      },
    })
    .transform(plain);
}
