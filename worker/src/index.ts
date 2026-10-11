import { DOWNLOAD_PAGES, withDownloadCount } from "./downloads.ts";
import { apiError } from "./http.ts";
import { createReport, deleteReport, getReport, unlockReport } from "./reports.ts";
import { sharePage } from "./share.ts";

export { UploadBudget } from "./budget.ts";
export { UnlockGuard } from "./guard.ts";

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
    const { pathname } = new URL(request.url);
    if (pathname === "/api" || pathname.startsWith("/api/")) return api(request, env, pathname);
    if (pathname === "/r" || pathname.startsWith("/r/")) return sharePage(request, env);
    if (DOWNLOAD_PAGES.has(pathname)) return withDownloadCount(request, env, ctx);
    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
