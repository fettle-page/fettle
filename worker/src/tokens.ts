// Delete tokens: shown to the owner once; only their SHA-256 is stored.

export function newDeleteToken(): string {
  return base64url(crypto.getRandomValues(new Uint8Array(32)));
}

export async function hashToken(token: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(token));
  return hex(new Uint8Array(digest));
}

export async function tokenMatches(token: string, storedHash: string): Promise<boolean> {
  if (!/^[0-9a-f]{64}$/.test(storedHash)) return false;
  return sameBytes(await hashToken(token), storedHash);
}

/** Constant-time comparison of two strings of the same length. */
export function sameBytes(a: string, b: string): boolean {
  const x = new TextEncoder().encode(a);
  const y = new TextEncoder().encode(b);
  return x.byteLength === y.byteLength && crypto.subtle.timingSafeEqual(x, y);
}

export function base64url(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}
