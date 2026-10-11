import { DurableObject } from "cloudflare:workers";
import { base64url, hashToken } from "./tokens.ts";

/** Wrong passwords allowed per report per minute, across every visitor and data centre. */
export const GUESSES_PER_MINUTE = 10;
/**
 * Wrong passwords one visitor (an IP address, or an IPv6 /64) may try on one report per day (UTC).
 * Far more than anyone mistyping their own password needs, and it stops one person from using up
 * the report's whole day and locking everyone else out.
 */
export const GUESSES_PER_VISITOR_PER_DAY = 20;
/**
 * Wrong passwords allowed per report per day across all visitors: the ceiling for someone with many
 * addresses. The minute cap alone would allow 14,400 a day; at 500, a dictionary of 10,000 common
 * passwords takes 20 days, and locking a report for a day takes 25 different addresses.
 */
export const GUESSES_PER_DAY = 500;
/** How long a browser that gave the right password can reopen the report without it; renewed on use. */
export const PASS_MS = 86_400_000;
const WINDOW_MS = 60_000;
const DAY_MS = 86_400_000;

/** The report's tries: `window`/`count` this minute's, `day`/`dayCount` this UTC day's. */
type State = { window: number; count: number; day: number; dayCount: number };
/** One visitor's tries today, kept under `visitor:<hash>`. */
type Visitor = { day: number; count: number };
/** The day's random salt for visitor hashes: dropped with the day, so old hashes can't be matched. */
type Salt = { day: number; value: string };

/**
 * One instance per password-protected report (keyed by its id), so the counts are global, unlike the
 * rate-limiting binding, which counts per data centre. A try is reserved before the password is
 * hashed, so parallel guesses can't race past the caps; a right password gives its try back.
 *
 * Visitors are counted by a salted hash of their address, never the address itself, and every count
 * is cleared once its day is over.
 *
 * It also keeps the report's unlock passes: random tokens handed to a browser after a right
 * password, stored here only as SHA-256 hashes (`pass:<hash>` → expiry in ms). A pass skips the
 * guess limits, so someone burning through wrong guesses can't lock out people who already opened it.
 */
export class UnlockGuard extends DurableObject<Env> {
  /** Reserves a try for `visitor`: 0 when allowed, otherwise the seconds to wait (the minute's or the day's end). */
  async begin(visitor: string, now = Date.now()): Promise<number> {
    const window = Math.floor(now / WINDOW_MS);
    const day = Math.floor(now / DAY_MS);
    const untilTomorrow = Math.ceil(((day + 1) * DAY_MS - now) / 1000);
    // One call at a time from reading the counts to writing them back, so parallel guesses can't
    // race past the caps (hashing the visitor would otherwise let them interleave).
    return this.ctx.blockConcurrencyWhile(async () => {
      const key = await this.visitorKey(visitor, day);
      const saved = await this.ctx.storage.get<State>("state");
      const sameDay = saved !== undefined && saved.day === day;
      const state: State = { window, count: sameDay && saved.window === window ? saved.count : 0, day, dayCount: sameDay ? saved.dayCount : 0 };
      const mine = await this.ctx.storage.get<Visitor>(key);
      const own: Visitor = { day, count: mine?.day === day ? mine.count : 0 };

      if (own.count >= GUESSES_PER_VISITOR_PER_DAY || state.dayCount >= GUESSES_PER_DAY) return untilTomorrow;
      if (state.count >= GUESSES_PER_MINUTE) return 60;
      state.count += 1;
      state.dayCount += 1;
      own.count += 1;
      await this.ctx.storage.put({ state, [key]: own });
      await this.cleanUpBy((day + 1) * DAY_MS);
      return 0;
    });
  }

  /** Gives back the try a right password reserved. */
  async succeeded(visitor: string, now = Date.now()): Promise<void> {
    const day = Math.floor(now / DAY_MS);
    await this.ctx.blockConcurrencyWhile(async () => {
      const key = await this.visitorKey(visitor, day);
      const state = await this.ctx.storage.get<State>("state");
      if (state && state.count > 0) await this.ctx.storage.put("state", { ...state, count: state.count - 1, dayCount: Math.max(0, state.dayCount - 1) });
      const mine = await this.ctx.storage.get<Visitor>(key);
      if (mine && mine.count > 0) await this.ctx.storage.put(key, { ...mine, count: mine.count - 1 });
    });
  }

  /** A new pass, valid for 24 hours but never past `notAfter` (the report's own expiry, in ms). */
  async issuePass(notAfter: number): Promise<{ pass: string; expiresAt: number }> {
    const pass = base64url(crypto.getRandomValues(new Uint8Array(32)));
    const expiresAt = Math.min(Date.now() + PASS_MS, notAfter);
    await this.ctx.storage.put(`pass:${await hashToken(pass)}`, expiresAt);
    await this.cleanUpBy(expiresAt);
    return { pass, expiresAt };
  }

  /**
   * Whether `pass` is a live pass; a live one is renewed for another 24 hours (never past `notAfter`),
   * so people who keep coming back are never locked out. Returns its new expiry, or null.
   */
  async usePass(pass: string, notAfter: number): Promise<number | null> {
    // Looked up by its hash, so no stored secret is ever compared.
    const key = `pass:${await hashToken(pass)}`;
    const expiresAt = await this.ctx.storage.get<number>(key);
    if (expiresAt === undefined || expiresAt <= Date.now()) return null;
    const renewed = Math.max(expiresAt, Math.min(Date.now() + PASS_MS, notAfter));
    await this.ctx.storage.put(key, renewed);
    return renewed;
  }

  /** Drops a past day's counts and salt and expired passes, and wakes again for whichever ends next. */
  async alarm(): Promise<void> {
    const now = Date.now();
    const today = Math.floor(now / DAY_MS);
    let next = Infinity;
    const state = await this.ctx.storage.get<State>("state");
    // A count saved before the daily caps has no day: it's from an earlier day either way.
    if (state && !(state.day >= today)) await this.ctx.storage.delete("state");
    else if (state) next = (state.day + 1) * DAY_MS;
    const salt = await this.ctx.storage.get<Salt>("salt");
    if (salt && salt.day < today) await this.ctx.storage.delete("salt");
    for (const [key, visitor] of await this.ctx.storage.list<Visitor>({ prefix: "visitor:" })) {
      if (visitor.day < today) await this.ctx.storage.delete(key);
      else next = Math.min(next, (visitor.day + 1) * DAY_MS);
    }
    for (const [key, expiresAt] of await this.ctx.storage.list<number>({ prefix: "pass:" })) {
      if (expiresAt <= now) await this.ctx.storage.delete(key);
      else next = Math.min(next, expiresAt);
    }
    if (next !== Infinity) await this.ctx.storage.setAlarm(next);
  }

  /** `visitor:<hash>`: the address hashed with the day's salt, so storage never holds an address. */
  private async visitorKey(visitor: string, day: number): Promise<string> {
    let salt = await this.ctx.storage.get<Salt>("salt");
    if (!salt || salt.day !== day) {
      salt = { day, value: base64url(crypto.getRandomValues(new Uint8Array(16))) };
      await this.ctx.storage.put("salt", salt);
    }
    return `visitor:${(await hashToken(`${salt.value}:${visitor}`)).slice(0, 32)}`;
  }

  private async cleanUpBy(when: number): Promise<void> {
    const current = await this.ctx.storage.getAlarm();
    if (current === null || when < current) await this.ctx.storage.setAlarm(when);
  }
}
