import { DurableObject } from "cloudflare:workers";
import { hashToken } from "./tokens.ts";

/**
 * Share-link uploads one address (an IP, or an IPv6 /64) may make per day (UTC). The per-minute
 * limiter alone would allow 14,400 a day, each kept for up to a year; 100 is far more than one
 * person, or a repair shop sharing many PCs, needs.
 */
export const UPLOADS_PER_DAY = 100;
const DAY_MS = 86_400_000;

type Used = { day: number; count: number };

/**
 * The budget's name for an address on a day (UTC): a SHA-256, so no address is ever a Durable
 * Object's name, and a new one every day, so yesterday's empties itself and is never used again.
 */
export async function budgetName(address: string, now = Date.now()): Promise<string> {
  return hashToken(`upload-budget:${Math.floor(now / DAY_MS)}:${address}`);
}

/** One instance per address. It holds one count, cleared once its day is over. */
export class UploadBudget extends DurableObject<Env> {
  /** Takes one upload: 0 when allowed, otherwise the seconds until the day is over. */
  async take(now = Date.now()): Promise<number> {
    const day = Math.floor(now / DAY_MS);
    const saved = await this.ctx.storage.get<Used>("used");
    const used: Used = { day, count: saved?.day === day ? saved.count : 0 };
    if (used.count >= UPLOADS_PER_DAY) return Math.ceil(((day + 1) * DAY_MS - now) / 1000);
    used.count += 1;
    await this.ctx.storage.put("used", used);
    const alarm = await this.ctx.storage.getAlarm();
    if (alarm === null) await this.ctx.storage.setAlarm((day + 1) * DAY_MS);
    return 0;
  }

  async alarm(): Promise<void> {
    const used = await this.ctx.storage.get<Used>("used");
    const today = Math.floor(Date.now() / DAY_MS);
    if (used && used.day >= today) await this.ctx.storage.setAlarm((used.day + 1) * DAY_MS);
    else await this.ctx.storage.deleteAll();
  }
}
