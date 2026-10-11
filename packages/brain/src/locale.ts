export const LOCALES = ["en", "it"] as const;
export type Locale = (typeof LOCALES)[number];

/**
 * The language of a tag ("it", "IT", "it-IT", "it_CH" → "it"), or null when Fettle doesn't speak it.
 * Only the whole first subtag counts, so "items" or "english" is no language at all.
 */
export function localeOf(tag: string): Locale | null {
  const lang = tag.trim().toLowerCase().split(/[-_]/)[0] ?? "";
  return (LOCALES as readonly string[]).includes(lang) ? (lang as Locale) : null;
}

/** "it-IT" → "it"; anything Fettle doesn't speak is English. */
export function toLocale(tag: string): Locale {
  return localeOf(tag) ?? "en";
}

/**
 * The languages in an Accept-Language header, most wanted first: by q-value, the header's own order
 * for ties. A q of 0 means "not this one", and an unreadable q counts as 0.
 */
export function acceptedLanguages(header: string | null | undefined): string[] {
  return (header ?? "")
    .split(",")
    .map((part, i) => {
      const [tag = "", ...params] = part.split(";");
      const raw = params.map((p) => p.trim()).find((p) => p.startsWith("q="))?.slice(2) ?? "1";
      const q = raw === "" ? 0 : Number(raw);
      return { tag: tag.trim(), q: Number.isFinite(q) ? q : 0, i };
    })
    .filter((l) => l.tag !== "" && l.q > 0)
    .sort((a, b) => b.q - a.q || a.i - b.i)
    .map((l) => l.tag);
}

/**
 * The language for a page: `asked` (a `?lang=` value) when it names one Fettle speaks, otherwise the
 * first of `preferred` (Accept-Language on the server, navigator.languages in the browser) that it
 * speaks, otherwise English. The share server and the share page both decide with this.
 */
export function pickLocale(asked: string | null | undefined, preferred: readonly string[]): Locale {
  for (const tag of [asked ?? "", ...preferred]) {
    const locale = localeOf(tag);
    if (locale) return locale;
  }
  return "en";
}
