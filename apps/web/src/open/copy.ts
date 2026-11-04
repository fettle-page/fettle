// Copy for /open (a saved report opened in the browser), in English and Italian. The page and its
// small up-front script both read it, so every message is ready before anything else loads.
import type { Locale } from "../i18n/paths.ts";

const en = {
  title: "Open a saved report — Fettle",
  description: "Open a report saved from Fettle and see its receipt. Your browser reads the file on this computer: it isn't uploaded.",
  kicker: "Open a report",
  h1: "Open a saved report.",
  intro: "Got a .json file from Fettle? Open it here to see its receipt, the same one the app showed.",
  promise: "The file stays on this computer. Your browser reads it right here, and it isn't uploaded or sent anywhere.",
  pick: "Choose a file",
  again: "Open another file",
  drop: "or drop it here",
  how: "To save one in the app, press Save and choose \"Report for experts (JSON)\" as the file type.",
  noscript: "This page needs JavaScript to read the file.",
  reading: "Reading the file…",
  opened: "Opened {name}. The receipt is below.",
  receipt: "The receipt",
  "error.not_json": "This file isn't a saved report: it isn't JSON. Choose the .json file Fettle saved.",
  "error.not_fettle": "This file isn't a Fettle report. Choose the .json file Fettle saved.",
  "error.newer": "A newer version of Fettle made this report, and this page can't read it yet. Try again later.",
  "error.unreadable": "This report couldn't be read. It may be damaged: save it again from Fettle and try once more.",
  "error.offline": "Couldn't load the part of this page that draws the receipt. Check your connection and try again.",
};

export type OpenKey = keyof typeof en;

const it: Record<OpenKey, string> = {
  title: "Apri un report salvato — Fettle",
  description: "Apri un report salvato da Fettle e guardane lo scontrino. Il browser legge il file su questo computer: non viene caricato.",
  kicker: "Apri un report",
  h1: "Apri un report salvato.",
  intro: "Hai un file .json di Fettle? Aprilo qui per vederne lo scontrino, lo stesso che mostrava l'app.",
  promise: "Il file resta su questo computer. Lo legge il browser, qui, e non viene caricato né inviato da nessuna parte.",
  pick: "Scegli un file",
  again: "Apri un altro file",
  drop: "o trascinalo qui",
  how: "Per salvarne uno dall'app, premi Salva e scegli \"Report per esperti (JSON)\" come tipo di file.",
  noscript: "Per leggere il file, questa pagina ha bisogno di JavaScript.",
  reading: "Lettura del file…",
  opened: "Hai aperto {name}. Lo scontrino è qui sotto.",
  receipt: "Lo scontrino",
  "error.not_json": "Questo file non è un report salvato: non è in formato JSON. Scegli il file .json salvato da Fettle.",
  "error.not_fettle": "Questo file non è un report di Fettle. Scegli il file .json salvato da Fettle.",
  "error.newer": "Questo report è stato creato da una versione più recente di Fettle e questa pagina non riesce ancora a leggerlo. Riprova più tardi.",
  "error.unreadable": "Non è stato possibile leggere questo report. Potrebbe essere danneggiato: salvalo di nuovo da Fettle e riprova.",
  "error.offline": "Non è stato possibile caricare la parte della pagina che disegna lo scontrino. Controlla la connessione e riprova.",
};

const catalogs: Record<Locale, Record<OpenKey, string>> = { en, it };

export function openText(locale: Locale, key: OpenKey, params: Record<string, string> = {}): string {
  return catalogs[locale][key].replace(/\{(\w+)\}/g, (_, name: string) => params[name] ?? `{${name}}`);
}
