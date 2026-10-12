// Optional share passwords. The server checks them and keeps only a salted PBKDF2 hash.
//
// Version 2 (since 1.0.4) first mixes in a server secret, PASSWORD_PEPPER, with HMAC-SHA256, then
// runs PBKDF2 over that. PBKDF2 can't go past 100,000 rounds in workerd, so the secret is what keeps a
// stolen hash from being guessed at offline: without it, no guess can even be checked. Reports stored
// before 1.0.4 carry no version and are checked the old way (PBKDF2 over the password alone).
import { base64url, hex, sameBytes } from "./tokens.ts";

/** The most workerd allows for PBKDF2. Guessing is also capped per report by UnlockGuard. */
const ITERATIONS = 100_000;

export const PASSWORD_MIN = 8;
export const PASSWORD_MAX = 128;

/** What a report's metadata keeps: `version` is "2", or missing for a hash from before 1.0.4. */
export type StoredPassword = { salt: string; hash: string; version?: string };

/** A new version 2 hash: PBKDF2 over HMAC-SHA256(pepper, password). */
export async function hashPassword(password: string, pepper: string, salt: Uint8Array = randomSalt()): Promise<Required<StoredPassword>> {
  return { salt: base64url(salt), hash: await derive(await peppered(password, pepper), salt), version: "2" };
}

/** The scheme before 1.0.4, kept only to check old reports (and to make them in tests). */
export async function hashPasswordV1(password: string, salt: Uint8Array = randomSalt()): Promise<StoredPassword> {
  return { salt: base64url(salt), hash: await derive(new TextEncoder().encode(password), salt) };
}

/** Whether `password` is the one behind `stored`. A version 2 hash can't be checked without the pepper. */
export async function passwordMatches(password: string, stored: StoredPassword, pepper: string | undefined): Promise<boolean> {
  if (!/^[0-9a-f]{64}$/.test(stored.hash)) return false;
  const saltBytes = fromBase64url(stored.salt);
  if (!saltBytes || saltBytes.byteLength !== 16) return false;
  if (stored.version === undefined) return sameBytes(await derive(new TextEncoder().encode(password), saltBytes), stored.hash);
  if (stored.version !== "2" || !pepper) return false;
  return sameBytes(await derive(await peppered(password, pepper), saltBytes), stored.hash);
}

const randomSalt = () => crypto.getRandomValues(new Uint8Array(16));

async function peppered(password: string, pepper: string): Promise<Uint8Array> {
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(pepper), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(password)));
}

async function derive(material: Uint8Array, salt: Uint8Array): Promise<string> {
  const key = await crypto.subtle.importKey("raw", material, "PBKDF2", false, ["deriveBits"]);
  const bits = await crypto.subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt, iterations: ITERATIONS }, key, 256);
  return hex(new Uint8Array(bits));
}

function fromBase64url(s: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]+$/.test(s)) return null;
  try {
    const bin = atob(s.replaceAll("-", "+").replaceAll("_", "/"));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}
