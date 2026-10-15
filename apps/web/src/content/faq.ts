// FAQ, shared by /faq and the landing page teaser (the first three), in English and Italian.
// Every answer must stay true for the released app.
import type { Locale } from "../i18n/paths.ts";

/** `contact`: the answer ends with the contact address, once there is one (src/release.ts). */
export type Faq = { id: string; q: string; a: string[]; contact?: true };

/** Printed in full on the landing page's questions slip; the More questions slip lists the rest. */
export const PRINTED_QUESTIONS: readonly string[] = ["admin", "changes", "mac"];

const en: Faq[] = [
  {
    id: "free",
    q: "Is it really free?",
    a: ["Yes. Fettle is free and open source under the MIT licence. There's no paid version, no ads and nothing to upgrade to."],
  },
  {
    id: "changes",
    q: "Does it change anything on my PC?",
    a: [
      "No. Fettle only reads. It doesn't clean, \"boost\", update drivers or touch the registry, and it doesn't install anything that runs in the background.",
      "When something needs doing, the receipt tells you what to do and you decide.",
    ],
  },
  {
    id: "admin",
    q: "Does it need administrator rights?",
    a: [
      "No. The normal check runs as a regular user.",
      "Disk health needs administrator access, so there's an optional Deep check that asks for it and explains why. Without it, the receipt says the disk health couldn't be checked, rather than guessing.",
    ],
  },
  {
    id: "warning",
    q: "Why does Windows warn me about the publisher?",
    a: [
      "New apps without a code-signing certificate get a SmartScreen warning, and Fettle's downloads aren't signed yet: free signing for open-source projects goes to projects that are already widely known, and Fettle is new.",
      "Until then the warning can appear: choose \"More info\", then \"Run anyway\". The source code is public and every release is built in the open, so you can check what you're running.",
      "Your browser may warn first that the file \"isn't commonly downloaded\": in its downloads list, open the file's menu (…), choose \"Keep\", then \"Show more\" and \"Keep anyway\".",
    ],
  },
  {
    id: "antivirus",
    q: "My antivirus flagged it. Is it safe?",
    a: [
      "Hardware tools sometimes trip antivirus heuristics because they read system information. Fettle uses no drivers, and its source and builds are public.",
      "If your antivirus flags it, please tell us which one, so we can report it as a false positive.",
    ],
    contact: true,
  },
  {
    id: "share",
    q: "What's in a share link, and who can see it?",
    a: [
      "The report behind the receipt you see in the app, and nothing else. Serial numbers, the PC's name, your name, username and email address, and network addresses are never in it. Anyone with the link can open it, so share it where you mean to.",
      "Links expire after 7 days, 30 days or 1 year, your choice. You can add a password, and you get a private link to delete it whenever you like. The app also keeps your links under My shared links, so you can delete any of them later.",
    ],
  },
  {
    id: "forum",
    q: "Can I post my receipt on a forum or Discord?",
    a: [
      "Yes. \"Copy for Discord or Reddit\" puts the receipt on your clipboard as text, ready to paste. Like everything Fettle makes, it leaves out serial numbers, the PC's name, your name, username and email address, and network addresses.",
      "If someone who knows PCs asks for the details, you can also save the full report as JSON and send them the file: press Save and choose \"Report for experts (JSON)\" as the file type.",
    ],
  },
  {
    id: "temps",
    q: "Why doesn't it show CPU temperature?",
    a: [
      "Reading CPU temperatures on Windows needs a kernel driver. Fettle v1 avoids drivers on purpose, so nothing it installs reaches into Windows itself. An optional, clearly explained add-on may come later.",
    ],
  },
  {
    id: "windows10",
    q: "Does it work on Windows 10?",
    a: [
      "Yes, on Windows 10 and Windows 11. On Windows 10 it also tells you whether the PC still gets security updates, including Extended Security Updates, and whether it's ready for Windows 11.",
    ],
  },
  {
    id: "mac",
    q: "Mac or Linux?",
    a: ["No. Fettle is a Windows app, so it can read Windows' own records properly."],
  },
  {
    id: "languages",
    q: "Which languages?",
    a: ["English and Italian. Fettle follows your Windows display language, and you can change it in the settings."],
  },
];

