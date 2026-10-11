import { DurableObject } from "cloudflare:workers";
import { base64url, hashToken } from "./tokens.ts";

/** Wrong passwords allowed per report per minute, across every visitor and data centre. */
export const GUESSES_PER_MINUTE = 10;
/**
 * Wrong passwords allowed per report per day (UTC). The minute cap alone still allows 14,400 a day;
 * 100 is far more than anyone mistyping their own password needs, and it keeps even a weak 8-character
 * password out of reach (a dictionary of 10,000 common passwords would take over three months).
 */
export const GUESSES_PER_DAY = 100;
/** How long a browser that gave the right password can reopen the report without it. */
export const PASS_MS = 86_400_000;
const WINDOW_MS = 60_000;
const DAY_MS = 86_400_000;

/** `window`/`count`: this minute's tries; `day`/`dayCount`: this UTC day's. */
type State = { window: number; count: number; day: number; dayCount: number };

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
  /** Reserves a try: 0 when allowed, otherwise the seconds to wait (the minute's or the day's end). */
  async begin(): Promise<number> {
    const now = Date.now();
    const window = Math.floor(now / WINDOW_MS);
    const day = Math.floor(now / DAY_MS);
    const saved = await this.ctx.storage.get<State>("state");
    const sameDay = saved !== undefined && saved.day === day;
    const state: State = {
      window,
      count: sameDay && saved.window === window ? saved.count : 0,
      day,
      dayCount: sameDay ? saved.dayCount : 0,
    };
    if (state.dayCount >= GUESSES_PER_DAY) return Math.ceil(((day + 1) * DAY_MS - now) / 1000);
    if (state.count >= GUESSES_PER_MINUTE) return 60;
    state.count += 1;
    state.dayCount += 1;
    await this.ctx.storage.put("state", state);
    await this.cleanUpBy((day + 1) * DAY_MS);
    return 0;
  }

  async succeeded(): Promise<void> {
    const state = await this.ctx.storage.get<State>("state");
    if (state && state.count > 0) await this.ctx.storage.put("state", { ...state, count: state.count - 1, dayCount: Math.max(0, state.dayCount - 1) });
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

  /** Drops a past day's guesses and expired passes, and wakes again for whichever ends next. */
  async alarm(): Promise<void> {
    const now = Date.now();
    const state = await this.ctx.storage.get<State>("state");
    let next = Infinity;
    // A count saved before the daily cap has no day: it's from an earlier day either way.
    if (state && !(state.day >= Math.floor(now / DAY_MS))) await this.ctx.storage.delete("state");
    else if (state) next = (state.day + 1) * DAY_MS;
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
