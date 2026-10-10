import { DurableObject } from "cloudflare:workers";
import { base64url, hashToken } from "./tokens.ts";

/** Wrong passwords allowed per report per minute, across every visitor and data centre. */
export const GUESSES_PER_MINUTE = 10;
/** How long a browser that gave the right password can reopen the report without it. */
export const PASS_MS = 86_400_000;
const WINDOW_MS = 60_000;

type State = { window: number; count: number };

/**
 * One instance per password-protected report (keyed by its id), so the count is global, unlike the
 * rate-limiting binding, which counts per data centre. A try is reserved before the password is
 * hashed, so parallel guesses can't race past the cap; a right password gives its try back.
 *
 * It also keeps the report's unlock passes: random tokens handed to a browser after a right
 * password, stored here only as SHA-256 hashes (`pass:<hash>` → expiry in ms). A pass skips the
 * guess limit, so someone burning through wrong guesses can't lock out people who already opened it.
 */
export class UnlockGuard extends DurableObject<Env> {
  async begin(): Promise<boolean> {
    const window = Math.floor(Date.now() / WINDOW_MS);
    const saved = await this.ctx.storage.get<State>("state");
    const state = saved && saved.window === window ? saved : { window, count: 0 };
    if (state.count >= GUESSES_PER_MINUTE) return false;
    state.count += 1;
    await this.ctx.storage.put("state", state);
    await this.cleanUpBy((window + 2) * WINDOW_MS);
    return true;
  }

  async succeeded(): Promise<void> {
    const state = await this.ctx.storage.get<State>("state");
    if (state && state.count > 0) await this.ctx.storage.put("state", { ...state, count: state.count - 1 });
  }

  /** A new pass, valid for 24 hours but never past `notAfter` (the report's own expiry, in ms). */
  async issuePass(notAfter: number): Promise<{ pass: string; expiresAt: number }> {
    const pass = base64url(crypto.getRandomValues(new Uint8Array(32)));
    const expiresAt = Math.min(Date.now() + PASS_MS, notAfter);
    await this.ctx.storage.put(`pass:${await hashToken(pass)}`, expiresAt);
    await this.cleanUpBy(expiresAt);
    return { pass, expiresAt };
  }

  async checkPass(pass: string): Promise<boolean> {
    // Looked up by its hash, so no stored secret is ever compared.
    const expiresAt = await this.ctx.storage.get<number>(`pass:${await hashToken(pass)}`);
    return expiresAt !== undefined && expiresAt > Date.now();
  }

  /** Drops an old guess window and expired passes, and wakes again for the next pass to expire. */
  async alarm(): Promise<void> {
    const now = Date.now();
    const state = await this.ctx.storage.get<State>("state");
    if (state && state.window < Math.floor(now / WINDOW_MS)) await this.ctx.storage.delete("state");
    let next = Infinity;
    for (const [key, expiresAt] of await this.ctx.storage.list<number>({ prefix: "pass:" })) {
      if (expiresAt <= now) await this.ctx.storage.delete(key);
      else next = Math.min(next, expiresAt);
    }
    if (next !== Infinity) await this.ctx.storage.setAlarm(next);
  }

  private async cleanUpBy(when: number): Promise<void> {
    const current = await this.ctx.storage.getAlarm();
    if (current === null || when < current) await this.ctx.storage.setAlarm(when);
  }
}
