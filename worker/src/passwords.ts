// Optional share passwords. The server checks them and keeps only a salted PBKDF2 hash.
import { base64url, hex, sameBytes } from "./tokens.ts";

/** The most workerd allows for PBKDF2. Guessing is also capped per report by UnlockGuard. */
const ITERATIONS = 100_000;

export const PASSWORD_MIN = 8;
export const PASSWORD_MAX = 128;

export async function hashPassword(password: string, salt: Uint8Array = crypto.getRandomValues(new Uint8Array(16))): Promise<{ salt: string; hash: string }> {
  return { salt: base64url(salt), hash: await derive(password, salt) };
}

export async function passwordMatches(password: string, salt: string, hash: string): Promise<boolean> {
  if (!/^[0-9a-f]{64}$/.test(hash)) return false;
  const saltBytes = fromBase64url(salt);
  if (!saltBytes || saltBytes.byteLength !== 16) return false;
  return sameBytes(await derive(password, saltBytes), hash);
}

async function derive(password: string, salt: Uint8Array): Promise<string> {
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(password), "PBKDF2", false, ["deriveBits"]);
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
