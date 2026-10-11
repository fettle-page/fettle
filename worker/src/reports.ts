// The share API: upload, read and delete reports in R2.
import type { Locale } from "@fettle/brain";
import { brain } from "@fettle/brain/worker";
import { parseReport, type Report } from "@fettle/report-schema";
import { apiError, json, MAX_UPLOAD_BYTES, readCapped } from "./http.ts";
import { isValidId, keyFor, newId, TTL_DAYS, type Ttl } from "./ids.ts";
import { hashPassword, PASSWORD_MAX, PASSWORD_MIN, passwordMatches } from "./passwords.ts";
import { hashToken, newDeleteToken, tokenMatches } from "./tokens.ts";

const DAY_MS = 86_400_000;
const isTtl = (v: unknown): v is Ttl => typeof v === "string" && Object.hasOwn(TTL_DAYS, v);

/**
 * The rate-limit key for a client address. IPv6 addresses are grouped by their /64, because one
 * home or server usually owns the whole block and could otherwise rotate through it.
 */
export function rateKey(ip: string): string {
  if (!ip.includes(":")) return ip;
  // An IPv4 address written as IPv6 (::ffff:203.0.113.7) is still one IPv4 client.
  const mapped = /^::ffff:(\d+\.\d+\.\d+\.\d+)$/i.exec(ip);
  if (mapped) return mapped[1]!;
  const [head = "", tail] = ip.toLowerCase().split("::");
  const left = head ? head.split(":") : [];
  const right = tail ? tail.split(":") : [];
  const groups = tail === undefined ? left : [...left, ...Array(Math.max(0, 8 - left.length - right.length)).fill("0"), ...right];
  return `${groups.slice(0, 4).map((g) => (parseInt(g, 16) || 0).toString(16)).join(":")}::/64`;
}

/**
 * How the logs name a report: the first 12 hex characters of a SHA-256 of its ID. Enough to follow one
 * link through the logs, but not the ID itself, which is all it takes to open a link without a password.
 */
export async function logRef(id: string): Promise<string> {
  return (await hashToken(`log:${id}`)).slice(0, 12);
}

const clientKey = (request: Request) => rateKey(request.headers.get("cf-connecting-ip") ?? "unknown");

const isJson = (request: Request) => (request.headers.get("content-type") ?? "").toLowerCase().startsWith("application/json");

export const rateLimited = (seconds = 60) => apiError(429, "rate_limited", { headers: { "retry-after": String(seconds) } });

