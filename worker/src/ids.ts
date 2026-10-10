// Report ids: one letter for the expiry, then 16 random characters (80 bits), so a link
// can't be guessed and the R2 key is found with a single read.

export type Ttl = "7d" | "30d" | "1y";

export const TTL_DAYS: Record<Ttl, number> = { "7d": 7, "30d": 30, "1y": 365 };

const LETTER: Record<Ttl, string> = { "7d": "w", "30d": "m", "1y": "y" };
const TTL_OF: Record<string, Ttl> = { w: "7d", m: "30d", y: "1y" };

// 32 symbols, without the look-alikes 1, i, l and o.
const ALPHABET = "023456789abcdefghjkmnpqrstuvwxyz";
const RANDOM_CHARS = 16;
const ID_SHAPE = /^[wmy][023456789abcdefghjkmnpqrstuvwxyz]{16}$/;

export function newId(ttl: Ttl): string {
  const bytes = crypto.getRandomValues(new Uint8Array(RANDOM_CHARS));
  let id = LETTER[ttl];
  for (const b of bytes) id += ALPHABET[b & 31];
  return id;
}

export function isValidId(id: string): boolean {
  return ID_SHAPE.test(id);
}

/** Only for ids that passed `isValidId`. */
export function ttlOfId(id: string): Ttl {
  const ttl = TTL_OF[id[0] ?? ""];
  if (!ttl) throw new RangeError(`not a report id: ${id}`);
  return ttl;
}

export function keyFor(id: string): string {
  return `${ttlOfId(id)}/${id}`;
}
