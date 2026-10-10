// Share page copy, in English and Italian.
import type { Locale } from "@fettle/brain";

const en = {
  label: "Shared receipt",
  expires: "This link expires on {date}.",
  "missing.title": "This receipt isn't here.",
  "missing.body": "It may have expired, or its owner deleted it.",
  cta: "Check your own PC with Fettle",
  "delete.title": "Delete this receipt?",
  "delete.body": "The link will stop working for everyone.",
  "delete.button": "Delete this receipt",
  "delete.done": "Deleted. The link no longer works.",
  "delete.no_token": "This delete link is incomplete. Copy it again from Fettle.",
  "delete.forbidden": "This delete link doesn't match this receipt.",
  "delete.gone": "This receipt is already gone.",
  offline: "Couldn't reach Fettle. Try again.",
  failed: "Something went wrong. Try again in a moment.",
  "locked.title": "This receipt is password protected.",
  "locked.label": "Password",
  "locked.button": "Open receipt",
  "locked.wrong": "That password isn't right.",
  "locked.limited": "Too many tries. Wait a minute and try again.",
  footer: "Fettle is a free check-up for Windows PCs.",
  "theme.label": "Theme",
  "theme.auto": "Auto",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "theme.oled": "OLED",
};

export type ShareKey = keyof typeof en;

const it: Record<ShareKey, string> = {
  label: "Scontrino condiviso",
  expires: "Questo link scade il {date}.",
  "missing.title": "Questo scontrino non c'è.",
  "missing.body": "Potrebbe essere scaduto, oppure chi l'ha condiviso l'ha eliminato.",
  cta: "Controlla il tuo PC con Fettle",
  "delete.title": "Eliminare questo scontrino?",
  "delete.body": "Il link smetterà di funzionare per tutti.",
  "delete.button": "Elimina lo scontrino",
  "delete.done": "Eliminato. Il link non funziona più.",
  "delete.no_token": "Questo link di eliminazione è incompleto. Copialo di nuovo da Fettle.",
  "delete.forbidden": "Questo link di eliminazione non corrisponde allo scontrino.",
  "delete.gone": "Questo scontrino non c'è già più.",
  offline: "Impossibile raggiungere Fettle. Riprova.",
  failed: "Qualcosa non ha funzionato. Riprova tra poco.",
  "locked.title": "Questo scontrino è protetto da password.",
  "locked.label": "Password",
  "locked.button": "Apri lo scontrino",
  "locked.wrong": "La password non è corretta.",
  "locked.limited": "Troppi tentativi. Aspetta un minuto e riprova.",
  footer: "Fettle è un controllo gratuito per PC Windows.",
  "theme.label": "Tema",
  "theme.auto": "Auto",
  "theme.light": "Chiaro",
  "theme.dark": "Scuro",
  "theme.oled": "OLED",
};

const catalogs: Record<Locale, Record<ShareKey, string>> = { en, it };
const INTL_LOCALE: Record<Locale, string> = { en: "en-GB", it: "it-IT" };

export function shareText(key: ShareKey, locale: Locale, params: Record<string, string> = {}): string {
  return catalogs[locale][key].replace(/\{(\w+)\}/g, (_, name: string) => params[name] ?? `{${name}}`);
}

/** "6 November 2026" / "6 novembre 2026". UTC by default, so the server-drawn page and the first
 *  browser render agree; pass `null` for the visitor's own time zone. */
export function longDate(iso: string, locale: Locale, timeZone: string | null = "UTC"): string {
  return new Intl.DateTimeFormat(INTL_LOCALE[locale], { dateStyle: "long", ...(timeZone && { timeZone }) }).format(new Date(iso));
}
