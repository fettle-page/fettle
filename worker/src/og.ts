// Open Graph text for share links, in English and Italian. Built from the verdicts only: no names,
// models or other report text.
import { text, type Level, type Locale, type Receipt } from "@fettle/brain";

type Counted = Exclude<Level, "info">;
const ORDER: Counted[] = ["act_soon", "keep_an_eye", "couldnt_check", "fine"];

const COUNT: Record<Locale, Record<Counted, (n: number) => string>> = {
  en: {
    act_soon: (n) => `${n} to act on soon`,
    keep_an_eye: (n) => `${n} to keep an eye on`,
    couldnt_check: (n) => `${n} couldn't check`,
    fine: (n) => `${n} fine`,
  },
  it: {
    act_soon: (n) => `${n} da sistemare presto`,
    keep_an_eye: (n) => `${n} da tenere d'occhio`,
    couldnt_check: (n) => `${n} ${n === 1 ? "non verificato" : "non verificati"}`,
    fine: (n) => `${n} a posto`,
  },
};

const TITLE: Record<Locale, string> = { en: "Fettle receipt", it: "Scontrino Fettle" };
const NOTHING_COUNTED: Record<Locale, string> = { en: "A PC check-up by Fettle.", it: "Un controllo del PC fatto con Fettle." };

export type OgText = { title: string; description: string };

export const LOCKED_OG: Record<Locale, OgText> = {
  en: { title: "Fettle receipt (password protected)", description: "Open the link and enter the password to see it." },
  it: { title: "Scontrino Fettle (protetto da password)", description: "Apri il link e inserisci la password per vederlo." },
};

export function ogText(receipt: Receipt, locale: Locale): OgText {
  const counts = new Map<Level, number>();
  for (const section of receipt.sections) for (const line of section.lines) counts.set(line.level, (counts.get(line.level) ?? 0) + 1);
  const parts = ORDER.filter((level) => (counts.get(level) ?? 0) > 0).map((level) => COUNT[locale][level](counts.get(level)!));
  return {
    title: `${TITLE[locale]}: ${text(receipt.summary.key, locale)}`,
    description: parts.length > 0 ? parts.join(" · ") : NOTHING_COUNTED[locale],
  };
}

const HTML_ESCAPES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => HTML_ESCAPES[c]!);
}

/** JSON that can't close its <script> element or break a parser: <, >, &, U+2028 and U+2029 become escapes. */
export function inlineJson(value: unknown): string {
  return JSON.stringify(value)
    .replaceAll("<", "\\u003c")
    .replaceAll(">", "\\u003e")
    .replaceAll("&", "\\u0026")
    .replaceAll(" ", "\\u2028")
    .replaceAll(" ", "\\u2029");
}
