// The app's side of share links: upload a report, delete it later. Never throws; every outcome
// is a value the UI can word. The limits mirror the Worker's, so a bad request isn't even sent.
import type { Report } from "@fettle/report-schema";

export type Expiry = "7d" | "30d" | "1y";
export const PASSWORD_MIN = 8;
export const PASSWORD_MAX = 128;
export const MAX_UPLOAD_BYTES = 65_536;

export type ShareRequest = { report: Report; expires?: Expiry; password?: string };
export type Shared = { id: string; url: string; deleteUrl: string; deleteToken: string; expiresAt: string; locked: boolean };

/** The Worker's error codes, plus what can go wrong before or around the request. */
export type ShareError =
  | "invalid_request"
  | "invalid_report"
  | "unavailable"
  | "too_large"
  | "rate_limited"
  | "unauthorized"
  | "forbidden"
  | "unsupported_media_type"
  | "invalid_json"
  | "password_too_short"
  | "password_too_long"
  | "network"
  | "bad_response";

export type Failure = { ok: false; error: ShareError; retryAfter?: number; detail?: string };
type FetchFn = typeof fetch;
/** `timeoutMs`: how long to wait for the server before giving up (default 20 s). */
export type Options = { timeoutMs?: number };
const DEFAULT_TIMEOUT_MS = 20_000;

const SERVER_ERRORS = new Set<string>(["invalid_request", "invalid_report", "unavailable", "too_large", "rate_limited", "unauthorized", "forbidden", "unsupported_media_type", "invalid_json"]);

const endpoint = (origin: string, path: string) => origin.replace(/\/+$/, "") + path;

export async function shareReport(origin: string, req: ShareRequest, fetchFn: FetchFn = fetch, opts: Options = {}): Promise<{ ok: true; shared: Shared } | Failure> {
  const password = req.password || undefined;
  if (password !== undefined && password.length < PASSWORD_MIN) return { ok: false, error: "password_too_short" };
  if (password !== undefined && password.length > PASSWORD_MAX) return { ok: false, error: "password_too_long" };
  const body = JSON.stringify({ report: req.report, ...(req.expires ? { expires: req.expires } : {}), ...(password ? { password } : {}) });
  if (new TextEncoder().encode(body).byteLength > MAX_UPLOAD_BYTES) return { ok: false, error: "too_large" };

  const res = await send(fetchFn, endpoint(origin, "/api/reports"), { method: "POST", headers: { "content-type": "application/json" }, body }, opts);
  if (!res) return { ok: false, error: "network" };
  if (res.status !== 201) return failure(res);
  const shared = await readJson(res);
  return isShared(shared) ? { ok: true, shared } : { ok: false, error: "bad_response" };
}

/**
 * A link the Worker says is already gone (expired or deleted elsewhere) counts as deleted. Any
 * other 404, such as a proxy's page at a wrong address, doesn't: the report may still be public.
 */
export async function deleteShared(origin: string, shared: Pick<Shared, "id" | "deleteToken">, fetchFn: FetchFn = fetch, opts: Options = {}): Promise<{ ok: true } | Failure> {
  const res = await send(fetchFn, endpoint(origin, `/api/reports/${encodeURIComponent(shared.id)}`), {
    method: "DELETE",
    headers: { authorization: `Bearer ${shared.deleteToken}` },
  }, opts);
  if (!res) return { ok: false, error: "network" };
  if (res.status === 204) return { ok: true };
  if (res.status === 404) {
    const body = await readJson(res);
    return isRecord(body) && body.error === "not_found" ? { ok: true } : { ok: false, error: "bad_response" };
  }
  return failure(res);
}

async function send(fetchFn: FetchFn, url: string, init: RequestInit, opts: Options): Promise<Response | null> {
  try {
    return await fetchFn(url, { ...init, signal: AbortSignal.timeout(opts.timeoutMs ?? DEFAULT_TIMEOUT_MS) });
  } catch {
    return null;
  }
}

async function readJson(res: Response): Promise<unknown> {
  try {
    return await res.json();
  } catch {
    return undefined;
  }
}

async function failure(res: Response): Promise<Failure> {
  const body = await readJson(res);
  const code = isRecord(body) && typeof body.error === "string" ? body.error : "";
  if (!SERVER_ERRORS.has(code)) return { ok: false, error: "bad_response" };
  const out: Failure = { ok: false, error: code as ShareError };
  const wait = Number(res.headers.get("retry-after"));
  if (code === "rate_limited" && Number.isFinite(wait) && wait > 0) out.retryAfter = wait;
  if (isRecord(body) && typeof body.detail === "string") out.detail = body.detail;
  return out;
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

function isShared(v: unknown): v is Shared {
  return (
    isRecord(v) &&
    ["id", "url", "deleteUrl", "deleteToken", "expiresAt"].every((k) => typeof v[k] === "string") &&
    typeof v.locked === "boolean"
  );
}