/** Reads a capped JSON body; on failure returns the error response to send instead. */
export async function readJson(request: Request, max: number): Promise<{ ok: true; value: unknown } | { ok: false; response: Response }> {
  if (!isJson(request)) return { ok: false, response: apiError(415, "unsupported_media_type") };
  const body = await readCapped(request, max);
  if (!body.ok) return { ok: false, response: apiError(413, "too_large") };
  try {
    return { ok: true, value: JSON.parse(body.text) };
  } catch {
    return { ok: false, response: apiError(400, "invalid_json") };
  }
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const isPassword = (v: unknown): v is string => typeof v === "string" && v.length >= PASSWORD_MIN && v.length <= PASSWORD_MAX;

/** POST /api/reports  `{ expires?, password?, report }` → 201 `{ id, url, deleteUrl, deleteToken, expiresAt, locked }`. */
export async function createReport(request: Request, env: Env): Promise<Response> {
  // Before reading the body, so a flood stays cheap.
  const { success } = await env.UPLOAD_LIMITER.limit({ key: clientKey(request) });
  if (!success) return rateLimited();

  const read = await readJson(request, MAX_UPLOAD_BYTES);
  if (!read.ok) return read.response;
  const envelope = read.value;
  if (!isRecord(envelope)) return apiError(400, "invalid_request");
  if (envelope.expires !== undefined && !isTtl(envelope.expires)) return apiError(400, "invalid_request");
  if (envelope.password !== undefined && !isPassword(envelope.password)) return apiError(400, "invalid_request");

  const parsed = parseReport(envelope.report);
  if (!parsed.ok) return apiError(400, "invalid_report", { detail: parsed.error.slice(0, 1000) });
  // The schema only checks the timestamp's shape; the verdicts need a real date (no month 13).
  if (Number.isNaN(Date.parse(parsed.report.collected_at))) return apiError(400, "invalid_report", { detail: "collected_at is not a real date" });

  const ttl: Ttl = (envelope.expires as Ttl | undefined) ?? "30d";
  const id = newId(ttl);
  const deleteToken = newDeleteToken();
  const expiresAt = new Date(Date.now() + TTL_DAYS[ttl] * DAY_MS).toISOString();
  // The parsed report, not the raw body: unknown fields never reach storage.
  const stored = JSON.stringify(parsed.report);
  const meta: Meta = { expiresAt, deleteHash: await hashToken(deleteToken) };
  const locked = envelope.password !== undefined;
  if (locked) {
    const pw = await hashPassword(envelope.password as string);
    meta.pwSalt = pw.salt;
    meta.pwHash = pw.hash;
  }
  await env.REPORTS.put(keyFor(id), stored, { httpMetadata: { contentType: "application/json" }, customMetadata: meta });
  console.log(JSON.stringify({ event: "report_created", ref: await logRef(id), ttl, locked, bytes: new TextEncoder().encode(stored).byteLength }));

  const url = `${new URL(request.url).origin}/r/${id}`;
  return json({ id, url, deleteUrl: `${url}/delete#${deleteToken}`, deleteToken, expiresAt, locked }, 201);
}

type Meta = Record<string, string>;

/**
 * R2 lifecycle rules can take about a day to delete an expired object, so expiry is checked here.
 * An expired object is removed on first sight.
 */
async function stillLive(env: Env, key: string, meta: Meta | undefined): Promise<boolean> {
  const expiresAt = Date.parse(meta?.expiresAt ?? "");
  if (Number.isFinite(expiresAt) && expiresAt > Date.now()) return true;
  await env.REPORTS.delete(key);
  return false;
}

export type Loaded = { report: Report; expiresAt: string; locked: boolean };

/** A live report, or null when the id is malformed, unknown, expired, damaged or no longer parses. */
export async function loadReport(env: Env, id: string): Promise<Loaded | null> {
  if (!isValidId(id)) return null;
  const key = keyFor(id);
  const object = await env.REPORTS.get(key);
  if (!object) return null;
  if (!(await stillLive(env, key, object.customMetadata))) return null;
  let body: unknown;
  try {
    body = await object.json();
  } catch {
    return null;
  }
  const parsed = parseReport(body);
  if (!parsed.ok) return null;
  const meta = object.customMetadata!;
  return { report: parsed.report, expiresAt: meta.expiresAt!, locked: meta.pwHash !== undefined };
}

/** A live object's metadata, read without its body. */
async function liveMeta(env: Env, id: string): Promise<Meta | null> {
  if (!isValidId(id)) return null;
  const key = keyFor(id);
  const head = await env.REPORTS.head(key);
  if (!head || !(await stillLive(env, key, head.customMetadata))) return null;
  return head.customMetadata ?? {};
}

/** GET /api/reports/:id */
export async function getReport(id: string, env: Env): Promise<Response> {
  const loaded = await loadReport(env, id);
  if (!loaded) return apiError(404, "not_found");
  if (loaded.locked) return apiError(401, "password_required");
  return json({ report: loaded.report, expiresAt: loaded.expiresAt });
}

/**
 * POST /api/reports/:id/unlock  `{ password | pass, locale? }` → the finished receipt in that language
 * (`en` unless `locale` is exactly "it"), built here with the Rust brain like a server-drawn page.
 * Wrong guesses are capped per report per minute, per visitor per day, and per report per day
 * (UnlockGuard); requests per IP are capped too, which bounds the hashing cost. All of it runs
 * before the password is hashed. Using a pass renews it, and the reply carries its new expiry.
 */
export async function unlockReport(request: Request, id: string, env: Env): Promise<Response> {
  // Metadata only: the report itself is read once the password is right.
  const meta = await liveMeta(env, id);
  if (!meta) return apiError(404, "not_found");
  const { success } = await env.UNLOCK_LIMITER.limit({ key: clientKey(request) });
  if (!success) return rateLimited();
  const read = await readJson(request, 4096);
  if (!read.ok) return read.response;
  const body = isRecord(read.value) ? read.value : {};
  const { password, pass } = body;
  if (typeof password !== "string" && typeof pass !== "string") return apiError(400, "invalid_request");
  const locale: Locale = body.locale === "it" ? "it" : "en";

  let issued: { pass: string; expiresAt: number } | undefined;
  if (meta.pwHash !== undefined) {
    const guard = env.UNLOCK_GUARD.getByName(id);
    const visitor = clientKey(request);
    if (typeof pass === "string") {
      // A pass from an earlier right password: no guess counted, so a lockout can't touch it.
      const renewed = await guard.usePass(pass, Date.parse(meta.expiresAt ?? ""));
      if (renewed === null) return apiError(403, "invalid_pass");
      issued = { pass, expiresAt: renewed };
    } else {
      const wait = await guard.begin(visitor);
      if (wait > 0) return rateLimited(wait);
      if (!(await passwordMatches(password as string, meta.pwSalt ?? "", meta.pwHash))) {
        console.log(JSON.stringify({ event: "unlock_failed", ref: await logRef(id) }));
        return apiError(403, "wrong_password");
      }
      await guard.succeeded(visitor);
      issued = await guard.issuePass(Date.parse(meta.expiresAt ?? ""));
    }
  }
  const loaded = await loadReport(env, id);
  if (!loaded) return apiError(404, "not_found");
  let model;
  try {
    // UTC, as on a server-drawn page; the browser shows its own time once mounted.
    model = brain.receiptFor(loaded.report, locale, "UTC").model;
  } catch {
    // Stored before uploads checked the date, or data the receipt can't read: "isn't here", never a 500.
    return apiError(404, "not_found");
  }
  console.log(JSON.stringify({ event: "report_unlocked", ref: await logRef(id), with: typeof pass === "string" ? "pass" : "password" }));
  const opened = { model, expiresAt: loaded.expiresAt };
  return json(issued ? { ...opened, pass: issued.pass, passExpiresAt: new Date(issued.expiresAt).toISOString() } : opened);
}

/** DELETE /api/reports/:id with `Authorization: Bearer <deleteToken>`. */
export async function deleteReport(request: Request, id: string, env: Env): Promise<Response> {
  const token = /^Bearer (\S+)$/.exec(request.headers.get("authorization") ?? "")?.[1];
  if (!token) return apiError(401, "unauthorized");
  const meta = await liveMeta(env, id);
  if (!meta) return apiError(404, "not_found");
  if (!(await tokenMatches(token, meta.deleteHash ?? ""))) return apiError(403, "forbidden");
  await env.REPORTS.delete(keyFor(id));
  console.log(JSON.stringify({ event: "report_deleted", ref: await logRef(id) }));
  return new Response(null, { status: 204 });
}
