// Checked on 2026-10-19 against Microsoft's pages:
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro (end of support, 22H2)
// https://www.microsoft.com/windows/extended-security-updates and /it-it/ (consumer ESU: end date, editions,
//   enrolment steps and options, EEA terms), https://www.microsoft.com/windows/end-of-support (free upgrade)
// https://learn.microsoft.com/en-us/windows/whats-new/extended-security-updates (commercial ESU, three years)
// What Fettle shows follows crates/fettle-verdicts/src/rules/windows.rs and lifecycle.rs; the quoted
// lines are the receipt's own wording (crates/fettle-verdicts/copy/*.json, keys windows.*).
import type { Guide } from "../guides.ts";

const guide: Guide = {
  slug: "windows-10-updates",
  en: {
    title: "Is my Windows 10 PC still getting security updates? — Fettle",
    description: "Windows 10 support ended on 14 October 2025. How to check whether your PC still gets security updates through Extended Security Updates, and what to do if it doesn't.",
    summary: "When Windows 10 support ended, what Extended Security Updates are, and how to check your PC.",
    kicker: "Guide",
    h1: "Is my Windows 10 PC still getting security updates?",
    intro: "Microsoft's support for Windows 10 ended in October 2025, but many Windows 10 PCs can still get security updates for a while longer. Here's how to tell whether yours does, using Windows' own tools, and what to do if it doesn't.",
    sections: [
      {
        h: "When support ended",
        p: [
          "Microsoft ended support for Windows 10 on 14 October 2025. The PC keeps working, but since then Windows Update brings it no security fixes unless it's enrolled in Extended Security Updates.",
          "Security updates fix the weak spots that malware can use; an antivirus doesn't replace them.",
          "On home PCs, older versions of Windows 10 (21H2 and earlier) lost support even sooner. Only version 22H2, the last one, can still get updates.",
        ],
      },
      {
        h: "What Extended Security Updates are",
        p: [
          "Extended Security Updates (ESU) is how Microsoft keeps sending security updates to Windows 10 after the end of support. It brings security fixes only: no new features, and no technical support.",
          "For home PCs (Windows 10 Home, Pro, Pro Education and Pro for Workstations, version 22H2) it runs until 12 October 2027. Microsoft first offered one year, to October 2026, then extended it: a PC that was already enrolled carries on to the new date without you doing anything.",
          "In the European Economic Area, enrolling is free if you sign in to the PC with a Microsoft account; staying on a local account costs a one-off $30 (or the local equivalent, plus tax). Elsewhere it's free if you sync your PC settings with a Microsoft account, or it costs 1,000 Microsoft Rewards points or the same $30.",
          "Work and school PCs (Enterprise and Education) get ESU through their organisation, for up to three years after the end of support.",
        ],
      },
      {
        h: "Check it with Windows' own tools",
        p: ["Two windows tell you what you need."],
        steps: [
          "Press the Windows key and R together, type winver and press Enter. The window shows the version, which needs to be 22H2, and the OS build: a number like 19045 followed by a dot and more digits.",
          "Open Settings > Update & Security > Windows Update. If the PC is enrolled in ESU, this page says so. If it isn't but can be, there's an \"Enroll now\" link.",
          "On the same page, choose \"Check for updates\" and install what it offers. An enrolled PC is only protected once the updates are installed.",
        ],
      },
      {
        h: "What to do",
        p: [
          "If the PC can run Windows 11, moving to it is the longer-lasting answer. The upgrade from Windows 10 Home or Pro is free, and when the PC meets the requirements, Windows Update offers it on the same page.",
          "If it can't, or you're not ready yet, enrol in ESU while it's offered: choose \"Enroll now\" in Windows Update and follow the steps. Install the latest updates first, because the link only appears once the PC meets the requirements. You need a Microsoft account that's an administrator on the PC; if you use a local account, Windows asks you to sign in with one.",
          "If winver shows a version older than 22H2, the PC can't enrol until it's been updated to 22H2.",
          "Enrolling in ESU doesn't stop you from moving to Windows 11 later. ESU ends on 12 October 2027 either way, so use the time to plan the move.",
        ],
      },
      {
        h: "How Fettle shows it",
        p: [
          "On Windows 10, the first line of the Windows section on Fettle's receipt says whether the PC gets security updates, until when, and what to do next. Fettle only reads: it doesn't enrol the PC, install updates or change anything.",
          "Fettle can't read the ESU enrolment itself, so it reads the build instead: since November 2025, every Windows 10 update past build 19045.6466 has come only through ESU. When the build is newer than that, the PC is getting ESU updates, and the line says (with your edition's name): \"Windows 10 Home gets security updates until 12 October 2027 through Extended Security Updates.\"",
          "If the PC is still on build 19045.6466 or older, the line says \"Windows 10 Home has missed the security updates released since November 2025.\" and suggests: \"Enrol in Extended Security Updates from Windows Update and install what it offers, or upgrade to Windows 11.\" An enrolled PC that hasn't installed the updates yet shows the same; Windows Update tells you which it is.",
          "If Fettle can't read the build, it doesn't guess: it says the PC \"only gets security updates if it's enrolled in Extended Security Updates\" and asks you to check Windows Update. On a version older than 22H2 it says the PC \"no longer gets security updates\". On a work or school PC it suggests asking whoever manages it.",
          "On Windows 10, Fettle also lists what Windows 11 needs: whether the processor is on Microsoft's list, the TPM 2.0 security chip and Secure Boot.",
        ],
      },
    ],
  },
  it: {
    title: "Windows 10 riceve ancora gli aggiornamenti di sicurezza? — Fettle",
    description: "Il supporto di Windows 10 è terminato il 14 ottobre 2025. Come verificare se il tuo PC riceve ancora gli aggiornamenti di sicurezza con gli Extended Security Updates, e cosa fare se non li riceve.",
    summary: "Quando è finito il supporto di Windows 10, cosa sono gli Extended Security Updates e come controllare il tuo PC.",
    kicker: "Guida",
    h1: "Il mio PC con Windows 10 riceve ancora gli aggiornamenti di sicurezza?",
    intro: "Il supporto di Windows 10 è terminato a ottobre 2025, ma molti PC con Windows 10 possono ricevere gli aggiornamenti di sicurezza ancora per un po'. Ecco come capire se il tuo li riceve, con gli strumenti di Windows, e cosa fare se non li riceve.",
    sections: [
      {
        h: "Quando è finito il supporto",
        p: [
          "Microsoft ha terminato il supporto di Windows 10 il 14 ottobre 2025. Il PC continua a funzionare, ma da allora Windows Update non porta più correzioni di sicurezza, a meno che il PC non sia iscritto agli Extended Security Updates.",
          "Gli aggiornamenti di sicurezza correggono i punti deboli che i malware possono sfruttare; un antivirus non li sostituisce.",
          "Sui PC di casa, le versioni precedenti di Windows 10 (la 21H2 e quelle prima) hanno perso il supporto ancora prima. Solo la 22H2, l'ultima, può ancora ricevere aggiornamenti.",
        ],
      },
      {
        h: "Cosa sono gli Extended Security Updates",
        p: [
          "Gli Extended Security Updates (ESU, sul sito di Microsoft \"Aggiornamenti di sicurezza estesa\") sono il programma con cui Microsoft continua a inviare aggiornamenti di sicurezza a Windows 10 dopo la fine del supporto. Portano solo correzioni di sicurezza: niente nuove funzionalità e niente assistenza tecnica.",
          "Per i PC di casa (Windows 10 Home, Pro, Pro Education e Pro for Workstations, versione 22H2) durano fino al 12 ottobre 2027. All'inizio Microsoft li aveva offerti per un anno, fino a ottobre 2026, poi li ha prolungati: un PC già iscritto arriva alla nuova data senza che tu debba fare nulla.",
          "Nello Spazio economico europeo, Italia compresa, l'iscrizione è gratuita se accedi al PC con un account Microsoft; se vuoi restare con un account locale costa 30 dollari una tantum (o l'equivalente in euro, più le tasse). Fuori dallo Spazio economico europeo è gratuita se sincronizzi le impostazioni del PC con un account Microsoft, altrimenti costa 1.000 punti Microsoft Rewards o gli stessi 30 dollari.",
          "I PC di lavoro e di scuola (Enterprise ed Education) ricevono gli ESU tramite l'organizzazione, per al massimo tre anni dopo la fine del supporto.",
        ],
      },
      {
        h: "Controlla con gli strumenti di Windows",
        p: ["Bastano due finestre di Windows."],
        steps: [
          "Premi insieme il tasto Windows e R, scrivi winver e premi Invio. La finestra mostra la versione, che deve essere 22H2, e la build del sistema operativo: un numero come 19045 seguito da un punto e altre cifre.",
          "Apri Impostazioni > Aggiornamento e sicurezza > Windows Update. Se il PC è iscritto agli ESU, la pagina lo indica. Se non lo è ma può esserlo, compare il link \"Iscriviti ora\".",
          "Nella stessa pagina scegli \"Verifica disponibilità aggiornamenti\" e installa ciò che propone. Anche un PC iscritto è protetto solo dopo aver installato gli aggiornamenti.",
        ],
      },
      {
        h: "Cosa fare",
        p: [
          "Se il PC può eseguire Windows 11, passare a Windows 11 è la soluzione più duratura. L'aggiornamento da Windows 10 Home o Pro è gratuito e, se il PC soddisfa i requisiti, Windows Update lo propone nella stessa pagina.",
          "Se non può, o non sei ancora pronto, iscriviti agli ESU finché sono disponibili: scegli \"Iscriviti ora\" in Windows Update e segui i passaggi. Prima installa gli ultimi aggiornamenti, perché il link compare solo quando il PC soddisfa i requisiti. Serve un account Microsoft con diritti di amministratore sul PC; se usi un account locale, Windows ti chiede di accedere con un account Microsoft.",
          "Se winver mostra una versione precedente alla 22H2, il PC non può iscriversi finché non viene aggiornato alla 22H2.",
          "L'iscrizione agli ESU non ti impedisce di passare a Windows 11 più avanti. Gli ESU finiscono comunque il 12 ottobre 2027: usa questo tempo per organizzare il passaggio.",
        ],
      },
      {
        h: "Come lo mostra Fettle",
        p: [
          "Su Windows 10, la prima riga della sezione Windows dello scontrino di Fettle dice se il PC riceve aggiornamenti di sicurezza, fino a quando e cosa fare. Fettle legge e basta: non iscrive il PC, non installa aggiornamenti e non cambia nulla.",
          "Fettle non può leggere direttamente l'iscrizione agli ESU, quindi legge la build: da novembre 2025 ogni aggiornamento di Windows 10 oltre la build 19045.6466 arriva solo tramite gli ESU. Se la build è più recente, il PC sta ricevendo gli ESU e la riga dice (con il nome della tua edizione): \"Windows 10 Home riceve aggiornamenti di sicurezza grazie agli Extended Security Updates. Fine del supporto: 12 ottobre 2027.\"",
          "Se il PC è ancora alla build 19045.6466 o precedente, la riga dice \"Windows 10 Home non ha ricevuto gli aggiornamenti di sicurezza usciti da novembre 2025.\" e consiglia: \"Iscriviti agli Extended Security Updates da Windows Update e installa ciò che propone, oppure passa a Windows 11.\" Lo stesso vale per un PC già iscritto che non ha ancora installato gli aggiornamenti: Windows Update ti dice qual è il caso.",
          "Se non riesce a leggere la build, Fettle non tira a indovinare: dice che il PC \"riceve aggiornamenti di sicurezza solo se è iscritto agli Extended Security Updates\" e ti chiede di controllare in Windows Update. Su una versione precedente alla 22H2 dice che il PC \"non riceve più aggiornamenti di sicurezza\". Su un PC di lavoro o di scuola consiglia di chiedere a chi lo gestisce.",
          "Sempre su Windows 10, Fettle elenca anche ciò che serve per Windows 11: se il processore è nell'elenco di Microsoft, il chip di sicurezza TPM 2.0 e il Secure Boot.",
        ],
      },
    ],
  },
};

export default guide;
