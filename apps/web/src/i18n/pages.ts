// Copy for /faq, /privacy, /changelog, /download, /code-signing and the 404, in English and Italian.
// The privacy wording is a public promise: it changes only when what the app does changes.
import type { Locale } from "./paths.ts";

/** `contact`: the section ends with the contact line (or, until there is one, a note that it's coming). */
export type Section = { h: string; p: string[]; contact?: true };

type Pages = {
  faq: { title: string; description: string; kicker: string; h1: string; intro: string };
  privacy: { title: string; description: string; kicker: string; h1: string; intro: string; updated: string; sections: Section[]; contactLine: string; noContact: string };
  changelog: { title: string; description: string; kicker: string; h1: string; empty: string };
  download: {
    title: string; description: string; kicker: string; h1: string; intro: string; button: string;
    /** Once released: {version} is the app's version. */
    introReady: string; installer: string; portable: string; checksums: string; allReleases: string;
    needsH: string; needs: string[]; formatsH: string; formats: string[]; warningH: string; warning: string[]; signingLink: string;
  };
  codeSigning: { title: string; description: string; kicker: string; h1: string; intro: string; unsigned: Section; signed: Section; sections: Section[]; privacyLink: string };
  /** One page serves every missing address, in both languages (src/scripts/not-found.ts picks one). */
  notFound: { title: string; description: string; h1: string; p: string; line: string; next: string; links: { href: string; label: string }[] };
};

