// Checked on 2026-10-19 against Microsoft's pages:
// https://support.microsoft.com/windows/windows-11-system-requirements-86c11283-ea52-4782-9efd-7674389a7ba3
//   and https://www.microsoft.com/windows/windows-11-specifications (the requirements, PC Health Check, waiting
//   for Windows Update after changing settings, Home and Pro for personal use needing internet and a Microsoft
//   account at setup), and the /it-it/ page (Italian names)
// https://www.microsoft.com/windows/end-of-support (the free upgrade: Home or Pro to the same edition)
// https://support.microsoft.com/windows/how-to-use-the-pc-health-check-app-9c8abd9b-03ba-4e67-81ef-36f37caa7844
//   and /it-it/ ("Controllo integrità PC", "Controlla ora")
// https://support.microsoft.com/windows/enable-tpm-2-0-on-your-pc-1fd5a332-360d-4f46-a1e7-ae6b0c90645c and /it-it/
//   (tpm.msc, Specification Version, a TPM that's turned off, the firmware path, the TPM setting names)
// https://support.microsoft.com/windows/windows-11-and-secure-boot-a8ff1202-c0d9-42f5-940f-843abef64fad and /it-it/
//   (Secure Boot capable is enough, Legacy/CSM, the maker's support pages)
// https://support.microsoft.com/windows/device-protection-in-windows-security-afa11526-de57-b1c5-599f-3a4c6a61c5e2
//   and /it-it/ (Security processor details)
// https://support.microsoft.com/windows/installing-windows-11-on-devices-that-don-t-meet-minimum-system-requirements-0b2dc4a2-5933-4ad4-9c09-ef0a331518f1
//   (unsupported: no guaranteed updates, warranty, watermark)
// https://learn.microsoft.com/windows/deployment/mbr-to-gpt (a Legacy disk is converted before switching to UEFI)
// msinfo32 showing BIOS Mode and Secure Boot State without administrator rights was tried on a Windows 11 PC.
// What Fettle shows follows crates/fettle-verdicts/src/rules/windows.rs (readiness_lines) and cpu.rs, and
// crates/fettle-collect/src/sources/mod.rs (the TPM is asked only in a Deep check); the quoted lines are the
// receipt's own wording (crates/fettle-verdicts/copy/*.json, keys windows.cpu_*, windows.tpm_unknown, windows.no_tpm,
// check.needs_admin.action). A TPM turned off in the firmware usually answers the Deep check's query with no rows,
// which crates/fettle-collect/src/map.rs reads as no TPM.
import type { Guide } from "../guides.ts";

