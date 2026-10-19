// Checked on 2026-10-19 against Microsoft's pages:
// https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/what-to-do-about-a-critical-warning-for-a-storage-device
//   and /it-it/ (Windows 11 and Windows 10 paths, the three warnings, the three health values, NVMe only)
// https://blogs.windows.com/windows-insider/2020/09/30/announcing-windows-10-insider-preview-build-20226/ (NVMe health
//   monitoring and its notification)
// https://support.microsoft.com/it-it/windows/experience/storage-filemanagement/storage-settings-in-windows (Italian
//   names: Impostazioni di archiviazione avanzate, Dischi e volumi, Proprietà)
// https://support.microsoft.com/en-us/windows/experience/backup-recovery/back-up-and-restore-with-windows-backup and
//   /it-it/ (Windows Backup: folders, OneDrive, personal Microsoft account, 5 GB free)
// What Fettle shows follows crates/fettle-verdicts/src/rules/storage.rs, thresholds.rs and summary.rs; the quoted
// lines are the receipt's own wording (crates/fettle-verdicts/copy/*.json, keys storage.* and summary.*).
import type { Guide } from "../guides.ts";

const guide: Guide = {
  slug: "disk-health",
  en: {
    title: "Is my SSD or hard drive wearing out? — Fettle",
    description: "How to tell whether an SSD or hard drive is wearing out, check its health with Windows' own tools, understand the numbers, and know when to replace it.",
    summary: "The signs of a worn drive, how to check its health in Windows, and when to replace it.",
    kicker: "Guide",
    h1: "Is my SSD or hard drive wearing out?",
    intro: "Drives wear out over the years, and many keep a record of their own health. On most recent PCs, Windows shows the main figures from it in Settings.",
    sections: [
      {
        h: "What a worn drive looks like",
        p: [
          "A hard drive that's wearing out often gives signs: new clicking or grinding noises, files that won't open, copies that stop halfway, or the PC freezing whenever it reads from the disk.",
          "An SSD makes no noise. It may start showing read errors, or stop with little warning. Some SSDs in trouble switch to read-only: your files are still there, but nothing new can be saved.",
          "Slowness alone isn't proof. A PC is more often slow because the drive is nearly full, updates are installing or too many apps start with Windows. Check the drive's health before blaming it.",
        ],
      },
      {
        h: "Back up first",
        p: [
          "If a drive worries you, copy anything important to another drive before anything else. A second partition on the same disk isn't a backup.",
          "Windows Backup, on Windows 11 and Windows 10, copies your Desktop, Documents, Pictures, Videos and Music folders to OneDrive. It needs a personal Microsoft account, and OneDrive's free space is 5 GB; for more, use an external drive.",
        ],
      },
      {
        h: "Check it with Windows' own tools",
        p: [
          "Windows reads the health of NVMe SSDs, the kind most recent PCs use. It doesn't monitor SATA SSDs or hard drives. When an NVMe SSD reports a problem, Windows also shows a warning (spare capacity is low, reliability is degraded, or the drive is set to read-only), and Microsoft's advice is to back up your data.",
        ],
        steps: [
          "Open Settings > System > Storage > Advanced storage settings > Disks & volumes. On Windows 10, Microsoft gives the path Settings > System > Storage > Manage disks and volumes.",
          "Select the disk, then choose Properties.",
          "Look for Drive health: estimated remaining life, available spare and temperature. If it isn't there, Windows doesn't read that drive's health, usually because it's a SATA SSD or a hard drive.",
        ],
      },
      {
        h: "What the numbers mean",
        p: [
          "Estimated remaining life is how much of its rated life the SSD has left, by the manufacturer's estimate. At 0% the drive doesn't stop that day, but it's past what the maker planned for. Available spare is the reserve that replaces worn-out cells; Windows warns when it drops below the maker's threshold.",
          "Hard drives and SATA SSDs keep a different record, called SMART. The counts that matter are reallocated sectors (worn spots already replaced with spare ones), pending sectors (spots the drive is struggling to read) and uncorrectable sectors (spots it couldn't read or repair). NVMe SSDs count media errors: reads or writes that went wrong. Windows has no screen that shows these counts for hard drives and SATA SSDs; Fettle's Deep check reads them.",
          "A small number that stays the same is worth watching. One that keeps growing means the drive is getting worse.",
        ],
      },
      {
        h: "What to do",
        p: [
          "If the drive looks healthy, keep backing up as usual. If a hard drive makes new noises, shows pending or uncorrectable sectors, or Windows shows a warning, back up anything important now and plan to replace the drive soon.",
          "No program can repair the worn parts of a drive: replacing it is the fix, and a repair shop can copy everything onto the new one.",
          "A drive that's only hot needs better airflow; one that's nearly full needs space freed up (Settings > System > Storage shows what's using it).",
        ],
      },
      {
        h: "How Fettle shows it",
        p: [
          "The Storage section of Fettle's receipt has a line for each drive inside the PC, like \"Samsung 990 PRO looks healthy, 3% worn.\" Fettle counts the life used, where Windows counts the life left. Fettle only reads: it doesn't test, repair or change anything.",
          "Fettle reads an NVMe SSD's health without administrator rights. Windows shares a SATA drive's health only with an administrator, so there the Quick check says \"Run a Deep check to see it.\" The Deep check asks for administrator access, and you can say no. Some drives don't share their health with Windows at all, and USB drives aren't checked.",
          "From 80% worn, an SSD gets a WATCH line: \"It still works, but back up anything important and plan a replacement in the next year or so.\" At 100% worn, or when a drive reports a problem with itself or has pending or uncorrectable sectors, the line is ACT, and its Next step starts \"Back up anything important now\". (A single pending sector on an SSD gets WATCH.) Replaced sectors and read or write errors get a WATCH line, as does a drive above 70 °C (55 °C for a hard drive).",
          "Under 10% free, a drive is \"almost full\" (a drive other than C: only if less than about 50 GB is left). If everything else is fine but a drive's health couldn't be read, the receipt says \"Fine so far, but Fettle couldn't read the disks' health.\"",
        ],
      },
    ],
  },
  it: {
    title: "Il mio SSD o disco rigido si sta consumando? — Fettle",
    description: "Come capire se un SSD o un disco rigido si sta consumando, controllarne lo stato con gli strumenti di Windows, leggere i valori e sapere quando sostituirlo.",
    summary: "I segnali di un disco consumato, come controllarne lo stato in Windows e quando sostituirlo.",
    kicker: "Guida",
    h1: "Il mio SSD o disco rigido si sta consumando?",
    intro: "Con gli anni i dischi si consumano, e molti tengono traccia del proprio stato. Sulla maggior parte dei PC recenti, Windows ne mostra i valori principali nelle Impostazioni.",
    sections: [
      {
        h: "Come si riconosce un disco consumato",
        p: [
          "Un disco rigido che si sta consumando spesso dà dei segnali: ticchettii o rumori di sfregamento nuovi, file che non si aprono, copie che si fermano a metà, il PC che si blocca ogni volta che legge dal disco.",
          "Un SSD non fa rumore. Può iniziare a dare errori di lettura, oppure fermarsi quasi senza preavviso. Alcuni SSD in difficoltà passano in sola lettura: i file ci sono ancora, ma non si può salvare niente di nuovo.",
          "La lentezza da sola non prova nulla. Più spesso un PC è lento perché il disco è quasi pieno, sta installando aggiornamenti o troppe app partono con Windows. Prima di dare la colpa al disco, controllane lo stato.",
        ],
      },
      {
        h: "Prima di tutto, il backup",
        p: [
          "Se un disco ti preoccupa, la prima cosa da fare è copiare i file importanti su un altro disco. Una seconda partizione sullo stesso disco non è un backup.",
          "Windows Backup, su Windows 11 e Windows 10, copia su OneDrive le cartelle Desktop, Documenti, Immagini, Video e Musica. Serve un account Microsoft personale, e lo spazio gratuito di OneDrive è di 5 GB; per il resto usa un disco esterno.",
        ],
      },
      {
        h: "Controlla con gli strumenti di Windows",
        p: [
          "Windows legge lo stato degli SSD NVMe, il tipo che usa la maggior parte dei PC recenti, ma non controlla gli SSD SATA né i dischi rigidi. Quando un SSD NVMe segnala un problema, Windows mostra anche un avviso (la capacità di riserva è bassa, l'affidabilità è ridotta o l'unità è impostata su sola lettura), e Microsoft consiglia di fare un backup dei dati.",
        ],
        steps: [
          "Apri Impostazioni > Sistema > Archiviazione > Impostazioni di archiviazione avanzate > Dischi e volumi. Su Windows 10 il percorso indicato da Microsoft è Impostazioni > Sistema > Archiviazione > Gestisci dischi e volumi.",
          "Seleziona il disco e scegli Proprietà.",
          "Cerca Integrità unità: Vita residua stimata, Ricambio disponibile e Temperatura. Se non c'è, Windows non legge lo stato di quel disco, di solito perché è un SSD SATA o un disco rigido.",
        ],
      },
      {
        h: "Cosa significano i valori",
        p: [
          "La vita residua stimata è la parte di vita prevista che resta all'SSD, secondo la stima del produttore. Allo 0% il disco non si ferma quel giorno, ma ha superato la durata prevista. Il ricambio disponibile è la riserva che sostituisce le celle usurate; Windows avvisa quando scende sotto la soglia del produttore.",
          "I dischi rigidi e gli SSD SATA tengono un registro diverso, chiamato SMART. I valori da guardare sono i settori riallocati (punti usurati già sostituiti con quelli di riserva), i settori in sospeso (punti che il disco fatica a leggere) e i settori non correggibili (punti che non è riuscito né a leggere né a riparare). Gli SSD NVMe contano invece gli errori di lettura o scrittura. Per i dischi rigidi e gli SSD SATA, Windows non ha una schermata che mostri questi valori; li legge il Controllo approfondito di Fettle.",
          "Un numero piccolo che resta fermo va tenuto d'occhio. Uno che continua a crescere vuol dire che il disco sta peggiorando.",
        ],
      },
      {
        h: "Cosa fare",
        p: [
          "Se il disco sembra in salute, continua a fare i backup come sempre. Se un disco rigido fa rumori nuovi, ha settori in sospeso o non correggibili, oppure Windows mostra un avviso, fai subito un backup dei file importanti e pensa a sostituire presto il disco.",
          "Nessun programma può riparare le parti usurate di un disco: la soluzione è sostituirlo, e un tecnico può copiare tutto sul disco nuovo.",
          "Un disco che scalda e basta ha bisogno di più ricircolo d'aria; uno quasi pieno, di spazio libero (in Impostazioni > Sistema > Archiviazione vedi cosa lo occupa).",
        ],
      },
      {
        h: "Come lo mostra Fettle",
        p: [
          "Nella sezione Spazio di archiviazione dello scontrino, Fettle dedica una riga a ogni disco interno del PC, per esempio \"Samsung 990 PRO sembra in salute (usura 3%).\" Fettle conta la vita consumata, Windows quella che resta. Fettle legge e basta: non mette alla prova il disco, non lo ripara e non cambia nulla.",
          "Fettle legge lo stato di un SSD NVMe senza permessi di amministratore. Windows condivide lo stato di un disco SATA solo con un amministratore, quindi lì il Controllo rapido dice \"Avvia un Controllo approfondito per vederlo.\" Il Controllo approfondito chiede l'accesso da amministratore, e puoi rifiutare. Alcuni dischi non condividono affatto il loro stato con Windows, e le unità USB non vengono controllate.",
          "Da un'usura dell'80% in su, un SSD riceve una riga OCCHIO: \"Funziona ancora, ma fai un backup dei file importanti e pensa a sostituirlo entro un anno circa.\" Con un'usura del 100%, o se un disco segnala un problema o ha settori in sospeso o non correggibili, la riga è AGISCI, e sotto Cosa fare il consiglio inizia con \"Fai subito un backup dei file importanti\". (Un solo settore in sospeso su un SSD dà OCCHIO.) I settori sostituiti e gli errori di lettura o scrittura danno una riga OCCHIO, come un disco sopra 70 °C (55 °C per un disco rigido).",
          "Con meno di un decimo di spazio libero, un'unità risulta \"quasi piena\" (un'unità diversa da C: solo se le restano meno di circa 50 GB). Se tutto il resto è a posto ma lo stato di un disco non si è potuto leggere, lo scontrino dice \"Tutto a posto finora, ma Fettle non è riuscito a leggere lo stato dei dischi.\"",
        ],
      },
    ],
  },
};

export default guide;
