// Releases, newest first. An entry shows on the changelog page once it has a date: write the notes
// ahead, and date them on release day (src/release.ts lists the steps).
import type { Locale } from "../i18n/paths.ts";

export type Release = { version: string; date: string | null; notes: Record<Locale, string[]> };

export const releases: Release[] = [
  {
    version: "1.0.3",
    date: null,
    notes: {
      en: [
        "On Windows 10, Fettle no longer says a PC is enrolled in Extended Security Updates just because it installed an update late. It now reads enrolment from the Windows build, and when it can't tell, it asks you to check Windows Update.",
      ],
      it: [
        "Su Windows 10, Fettle non dice più che un PC è iscritto agli Extended Security Updates solo perché ha installato un aggiornamento in ritardo. Ora lo ricava dalla build di Windows e, quando non può saperlo, ti chiede di controllare in Windows Update.",
      ],
    },
  },
  {
    version: "1.0.2",
    date: "2026-10-11",
    notes: {
      en: [
        "Wear on SATA SSDs is read the way each maker defines it. Before, a drive built on a SandForce controller (such as a Kingston SV300) could wrongly show as 100% worn.",
        "Power-on hours are right on drives that pack other counters into that value, and impossible values are no longer shown.",
        "After a Deep check, a PC without a TPM chip says so, instead of saying it couldn't tell.",
        "A share link with a password now takes at most 100 wrong passwords a day, on top of 10 a minute, so nobody can keep guessing it.",
      ],
      it: [
        "L'usura degli SSD SATA viene letta come la definisce ciascun produttore. Prima un disco con controller SandForce (come un Kingston SV300) poteva risultare usurato al 100% per errore.",
        "Le ore di accensione sono corrette anche sui dischi che inseriscono altri contatori in quel valore, e i valori impossibili non vengono più mostrati.",
        "Dopo un Controllo approfondito, un PC senza chip TPM lo dice, invece di dire che non è riuscito a verificarlo.",
        "Un link condiviso con password ora accetta al massimo 100 password sbagliate al giorno, oltre alle 10 al minuto, così nessuno può continuare a tentare.",
      ],
    },
  },
  {
    version: "1.0.1",
    date: "2026-10-10",
    notes: {
      en: ["A drive that reports a problem itself (an NVMe warning flag or low spare capacity) is now marked ACT, the most urgent level. Before, those flags were read but never used.", "Two disks of the same model each get their own health data, instead of both showing the first one's.", "Fettle opens disks read-only, and stops as soon as it has read their health.", "Before it installs anything, the installer shows what Fettle sends (only the update check) and offers a box to turn the update check off.", "Opening the JSON in Share no longer spreads the view apart.", "A receipt with three next steps no longer draws the first section over the third step.", "After a check, the side panel says “This check” instead of “Last check”.", "When a link is ready, it comes on a ticket with a QR code to open it on a phone, and the delete link has its own Copy button.", "My shared links can now check which links still work, and drop the ones deleted on the website.", "The Deep check now runs in the same window, and on a standard account it shows your own startup apps rather than the administrator's.", "Installing for all users with the update check cleared now turns it off for every account on the PC; anyone can still turn it on in Settings.", "Italian dates read correctly: «l'8», «all'11», «il 1°».", "SATA SSDs now get the same wear warnings as NVMe drives.", "A Quick check that couldn't read the disks no longer says “In fine fettle”: it says a Deep check is still needed, and lists it as a next step.", "Fettle now warns in the last 60 days before your version of Windows stops getting security updates, and says what to do.", "Hard drives no longer get SSD wear warnings from a SMART value that means something else on them.", "An NVMe drive that is only running hot is no longer reported as failing; the temperature line covers it.", "Fettle now also warns in the last 60 days before Extended Security Updates end.", "Upgrading Fettle no longer turns the update check back off after you turned it on in Settings.", "The Deep check leaves no files behind, even if Fettle is closed during it.", "The Deep check's time no longer includes the time spent on Windows' prompt.", "Memory details now name DDR3, and the LPDDR3, LPDDR4 and LPDDR5 memory soldered into thin laptops.", "Fettle now knows the support dates of Windows 11 LTSC 2024, Windows 10 LTSB 2015 and 2016, Windows 10 IoT Enterprise LTSC 2019 and Windows IoT Enterprise, instead of calling them an unknown version.", "A small partition with a drive letter (a recovery partition, say) no longer gets an “almost full” warning.", "When printing, the footer no longer ends up alone on a last page."],
      it: ["Un disco che segnala da solo un problema (un avviso NVMe o la riserva bassa) ora viene segnato AGISCI, il livello più urgente. Prima quegli avvisi venivano letti ma non usati.", "Due dischi dello stesso modello ricevono ora ciascuno i propri dati sullo stato, invece di mostrare entrambi quelli del primo.", "Fettle apre i dischi in sola lettura e si ferma appena ne ha letto lo stato.", "Prima di installare, l'installer mostra cosa invia Fettle (solo il controllo degli aggiornamenti) e offre una casella per disattivarlo.", "Aprire il JSON in Condividi non allarga più la vista.", "Uno scontrino con tre passi successivi non disegna più la prima sezione sopra il terzo.", "Dopo un controllo, il pannello laterale dice «Questo controllo» invece di «Ultimo controllo».", "Quando un link è pronto, arriva su un biglietto con un codice QR per aprirlo dal telefono, e il link di eliminazione ha il suo pulsante Copia.", "In I miei link condivisi ora puoi controllare quali link funzionano ancora; quelli eliminati dal sito vengono tolti dall'elenco.", "Il Controllo approfondito ora avviene nella stessa finestra e, su un account standard, mostra le tue app all'avvio invece di quelle dell'amministratore.", "Installando per tutti gli utenti con il controllo degli aggiornamenti disattivato, ora resta spento per ogni account del PC; ognuno può comunque attivarlo nelle Impostazioni.", "Le date in italiano ora usano l'articolo giusto: «l'8», «all'11», «il 1°».", "Anche gli SSD SATA ricevono ora gli avvisi di usura, come quelli NVMe.", "Un Controllo rapido che non è riuscito a leggere i dischi non dice più «In gran forma»: segnala che serve ancora un Controllo approfondito e lo indica tra le cose da fare.", "Fettle ora avvisa negli ultimi 60 giorni prima che la tua versione di Windows smetta di ricevere aggiornamenti di sicurezza, e dice cosa fare.", "I dischi rigidi non ricevono più avvisi di usura da SSD per un valore SMART che su di loro significa altro.", "Un disco NVMe che è solo troppo caldo non viene più segnalato come guasto: basta l'avviso sulla temperatura.", "Fettle ora avvisa anche negli ultimi 60 giorni prima della fine degli Extended Security Updates.", "Aggiornare Fettle non spegne più il controllo degli aggiornamenti se l'avevi attivato nelle Impostazioni.", "Il Controllo approfondito non lascia più file, anche se Fettle viene chiuso mentre è in corso.", "Il tempo del Controllo approfondito non include più l'attesa sul prompt di Windows.", "I dettagli della memoria ora indicano anche la DDR3 e la memoria LPDDR3, LPDDR4 e LPDDR5 saldata nei portatili sottili.", "Fettle ora conosce le date di supporto di Windows 11 LTSC 2024, Windows 10 LTSB 2015 e 2016, Windows 10 IoT Enterprise LTSC 2019 e Windows IoT Enterprise, invece di indicarle come versione sconosciuta.", "Una piccola partizione con una lettera di unità (per esempio quella di ripristino) non riceve più l'avviso «quasi piena».", "In stampa, il piè di pagina non finisce più da solo su un'ultima pagina."],
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
