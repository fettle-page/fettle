// Releases, newest first. An entry shows on the changelog page once it has a date: write the notes
// ahead, and date them on release day (src/release.ts lists the steps).
import type { Locale } from "../i18n/paths.ts";

export type Release = { version: string; date: string | null; notes: Record<Locale, string[]> };

export const releases: Release[] = [
  {
    version: "1.0.1",
    date: null,
    notes: {
      en: ["Before it installs anything, the installer shows what Fettle sends (only the update check) and offers a box to turn the update check off.", "Opening the JSON in Share no longer spreads the view apart.", "A receipt with three next steps no longer draws the first section over the third step.", "After a check, the side panel says “This check” instead of “Last check”.", "When a link is ready, it comes on a ticket with a QR code to open it on a phone, and the delete link has its own Copy button.", "My shared links can now check which links still work, and drop the ones deleted on the website.", "The Deep check now runs in the same window, and on a standard account it shows your own startup apps rather than the administrator's.", "Installing for all users with the update check cleared now turns it off for every account on the PC; anyone can still turn it on in Settings.", "Italian dates read correctly: «l'8», «all'11», «il 1°»."],
      it: ["Prima di installare, l'installer mostra cosa invia Fettle (solo il controllo degli aggiornamenti) e offre una casella per disattivarlo.", "Aprire il JSON in Condividi non allarga più la vista.", "Uno scontrino con tre passi successivi non disegna più la prima sezione sopra il terzo.", "Dopo un controllo, il pannello laterale dice «Questo controllo» invece di «Ultimo controllo».", "Quando un link è pronto, arriva su un biglietto con un codice QR per aprirlo dal telefono, e il link di eliminazione ha il suo pulsante Copia.", "In I miei link condivisi ora puoi controllare quali link funzionano ancora; quelli eliminati dal sito vengono tolti dall'elenco.", "Il controllo approfondito ora avviene nella stessa finestra e, su un account standard, mostra le tue app all'avvio invece di quelle dell'amministratore.", "Installando per tutti gli utenti con il controllo degli aggiornamenti disattivato, ora resta spento per ogni account del PC; ognuno può comunque attivarlo nelle Impostazioni.", "Le date in italiano ora usano l'articolo giusto: «l'8», «all'11», «il 1°»."],
    },
  },
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