const en: Pages = {
  faq: {
    title: "Questions — Fettle",
    description: "Answers about Fettle, the free Windows check-up: what it reads, what it never changes, share links, privacy and Windows 10.",
    kicker: "FAQ",
    h1: "Questions, answered.",
    intro: "The short version: Fettle only reads, it's free, and it tells you when it isn't sure.",
  },
  privacy: {
    title: "Privacy — Fettle",
    description: "What Fettle does with your data, in plain words: the app sends nothing but an update check you can turn off, share links are redacted and expire, and the website has no cookies.",
    kicker: "Privacy",
    h1: "Privacy, in plain words.",
    intro: "Fettle is built to know as little about you as it can. Here is exactly what happens, and what doesn't.",
    updated: "Last updated: 11 October 2026",
    sections: [
      {
        h: "The app",
        p: [
          "Fettle checks your PC on your PC. It sends nothing anywhere by itself. The one automatic request is the update check: by default once a day (you can choose every hour or every week), it asks this website for the latest version number, sending nothing about your PC. You can turn it off in the settings.",
          "Serial numbers, the PC's name, your Windows username, your name and email address, the work or school domain the PC belongs to, network addresses and the names of servers on your network never appear on receipts, in saved files or in share links. If you like, the app can show the PC's name and serial numbers on your own screen. Product keys are never read at all.",
        ],
      },
      {
        h: "Share links",
        p: [
          "Only when you choose to share, Fettle uploads the report behind the receipt you see, with the private details taken out, and nothing else. Our server checks it again before storing it, and removes any email address, IP address, account ID, server name or profile folder name it finds, even from an older app. It's stored with Cloudflare, in its EU data centres only, for the time you pick (7 days, 30 days or 1 year) and then deleted. A private delete link lets you remove it sooner.",
          "If you add a password, our server keeps only a scrambled hash of it, never the password itself. After a right password it also keeps a scrambled hash of a pass, so the same browser can reopen the receipt; the pass expires 24 hours after it was last used. Anyone with the link, and the password if you set one, can open the receipt.",
        ],
      },
      {
        h: "What our server keeps",
        p: [
          "Our server logs what it did and to which link, for example \"a report was created\" with its size and a short code made from its ID (never the ID itself, which is all it takes to open a link without a password). It doesn't log IP addresses, passwords or report contents.",
          "To stop abuse, it counts uploads, password attempts, share page views, report reads and deletions per IP address, a minute at a time. It also counts each address's uploads for the day, and for a link with a password each visitor's wrong passwords for the day, each under a scrambled code made from the IP address, never the address itself. These counts are deleted when the day ends.",
        ],
      },
      {
        h: "This website",
        p: [
          "No cookies, no analytics, no tracking pixels, and no third-party fonts or scripts: everything loads from this site.",
          "If you open a password-protected receipt, your browser keeps a 24-hour pass in its local storage, so you don't have to type the password again. If you pick a theme (light, dark or OLED), it remembers that too. Nothing else is stored in your browser.",
        ],
      },
      {
        h: "Who runs it",
        p: [
          "Fettle is an open-source project. The website and share links are hosted on Cloudflare, which processes requests on our behalf.",
        ],
        contact: true,
      },
    ],
    contactLine: "Questions about privacy, or anything else? Write to",
    noContact: "Contact details will be added here before the first release.",
  },
  changelog: {
    title: "Changelog — Fettle",
    description: "Every Fettle release, newest first.",
    kicker: "Changelog",
    h1: "What's new.",
    empty: "No releases yet. The first version is on its way; every release will be listed here, newest first.",
  },
  download: {
    title: "Download — Fettle",
    description: "Download Fettle, the free check-up for Windows 10 and 11: an installer (about 12 MB) or a portable zip (about 14 MB), no administrator rights needed.",
    kicker: "Download",
    h1: "Fettle for Windows.",
    intro: "Fettle isn't released yet. The first version is being built and tested on real PCs; when it's ready, it will be right here.",
    button: "Coming soon",
    introReady: "Fettle {version} is ready. Pick the installer, or the portable zip if you'd rather not install anything.",
    installer: "Download the installer",
    portable: "Portable zip",
    checksums: "SHA-256 checksums",
    allReleases: "All releases on GitHub",
    needsH: "What you'll need",
    needs: ["Windows 10 or 11, 64-bit", "About 35 MB of disk space", "No administrator rights (the Deep check asks only if you want it)"],
    formatsH: "Two ways to run it",
    formats: ["Installer, about 12 MB, with an update check you can turn off. To remove it: Windows Settings > Apps, then Fettle > Uninstall.", "Portable zip, about 14 MB: nothing to install, it keeps its settings in its own folder, and it has the same update check you can turn off. To remove it, delete its folder."],
    warningH: "About the Windows warning",
    warning: [
      "New apps without a code-signing certificate get a SmartScreen warning the first time they run, and Fettle's downloads aren't signed yet, so you may see \"Windows protected your PC\".",
      "Choose \"More info\", then \"Run anyway\". The source code is public and every release is built in the open, so you can check what you're running.",
      "Your browser may warn first that the file \"isn't commonly downloaded\": in its downloads list, open the file's menu (…), choose \"Keep\", then \"Show more\" and \"Keep anyway\".",
    ],
    signingLink: "Code signing policy",
  },
  codeSigning: {
    title: "Code signing policy — Fettle",
    description: "How Fettle's downloads are built, checked and signed, and who approves a release.",
    kicker: "Code signing policy",
    h1: "How releases are built and signed.",
    intro: "Every Fettle download is built from the public source by a public build service, never on someone's own PC. Here's how, and who can approve a release.",
    unsigned: {
      h: "Signing",
      p: [
        "Fettle's releases aren't signed yet: a signing certificate costs money, and the free ones for open-source projects go to projects that are already widely known. Fettle is new. Until it's signed, Windows may say the publisher is unknown the first time you run it.",
        "Each release lists the SHA-256 checksum of every download, so you can check that the file you have is the one that was built.",
      ],
    },
    signed: {
      h: "Signing",
      p: [
        "Free code signing provided by SignPath.io, certificate by SignPath Foundation.",
        "Each release also lists the SHA-256 checksum of every download, so you can check that the file you have is the one that was built.",
      ],
    },
    sections: [
      {
        h: "How a release is built",
        p: [
          "A release starts when the maintainer tags a version in the public repository. GitHub Actions then builds the installer and the portable zip from that tag's source, runs every test, and records a build attestation: a signed note of which source and which build made each file.",
          "Nobody builds a release on their own PC, and a release isn't public until the maintainer has looked it over and published it.",
        ],
      },
      {
        h: "Who does what",
        p: [
          "Author and reviewer: candygoogy, who maintains Fettle. Changes from anyone else are reviewed before they're merged.",
          "Approver: candygoogy approves every release before it's published.",
        ],
      },
      {
        h: "Privacy",
        p: [
          "Fettle sends nothing on its own, except the update check: it asks this website for the newest version number, sending nothing about your PC, and you can turn it off in Settings. A receipt is uploaded only when you ask for a share link.",
        ],
      },
    ],
    privacyLink: "Read the privacy page",
  },
  notFound: {
    title: "Not found — Fettle",
    description: "This page isn't here.",
    h1: "This page isn't here.",
    p: "The link may be old, or mistyped.",
    line: "Nothing at this address.",
    next: "Next step",
    links: [
      { href: "/", label: "Check your own PC" },
      { href: "/download", label: "Download Fettle" },
      { href: "/faq", label: "Questions, answered" },
      { href: "/changelog", label: "What's new" },
    ],
  },
};

