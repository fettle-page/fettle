// Response helpers shared by the API and the share page.

export type ErrorCode =
  | "not_found"
  | "method_not_allowed"
  | "unsupported_media_type"
  | "invalid_json"
  | "invalid_request"
  | "invalid_report"
  | "too_large"
  | "rate_limited"
  | "unauthorized"
  | "forbidden"
  | "password_required"
  | "wrong_password"
  | "invalid_pass";

/** A JSON API response. Reports can be deleted at any time, so nothing is cached. */
export function json(body: unknown, status = 200, headers?: HeadersInit): Response {
  const h = new Headers(headers);
  h.set("content-type", "application/json; charset=utf-8");
  h.set("x-content-type-options", "nosniff");
  h.set("cache-control", "no-store");
  return new Response(JSON.stringify(body), { status, headers: h });
}

export function apiError(status: number, code: ErrorCode, extra: { detail?: string; headers?: HeadersInit } = {}): Response {
  const body = extra.detail === undefined ? { error: code } : { error: code, detail: extra.detail };
  return json(body, status, extra.headers);
}

/**
 * Reports are capped at 64 KiB. Every text field has its own length limit, so the largest real report
 * is about 4 KB (5 KB with 26 startup apps), and one with every list long and every name at its limit
 * is about 40 KB.
 */
export const MAX_UPLOAD_BYTES = 65536;

/** Reads a request body as text, giving up as soon as it passes `max` bytes, without buffering the rest. */
export async function readCapped(request: Request, max: number): Promise<{ ok: true; text: string } | { ok: false; reason: "too_large" }> {
  const declared = Number(request.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > max) return { ok: false, reason: "too_large" };
  if (!request.body) return { ok: true, text: "" };
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > max) {
      await reader.cancel();
      return { ok: false, reason: "too_large" };
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  let at = 0;
  for (const c of chunks) {
    bytes.set(c, at);
    at += c.byteLength;
  }
  return { ok: true, text: new TextDecoder().decode(bytes) };
}
