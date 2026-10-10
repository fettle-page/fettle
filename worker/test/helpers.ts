import { exports } from "cloudflare:workers";
import tired from "./fixtures/tired.json";

export const ORIGIN = "http://example.com";
export const DAY = 86_400_000;

/** A fresh copy of the tired laptop report, safe to mutate. */
export const tiredReport = (): Record<string, any> => structuredClone(tired);

export function call(path: string, init?: RequestInit): Promise<Response> {
  return exports.default.fetch(new Request(ORIGIN + path, init));
}

/** POSTs an upload envelope (any JSON value, or a raw string body). */
export function upload(body: unknown, init: { ip?: string; contentType?: string } = {}): Promise<Response> {
  const headers: Record<string, string> = { "content-type": init.contentType ?? "application/json" };
  if (init.ip) headers["cf-connecting-ip"] = init.ip;
  return call("/api/reports", { method: "POST", headers, body: typeof body === "string" ? body : JSON.stringify(body) });
}

export type Created = { id: string; url: string; deleteUrl: string; deleteToken: string; expiresAt: string; locked: boolean };

let ipCounter = 0;
/** A unique documentation IP per call, so tests never share a rate-limit bucket. */
export const freshIp = () => `198.51.100.${++ipCounter}`;

/** Uploads the tired report and returns the 201 body. */
export async function share(extra: Record<string, unknown> = {}, report = tiredReport()): Promise<Created> {
  const res = await upload({ report, ...extra }, { ip: freshIp() });
  if (res.status !== 201) throw new Error(`upload failed: ${res.status} ${await res.text()}`);
  return res.json();
}

/**
 * The local rate limiter counts in fixed windows aligned to the wall clock (floor(now / 60 s)).
 * A burst that straddles a window edge would wrongly pass, so start bursts early in a window.
 */
export async function untilFreshWindow(): Promise<void> {
  const into = Date.now() % 60_000;
  if (into > 45_000) await new Promise((r) => setTimeout(r, 60_000 - into + 50));
}