const guide: Guide = {
  slug: "windows-11-ready",
  en: {
    title: "Is my PC ready for Windows 11? — Fettle",
    description: "What Windows 11 needs (a processor on Microsoft's list, TPM 2.0, Secure Boot), how to check your PC with Microsoft's own tools, and what to do if it isn't ready.",
    summary: "What Windows 11 needs, how to check your PC with Microsoft's own tools, and what to do if it isn't ready.",
    kicker: "Guide",
    h1: "Is my PC ready for Windows 11?",
    intro: "Windows 11 asks more of a PC than Windows 10: a processor on Microsoft's list, a TPM 2.0 security chip and Secure Boot. Microsoft's PC Health Check app tells you whether your PC has them, and when it falls short, the cause is sometimes just a setting.",
    sections: [
      {
        h: "What Windows 11 needs",
        p: [
          "A 64-bit processor on Microsoft's approved list, with 2 or more cores at 1 GHz or faster. Older PCs miss this one most often: many processors from before 2018 aren't on the list, even fast ones.",
          "A TPM 2.0, a small security chip that keeps encryption keys safe, and UEFI firmware (the modern BIOS) that supports Secure Boot, which stops untrusted software from starting before Windows. Secure Boot has to be supported, not necessarily turned on.",
          "At least 4 GB of memory, 64 GB of storage and DirectX 12 graphics. Windows 11 Home and Windows 11 Pro for personal use also need an internet connection and a Microsoft account during setup. For a Windows 10 Home or Pro PC that meets the requirements, the upgrade is free.",
        ],
      },
      {
        h: "Check it with Microsoft's own tools",
        p: ["PC Health Check gives the quickest answer; the other tools show one requirement each."],
        steps: [
          "Search for PC Health Check on the taskbar and open it (if it's missing, download it from Microsoft's website). Choose \"Check now\": it says whether the PC can run Windows 11 and, if not, why.",
          "Open Settings > Update & Security > Windows Update. When the PC meets the requirements and Microsoft is ready, the upgrade to Windows 11 is offered here; the timing varies from PC to PC.",
          "For the TPM, press the Windows key and R together, type tpm.msc and press Enter (Windows may ask for administrator permission). Specification Version should start with 2.0. A message that a compatible TPM cannot be found can mean it's there but turned off. Without administrator rights, Windows Security > Device security > Security processor details shows the same chip.",
          "For Secure Boot, press the Windows key and R, type msinfo32 and press Enter; this one needs no administrator rights. BIOS Mode should say UEFI, and Secure Boot State says whether it's on.",
        ],
      },
      {
        h: "If it's only a setting",
        p: [
          "A TPM that can't be found may just be turned off in the firmware settings. If you're unsure about a setting, check the PC maker's support pages first. If BIOS Mode says Legacy, moving to UEFI also means converting the disk, or Windows won't start: leave that to the maker's instructions or a technician, after a backup.",
        ],
        steps: [
          "Open Settings > Update & Security > Recovery and, under Advanced startup, choose Restart now.",
          "Choose Troubleshoot > Advanced options > UEFI Firmware Settings, then Restart.",
          "Turn on the TPM option. Makers name it differently: Intel PTT, AMD fTPM, Security Device Support or TPM State.",
          "Save and restart, then wait until Windows Update says Windows 11 is ready for the PC before upgrading.",
        ],
      },
      {
        h: "If the PC can't run Windows 11",
        p: [
          "If the processor isn't on the list or there's no TPM 2.0, no setting fixes it. The PC keeps working on Windows 10, and Extended Security Updates (ESU) keep home PCs on version 22H2 getting security updates until 12 October 2027, so there's no need to rush into a new PC.",
          "Microsoft documents a way to install Windows 11 on these PCs too, but it's unsupported and Microsoft doesn't recommend it: updates, security updates included, aren't guaranteed, damage from compatibility problems isn't covered by the warranty, and the desktop shows a watermark.",
        ],
        link: { href: "/guides/windows-10-updates", label: "See also: Is my Windows 10 PC still getting security updates?" },
      },
      {
        h: "How Fettle shows it",
        p: [
          "On Windows 10, the Windows section of Fettle's receipt adds notes on the processor, the TPM and Secure Boot. On Windows 11, it mentions Secure Boot only when it's off, and the processor only when it isn't on Microsoft's list: \"This processor isn't on Microsoft's list for Windows 11. Windows runs, but on PCs like this Microsoft doesn't promise to keep sending updates.\" Fettle doesn't change any settings.",
          "It compares the processor's name with Microsoft's lists (roughly, Intel Core from the 8th generation on and AMD Ryzen from the 2000 series on) and says \"This processor is on Microsoft's list for Windows 11.\" or that it isn't. When the name doesn't settle it, as with Pentium, Celeron or Xeon, it says \"Fettle couldn't tell whether this processor is on Microsoft's list for Windows 11.\"",
          "Windows only gives an app the TPM's details when the app runs as administrator, so the Quick check doesn't read the TPM: it says \"Fettle couldn't check the TPM security chip.\" and suggests \"Run a Deep check to see it.\" The optional Deep check then shows whether there's a TPM 2.0 or an older one. When it finds none, it says \"No TPM security chip found.\" and adds that most recent processors have one built in that can be turned on in the BIOS (fTPM or PTT): that's often the case with a TPM turned off in the firmware. If a TPM is there but reports no version, Fettle says it couldn't check it.",
          "Both checks read whether Secure Boot is on; if Windows doesn't report it, the line is left out. Fettle doesn't check memory, storage or graphics against the minimums: PC Health Check does.",
        ],
      },
    ],
  },
  it: {
    title: "Il mio PC è pronto per Windows 11? — Fettle",
    description: "Cosa chiede Windows 11 (processore nell'elenco di Microsoft, TPM 2.0, Secure Boot), come controllare il PC con gli strumenti Microsoft e cosa fare se non è pronto.",
    summary: "Cosa chiede Windows 11, come controllare il PC con gli strumenti di Microsoft e cosa fare se non è pronto.",
    kicker: "Guida",
    h1: "Il mio PC è pronto per Windows 11?",
    intro: "Windows 11 chiede a un PC più di Windows 10: un processore nell'elenco di Microsoft, un chip di sicurezza TPM 2.0 e il Secure Boot. L'app Controllo integrità PC di Microsoft ti dice se il tuo PC li ha e, quando non ci arriva, a volte è solo questione di un'impostazione.",
    sections: [
      {
        h: "Cosa chiede Windows 11",
        p: [
          "Un processore a 64 bit nell'elenco approvato da Microsoft, con almeno 2 core da 1 GHz o più. È il requisito che manca più spesso ai PC meno recenti: molti processori di prima del 2018 non ci sono, anche se sono veloci.",
          "Un TPM 2.0, un piccolo chip di sicurezza che custodisce le chiavi di crittografia, e un firmware UEFI (il BIOS moderno) che supporti il Secure Boot, in Windows chiamato Avvio protetto, che impedisce a software non attendibile di partire prima di Windows. Basta che il Secure Boot sia supportato, non per forza attivo.",
          "Almeno 4 GB di memoria, 64 GB di spazio di archiviazione e una grafica DirectX 12. Windows 11 Home e Windows 11 Pro per uso personale richiedono anche una connessione a Internet e un account Microsoft durante la configurazione. Per un PC con Windows 10 Home o Pro che ha i requisiti, l'aggiornamento è gratuito.",
        ],
      },
      {
        h: "Controlla con gli strumenti di Microsoft",
        p: ["Controllo integrità PC dà la risposta più rapida; gli altri strumenti mostrano un requisito alla volta."],
        steps: [
          "Cerca Controllo integrità PC dalla barra delle applicazioni e apri l'app (se manca, scaricala dal sito di Microsoft). Scegli \"Controlla ora\": ti dice se il PC può eseguire Windows 11 e, se no, perché.",
          "Apri Impostazioni > Aggiornamento e sicurezza > Windows Update. Quando il PC ha i requisiti e Microsoft è pronta, qui compare l'aggiornamento a Windows 11; i tempi cambiano da PC a PC.",
          "Per il TPM, premi insieme il tasto Windows e R, scrivi tpm.msc e premi Invio (Windows potrebbe chiedere i permessi di amministratore). Versione specifica deve iniziare con 2.0. Se un messaggio dice che non è possibile trovare un TPM compatibile, il TPM potrebbe esserci ma essere disattivato. Senza permessi di amministratore, lo stesso chip compare in Sicurezza di Windows > Sicurezza dispositivi > Dettagli processore sicurezza.",
          "Per il Secure Boot, premi il tasto Windows e R, scrivi msinfo32 e premi Invio: qui non servono permessi di amministratore. Modalità BIOS deve indicare UEFI, e Stato avvio protetto dice se il Secure Boot è attivo.",
        ],
      },
      {
        h: "Se è solo un'impostazione",
        p: [
          "Un TPM che non si trova potrebbe essere solo disattivato nelle impostazioni del firmware. Se hai dubbi su un'impostazione, guarda prima le pagine di assistenza del produttore del PC. Se Modalità BIOS indica Legacy, passare a UEFI vuol dire anche convertire il disco, altrimenti Windows non si avvia: segui le istruzioni del produttore o rivolgiti a un tecnico, dopo aver fatto un backup.",
        ],
        steps: [
          "Apri Impostazioni > Aggiornamento e sicurezza > Ripristino e, in Avvio avanzato, scegli Riavvia ora.",
          "Scegli Risoluzione dei problemi > Opzioni avanzate > Impostazioni firmware UEFI, poi Riavvia.",
          "Attiva l'opzione del TPM. Ogni produttore la chiama a modo suo: Intel PTT, AMD fTPM, Security Device Support o TPM State.",
          "Salva e riavvia, poi aspetta che Windows Update ti dica che Windows 11 è pronto per il PC, prima di aggiornare.",
        ],
      },
      {
        h: "Se il PC non può passare a Windows 11",
        p: [
          "Se il processore non è nell'elenco o manca il TPM 2.0, nessuna impostazione può rimediare. Il PC continua a funzionare con Windows 10 e, con gli Extended Security Updates (ESU), i PC di casa con la versione 22H2 ricevono aggiornamenti di sicurezza fino al 12 ottobre 2027: non serve correre a comprare un PC nuovo.",
          "Microsoft descrive un modo per installare Windows 11 anche su questi PC, ma non è supportato e Microsoft lo sconsiglia: gli aggiornamenti, compresi quelli di sicurezza, non sono garantiti, i danni dovuti a problemi di compatibilità non sono coperti dalla garanzia e sul desktop compare una filigrana.",
        ],
        link: { href: "/guides/windows-10-updates", label: "Leggi anche: Il mio PC con Windows 10 riceve ancora gli aggiornamenti di sicurezza?" },
      },
      {
        h: "Come lo mostra Fettle",
        p: [
          "Su Windows 10, la sezione Windows dello scontrino di Fettle aggiunge note sul processore, sul TPM e sul Secure Boot. Su Windows 11 parla del Secure Boot solo quando è disattivato, e del processore solo quando non è nell'elenco di Microsoft: \"Questo processore non è nell'elenco di Microsoft per Windows 11. Windows funziona, ma sui PC come questo Microsoft non garantisce di continuare a inviare gli aggiornamenti.\" Fettle non cambia nessuna impostazione.",
          "Confronta il nome del processore con gli elenchi di Microsoft (in breve, Intel Core dall'ottava generazione in poi e AMD Ryzen dalla serie 2000 in poi) e dice \"Questo processore è nell'elenco di Microsoft per Windows 11.\" oppure che non c'è. Quando il nome non basta, come con i Pentium, i Celeron o gli Xeon, dice \"Fettle non è riuscito a capire se questo processore è nell'elenco di Microsoft per Windows 11.\"",
          "Windows dà i dettagli del TPM a un'app solo se l'app viene eseguita come amministratore, quindi il Controllo rapido non legge il TPM: dice \"Fettle non è riuscito a controllare il chip di sicurezza TPM.\" e consiglia \"Avvia un Controllo approfondito per vederlo.\" Il Controllo approfondito, facoltativo, mostra poi se c'è un TPM 2.0 o uno più vecchio. Se non ne trova nessuno, dice \"Nessun chip di sicurezza TPM trovato.\" e aggiunge che la maggior parte dei processori recenti ne ha uno integrato da attivare nel BIOS (fTPM o PTT): spesso è proprio il caso di un TPM disattivato nel firmware. Se invece un TPM c'è ma non riporta la versione, Fettle dice che non è riuscito a controllarlo.",
          "Entrambi i controlli leggono se il Secure Boot è attivo; se Windows non lo riporta, la riga non compare. Fettle non confronta memoria, spazio di archiviazione e grafica con i minimi: lo fa Controllo integrità PC.",
        ],
      },
    ],
  },
};

export default guide;
