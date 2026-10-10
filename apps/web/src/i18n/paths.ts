// English lives at the root, Italian under /it.
export type Locale = "en" | "it";
export const LOCALES: Locale[] = ["en", "it"];

export function localePath(locale: Locale, path: string): string {
  if (locale === "en") return path;
  return path === "/" ? "/it" : `/it${path}`;
}

export function alternates(path: string): Record<Locale, string> {
  return { en: localePath("en", path), it: localePath("it", path) };
}

export function localeFromPath(pathname: string): Locale {
  return pathname === "/it" || pathname.startsWith("/it/") ? "it" : "en";
}

/** The page path without its language prefix: "/it/faq" → "/faq", "/it" → "/". */
export function basePath(pathname: string): string {
  if (pathname === "/it") return "/";
  return pathname.startsWith("/it/") ? pathname.slice(3) : pathname;
}
