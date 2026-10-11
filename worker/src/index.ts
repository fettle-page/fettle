import { DOWNLOAD_PAGES, withDownloadCount } from "./downloads.ts";
import { apiError } from "./http.ts";
import { createReport, deleteReport, getReport, unlockReport } from "./reports.ts";
import { sharePage } from "./share.ts";

export { UploadBudget } from "./budget.ts";
export { UnlockGuard } from "./guard.ts";

/**
 * The clean address for "/x/", "/x.html" and "/x/index.html" ("/x"), or null when the address is
 * already clean. The static files would redirect these too, but only with a 307.
 * Leading slashes and backslashes collapse to one, so "//evil.example/" can never become a
 * protocol-relative address on another site.
 */
export function cleanPath(pathname: string): string | null {
  if (pathname === "/" || !(pathname.endsWith("/") || pathname.endsWith(".html"))) return null;
  const clean = pathname
    .replace(/^[/\\]+/, "/")
    .replace(/\/+$/, "")
    .replace(/\.html$/, "")
    .replace(/(^|\/)index$/, "");
  return clean === "" ? "/" : clean;
}

/** A 301 to `path` on this site; null if the result would somehow leave it. */
function redirectWithin(url: URL, path: string): Response | null {
  // Joined as text onto our own origin, never resolved against the request, so it can't change host.
  const target = new URL(url.origin + path + url.search);
  return target.origin === url.origin ? Response.redirect(target.href, 301) : null;
}

const notAllowed = (allow: string) => apiError(405, "method_not_allowed", { headers: { allow } });

async function api(request: Request, env: Env, path: string): Promise<Response> {
  if (path === "/api/reports") return request.method === "POST" ? createReport(request, env) : notAllowed("POST");
  const one = /^\/api\/reports\/([^/]+)$/.exec(path);
  if (one) {
    const id = one[1]!;
    if (request.method === "GET") return getReport(request, id, env);
    if (request.method === "DELETE") return deleteReport(request, id, env);
    return notAllowed("GET, DELETE");
  }
  const unlock = /^\/api\/reports\/([^/]+)\/unlock$/.exec(path);
  if (unlock) return request.method === "POST" ? unlockReport(request, unlock[1]!, env) : notAllowed("POST");
  return apiError(404, "not_found");
}

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    const { pathname } = url;
    if (pathname === "/api" || pathname.startsWith("/api/")) return api(request, env, pathname);
    const clean = cleanPath(pathname);
    const redirect = clean !== null && !pathname.startsWith("/r/") ? redirectWithin(url, clean) : null;
    if (redirect) return redirect;
    if (pathname === "/r" || pathname.startsWith("/r/")) return sharePage(request, env);
    if (DOWNLOAD_PAGES.has(pathname)) return withDownloadCount(request, env, ctx);
    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
