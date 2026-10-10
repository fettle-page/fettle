export const LOCALES = ["en", "it"] as const;
export type Locale = (typeof LOCALES)[number];

/** "it-IT" → "it"; anything Fettle doesn't speak is English. */
export function toLocale(tag: string): Locale {
  const lang = tag.slice(0, 2).toLowerCase();
  return (LOCALES as readonly string[]).includes(lang) ? (lang as Locale) : "en";
}
