// Releases, newest first. An entry shows on the changelog page once it has a date: write the notes
// ahead, and date them on release day (src/release.ts lists the steps).
import type { Locale } from "../i18n/paths.ts";

export type Release = { version: string; date: string | null; notes: Record<Locale, string[]> };

export const releases: Release[] = [
  {
    version: "1.0.0",
    date: "2026-10-10",
    notes: {
      en: [
        "The first release. One button checks the disks, battery, memory, Windows, startup apps and graphics, and prints a receipt with a plain verdict on every line.",
        "It needs no administrator rights. The Deep check asks for them only to read the health of your disks and the security chip.",
        "Print the receipt or save it as a PDF, save the full report as JSON, or copy it for Discord or Reddit.",
        "Share a receipt as a link that lasts 7 days, 30 days or a year, with an optional password and a link to delete it.",
        "In English and Italian, with light, dark and OLED themes.",
        "An installer or a portable zip, with an update check you can turn off.",
      ],
      it: [
        "La prima versione. Un pulsante controlla dischi, batteria, memoria, Windows, app all'avvio e grafica, e stampa uno scontrino con un verdetto chiaro su ogni riga.",
        "Non servono diritti di amministratore. Il Controllo approfondito li chiede solo per leggere lo stato dei dischi e il chip di sicurezza.",
        "Stampa lo scontrino o salvalo in PDF, salva il report completo in JSON, o copialo per Discord o Reddit.",
        "Condividi uno scontrino con un link che dura 7 giorni, 30 giorni o un anno, con una password facoltativa e un link per eliminarlo.",
        "In inglese e in italiano, con i temi chiaro, scuro e OLED.",
        "Un installer o uno zip portatile, con un controllo degli aggiornamenti che puoi disattivare.",
      ],
    },
  },
];