const it: Pages = {
  faq: {
    title: "Domande — Fettle",
    description: "Risposte su Fettle, il controllo gratuito per Windows: cosa legge, cosa non modifica mai, link condivisi, privacy e Windows 10.",
    kicker: "FAQ",
    h1: "Domande e risposte.",
    intro: "In breve: Fettle legge e basta, è gratis, e ti dice quando non è sicuro.",
  },
  privacy: {
    title: "Privacy — Fettle",
    description: "Cosa fa Fettle con i tuoi dati, in parole semplici: l'app non invia nulla da sola tranne il controllo degli aggiornamenti, i link condivisi nascondono i dati personali e scadono, e il sito non usa cookie.",
    kicker: "Privacy",
    h1: "La privacy, in parole semplici.",
    intro: "Fettle è fatto per sapere di te il meno possibile. Ecco esattamente cosa succede, e cosa no.",
    updated: "Ultimo aggiornamento: 11 ottobre 2026",
    sections: [
      {
        h: "L'app",
        p: [
          "Fettle controlla il tuo PC sul tuo PC. Non invia nulla da solo. L'unica richiesta automatica è il controllo degli aggiornamenti: di norma una volta al giorno (puoi scegliere ogni ora o ogni settimana), chiede a questo sito il numero dell'ultima versione, senza inviare alcuna informazione sul tuo PC. Puoi disattivarlo nelle impostazioni.",
          "Numeri di serie, nome del PC, nome utente di Windows, il tuo nome e il tuo indirizzo email, il dominio aziendale o scolastico del PC, indirizzi di rete e nomi dei server della tua rete non compaiono mai negli scontrini, nei file salvati o nei link condivisi. Se vuoi, l'app può mostrare il nome del PC e i numeri di serie sul tuo schermo. I codici Product Key non vengono mai letti.",
        ],
      },
      {
        h: "Link condivisi",
        p: [
          "Solo quando scegli di condividere, Fettle carica il report da cui nasce lo scontrino che vedi, con i dati personali tolti, e nient'altro. Il nostro server lo ricontrolla prima di conservarlo e toglie ogni indirizzo email, indirizzo IP, ID di account, nome di server o nome di cartella del profilo che trova, anche se arriva da un'app più vecchia. Viene conservato su Cloudflare, solo nei suoi data center nell'UE, per il tempo che scegli (7 giorni, 30 giorni o 1 anno) e poi eliminato. Un link privato di eliminazione ti permette di toglierlo prima.",
          "Se aggiungi una password, il nostro server conserva solo un'impronta (hash), mai la password. Dopo una password corretta conserva anche l'impronta di un permesso, così lo stesso browser può riaprire lo scontrino; il permesso scade 24 ore dall'ultimo utilizzo. Chiunque abbia il link, e la password se l'hai impostata, può aprire lo scontrino.",
        ],
      },
      {
        h: "Cosa conserva il nostro server",
        p: [
          "Il nostro server registra cosa ha fatto e su quale link, per esempio \"è stato creato un report\" con la dimensione e un breve codice ricavato dal suo ID (mai l'ID stesso, che basta per aprire un link senza password). Non registra indirizzi IP, password né il contenuto dei report.",
          "Per evitare abusi, conta i caricamenti, i tentativi di password, le visite alle pagine condivise, le letture e le eliminazioni dei report per indirizzo IP, un minuto alla volta. Conta anche i caricamenti di ogni indirizzo nella giornata e, per un link con password, le password sbagliate di ogni visitatore nella giornata, ciascuno sotto un'impronta (hash) dell'indirizzo IP, mai l'indirizzo stesso. Questi conteggi vengono cancellati a fine giornata.",
        ],
      },
      {
        h: "Questo sito",
        p: [
          "Niente cookie, niente statistiche, niente pixel di tracciamento, e nessun font o script di terze parti: tutto viene caricato da questo sito.",
          "Se apri uno scontrino protetto da password, il browser conserva un permesso di 24 ore nella sua memoria locale, così non devi riscrivere la password. Se scegli un tema (chiaro, scuro o OLED), ricorda anche quello. Nient'altro viene salvato nel browser.",
        ],
      },
      {
        h: "Chi lo gestisce",
        p: [
          "Fettle è un progetto open source. Il sito e i link condivisi sono ospitati su Cloudflare, che gestisce le richieste per nostro conto.",
        ],
        contact: true,
      },
    ],
    contactLine: "Domande sulla privacy, o su altro? Scrivi a",
    noContact: "I contatti verranno aggiunti qui prima della prima versione.",
  },
  changelog: {
    title: "Novità — Fettle",
    description: "Tutte le versioni di Fettle, dalla più recente.",
    kicker: "Novità",
    h1: "Cosa c'è di nuovo.",
    empty: "Nessuna versione per ora. La prima è in arrivo; ogni versione verrà elencata qui, dalla più recente.",
  },
  download: {
    title: "Scarica — Fettle",
    description: "Scarica Fettle, il controllo gratuito per Windows 10 e 11: un installer (circa 12 MB) o uno zip portatile (circa 14 MB), senza diritti di amministratore.",
    kicker: "Scarica",
    h1: "Fettle per Windows.",
    intro: "Fettle non è ancora disponibile. La prima versione è in costruzione e in prova su PC veri; quando sarà pronta, la trovi proprio qui.",
    button: "In arrivo",
    introReady: "Fettle {version} è pronto. Scegli l'installer, o lo zip portatile se preferisci non installare nulla.",
    installer: "Scarica l'installer",
    portable: "Zip portatile",
    checksums: "Checksum SHA-256",
    allReleases: "Tutte le versioni su GitHub",
    needsH: "Cosa serve",
    needs: ["Windows 10 o 11, 64 bit", "Circa 35 MB di spazio su disco", "Nessun diritto di amministratore (il Controllo approfondito lo chiede solo se vuoi)"],
    formatsH: "Due modi per avviarlo",
    formats: ["Installer, circa 12 MB, con un controllo degli aggiornamenti che puoi disattivare. Per rimuoverlo: Impostazioni di Windows > App, poi Fettle > Disinstalla.", "Zip portatile, circa 14 MB: niente da installare, tiene le impostazioni nella sua cartella e ha lo stesso controllo degli aggiornamenti che puoi disattivare. Per rimuoverlo, elimina la sua cartella."],
    warningH: "Sull'avviso di Windows",
    warning: [
      "Le app nuove senza certificato di firma del codice ricevono un avviso di SmartScreen al primo avvio, e i download di Fettle non sono ancora firmati, quindi potresti vedere \"Windows ha protetto il PC\".",
      "Scegli \"Ulteriori informazioni\", poi \"Esegui comunque\". Il codice sorgente è pubblico e ogni versione viene compilata alla luce del sole, così puoi verificare cosa stai avviando.",
      "Prima ancora, il browser potrebbe avvisarti che il file \"non viene scaricato comunemente\": nell'elenco dei download apri il menu del file (…), scegli \"Mantieni\", poi \"Mostra altro\" e \"Mantieni comunque\".",
    ],
    signingLink: "Policy di firma del codice",
  },
  codeSigning: {
    title: "Policy di firma del codice — Fettle",
    description: "Come vengono compilati, verificati e firmati i download di Fettle, e chi approva una versione.",
    kicker: "Policy di firma del codice",
    h1: "Come nascono e si firmano le versioni.",
    intro: "Ogni download di Fettle viene compilato dal codice pubblico da un servizio di build pubblico, mai sul PC di qualcuno. Ecco come, e chi può approvare una versione.",
    unsigned: {
      h: "Firma",
      p: [
        "Le versioni di Fettle non sono ancora firmate: un certificato di firma costa, e quelli gratuiti per i progetti open source vanno a progetti già molto conosciuti. Fettle è nuovo. Finché non sarà firmato, al primo avvio Windows potrebbe dire che l'editore è sconosciuto.",
        "Ogni versione elenca il checksum SHA-256 di ogni download, così puoi verificare che il file che hai sia proprio quello compilato.",
      ],
    },
    signed: {
      h: "Firma",
      p: [
        "Firma del codice gratuita offerta da SignPath.io, certificato di SignPath Foundation.",
        "Ogni versione elenca anche il checksum SHA-256 di ogni download, così puoi verificare che il file che hai sia proprio quello compilato.",
      ],
    },
    sections: [
      {
        h: "Come nasce una versione",
        p: [
          "Una versione parte quando il maintainer assegna un tag nel repository pubblico. GitHub Actions compila l'installer e lo zip portatile dal codice di quel tag, esegue tutti i test e registra un'attestazione di build: una nota firmata che dice da quale codice e da quale build viene ogni file.",
          "Nessuno compila una versione sul proprio PC, e una versione non è pubblica finché il maintainer non l'ha controllata e pubblicata.",
        ],
      },
      {
        h: "Chi fa cosa",
        p: [
          "Autore e revisore: candygoogy, che cura Fettle. Le modifiche di chiunque altro vengono riviste prima di essere unite.",
          "Approvazione: candygoogy approva ogni versione prima che venga pubblicata.",
        ],
      },
      {
        h: "Privacy",
        p: [
          "Fettle non invia nulla da solo, tranne il controllo degli aggiornamenti: chiede a questo sito il numero dell'ultima versione, senza inviare alcuna informazione sul tuo PC, e puoi disattivarlo nelle Impostazioni. Lo scontrino viene caricato online solo quando chiedi un link di condivisione.",
        ],
      },
    ],
    privacyLink: "Leggi la pagina sulla privacy",
  },
  notFound: {
    title: "Pagina non trovata — Fettle",
    description: "Questa pagina non c'è.",
    h1: "Questa pagina non c'è.",
    p: "Il link potrebbe essere vecchio, o scritto male.",
    line: "Nessuna pagina a questo indirizzo.",
    next: "Cosa fare",
    links: [
      { href: "/it", label: "Controlla il tuo PC" },
      { href: "/it/download", label: "Scarica Fettle" },
      { href: "/it/faq", label: "Domande e risposte" },
      { href: "/it/changelog", label: "Novità" },
    ],
  },
};

export const pages = (locale: Locale): Pages => (locale === "it" ? it : en);