const it: Faq[] = [
  {
    id: "free",
    q: "È davvero gratis?",
    a: ["Sì. Fettle è gratuito e open source con licenza MIT. Non c'è una versione a pagamento, niente pubblicità e niente da sbloccare."],
  },
  {
    id: "changes",
    q: "Modifica qualcosa sul mio PC?",
    a: [
      "No. Fettle legge e basta. Non pulisce, non \"ottimizza\", non aggiorna driver e non tocca il registro, e non installa nulla che resti attivo in background.",
      "Quando c'è qualcosa da fare, lo scontrino ti dice cosa fare e decidi tu.",
    ],
  },
  {
    id: "admin",
    q: "Servono i diritti di amministratore?",
    a: [
      "No. Il controllo normale funziona come utente standard.",
      "La salute dei dischi richiede l'accesso da amministratore, quindi c'è un Controllo approfondito facoltativo che lo chiede e spiega perché. Senza, lo scontrino dice che la salute del disco non è stata verificata, invece di tirare a indovinare.",
    ],
  },
  {
    id: "warning",
    q: "Perché Windows mi avvisa sull'editore?",
    a: [
      "Le app nuove senza un certificato di firma del codice ricevono un avviso di SmartScreen, e i download di Fettle non sono ancora firmati: la firma gratuita per i progetti open source va a progetti già molto conosciuti, e Fettle è nuovo.",
      "Fino ad allora l'avviso può comparire: scegli \"Ulteriori informazioni\", poi \"Esegui comunque\". Il codice sorgente è pubblico e ogni versione viene compilata alla luce del sole, così puoi verificare cosa stai avviando.",
      "Prima ancora, il browser potrebbe avvisarti che il file \"non viene scaricato comunemente\": nell'elenco dei download apri il menu del file (…), scegli \"Mantieni\", poi \"Mostra altro\" e \"Mantieni comunque\".",
    ],
  },
  {
    id: "antivirus",
    q: "Il mio antivirus lo segnala. È sicuro?",
    a: [
      "Gli strumenti hardware a volte fanno scattare le euristiche degli antivirus perché leggono informazioni di sistema. Fettle non usa driver, e codice e build sono pubblici.",
      "Se il tuo antivirus lo segnala, dicci quale, così possiamo segnalarlo come falso positivo.",
    ],
    contact: true,
  },
  {
    id: "share",
    q: "Cosa c'è in un link condiviso, e chi può vederlo?",
    a: [
      "Il report da cui nasce lo scontrino che vedi nell'app, e nient'altro. Numeri di serie, nome del PC, il tuo nome, nome utente, indirizzo email e indirizzi di rete non ci sono mai. Chiunque abbia il link può aprirlo, quindi condividilo dove intendi tu.",
      "I link scadono dopo 7 giorni, 30 giorni o 1 anno, a tua scelta. Puoi aggiungere una password, e ricevi un link privato per eliminarlo quando vuoi. L'app conserva anche i tuoi link in I miei link condivisi, così puoi eliminarli in qualsiasi momento.",
    ],
  },
  {
    id: "forum",
    q: "Posso pubblicare lo scontrino su un forum o su Discord?",
    a: [
      "Sì. \"Copia per Discord o Reddit\" copia lo scontrino come testo negli appunti, pronto da incollare. Come tutto ciò che crea Fettle, non include numeri di serie, nome del PC, il tuo nome, nome utente, indirizzo email né indirizzi di rete.",
      "Se qualcuno esperto di PC ti chiede i dettagli, puoi anche salvare il report completo in JSON e mandargli il file: premi Salva e scegli \"Report per esperti (JSON)\" come tipo di file.",
    ],
  },
  {
    id: "temps",
    q: "Perché non mostra la temperatura della CPU?",
    a: [
      "Leggere la temperatura della CPU su Windows richiede un driver del kernel. Fettle v1 evita i driver di proposito, così nulla di ciò che installa entra nel cuore di Windows. Più avanti potrebbe arrivare un componente facoltativo, spiegato con chiarezza.",
    ],
  },
  {
    id: "windows10",
    q: "Funziona su Windows 10?",
    a: [
      "Sì, su Windows 10 e Windows 11. Su Windows 10 ti dice anche se il PC riceve ancora gli aggiornamenti di sicurezza, compresi gli Extended Security Updates, e se è pronto per Windows 11.",
    ],
  },
  {
    id: "mac",
    q: "Mac o Linux?",
    a: ["No. Fettle è un'app per Windows, così può leggere bene i dati che Windows conserva."],
  },
  {
    id: "languages",
    q: "In quali lingue?",
    a: ["Inglese e italiano. Fettle segue la lingua di visualizzazione di Windows, e puoi cambiarla nelle impostazioni."],
  },
];

export const faqs = (locale: Locale): Faq[] => (locale === "it" ? it : en);
