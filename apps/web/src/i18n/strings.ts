// Website copy, in English and Italian.
// Voice: sentence case, no exclamation marks, nothing scary.
import type { Locale } from "./paths.ts";

const en = {
  "meta.site": "Fettle",
  "skip": "Skip to content",
  "nav.label": "Main",
  "nav.how": "How it works",
  "nav.privacy": "Privacy",
  "nav.faq": "FAQ",
  "nav.download": "Download",
  "nav.other_language": "Italiano",
  "footer.tagline": "A free check-up for Windows PCs.",
  "footer.small": "Free · open source (MIT) · no cookies · no tracking",
  "downloads.one": "Downloaded once",
  "downloads.many": "Downloaded {n} times",
  "footer.links": "Site",
  "footer.changelog": "Changelog",
  "footer.source": "Source code",
  "footer.signing": "Code signing policy",
  "faq.write": "Write to us at",
  "footer.made": "Made with care, one receipt at a time.",
  "theme.label": "Theme",
  "theme.auto": "Auto",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "theme.oled": "OLED",
};

export type Key = keyof typeof en;

const it: Record<Key, string> = {
  "meta.site": "Fettle",
  "skip": "Vai al contenuto",
  "nav.label": "Principale",
  "nav.how": "Come funziona",
  "nav.privacy": "Privacy",
  "nav.faq": "FAQ",
  "nav.download": "Scarica",
  "nav.other_language": "English",
  "footer.tagline": "Un controllo gratuito per PC Windows.",
  "footer.small": "Gratis · open source (MIT) · niente cookie · nessun tracciamento",
  "downloads.one": "Scaricato una volta",
  "downloads.many": "Scaricato {n} volte",
  "footer.links": "Sito",
  "footer.changelog": "Novità",
  "footer.source": "Codice sorgente",
  "footer.signing": "Policy di firma del codice",
  "faq.write": "Scrivici a",
  "footer.made": "Fatto con cura, uno scontrino alla volta.",
  "theme.label": "Tema",
  "theme.auto": "Auto",
  "theme.light": "Chiaro",
  "theme.dark": "Scuro",
  "theme.oled": "OLED",
};

const catalogs: Record<Locale, Record<Key, string>> = { en, it };

export function t(locale: Locale, key: Key): string {
  return catalogs[locale][key];
}
