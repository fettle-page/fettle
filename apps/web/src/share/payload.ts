// What the Worker puts in the share shell's <script id="fettle-share" type="application/json">.
// The same contract binds worker/src/share.ts.
import { LOCALES, type Locale, type ReceiptModel } from "@fettle/brain";

/**
 * `locale` is decided by the server, so a server-drawn page hydrates in the same language.
 * A report always comes as `model`: the finished receipt, built once by the server (the Rust brain),
 * which also drew it. The browser never builds one.
 */
export type SharePayload =
  | { kind: "report"; id: string; expiresAt: string; locale?: Locale; model: ReceiptModel }
  | { kind: "locked"; id: string; expiresAt: string; locale?: Locale }
  | { kind: "missing"; locale?: Locale }
  | { kind: "delete"; id: string; locale?: Locale };

const MISSING: SharePayload = { kind: "missing" };
const isString = (v: unknown): v is string => typeof v === "string" && v.length > 0;
const isLocale = (v: unknown): v is Locale => typeof v === "string" && (LOCALES as readonly string[]).includes(v);

/** A timestamp the page can format; "x" would make Intl throw. */
export const isDate = (v: unknown): v is string => typeof v === "string" && !Number.isNaN(Date.parse(v));

/** Enough of a receipt model to render; anything else falls back to the "isn't here" page. */
export function isModel(v: unknown): v is ReceiptModel {
  if (typeof v !== "object" || v === null) return false;
  const m = v as Partial<ReceiptModel>;
  return isLocale(m.locale) && typeof m.summary === "string" && Array.isArray(m.sections) && typeof m.meta?.collectedAt === "string" && typeof m.labels === "object";
}

/** Reads the payload; anything absent, broken or not a valid report shows the "isn't here" page. */
export function readPayload(doc: Document): SharePayload {
  const text = doc.getElementById("fettle-share")?.textContent;
  if (!text) return MISSING;
  let data: Record<string, unknown>;
  try {
    data = JSON.parse(text);
  } catch {
    return MISSING;
  }
  if (typeof data !== "object" || data === null) return MISSING;
  const { kind, id, expiresAt } = data;
  const lang = isLocale(data.locale) ? { locale: data.locale } : {};
  if (kind === "missing") return { kind, ...lang };
  if (!isString(id)) return MISSING;
  if (kind === "delete") return { kind, id, ...lang };
  if (!isDate(expiresAt)) return MISSING;
  if (kind === "locked") return { kind, id, expiresAt, ...lang };
  if (kind === "report") return isModel(data.model) ? { kind, id, expiresAt, ...lang, model: data.model } : MISSING;
  return MISSING;
}

/** `?lang=` wins, then the first browser language Fettle speaks, then English. */
export function pickLocale(search: string, languages: readonly string[]): Locale {
  const wanted = [new URLSearchParams(search).get("lang") ?? "", ...languages];
  for (const tag of wanted) {
    const lang = tag.slice(0, 2).toLowerCase();
    if ((LOCALES as readonly string[]).includes(lang)) return lang as Locale;
  }
  return "en";
}
