// Checked on 2026-10-19 against Microsoft's pages:
// https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/powercfg-command-line-options
//   (/batteryreport: a report of battery use over the PC's life, saved as HTML in the current folder)
// https://support.microsoft.com/en-us/surface/battery/caring-for-your-surface-battery and /it-it/ (lithium-ion
//   batteries are consumables that lose capacity with time and use; heat, long stretches on the charger,
//   storing at 50%, driver and firmware updates, a swollen battery)
// https://support.microsoft.com/en-us/surface/battery/smart-charging-on-surface and /it-it/ (80% limit)
// The report's labels (Installed batteries, Battery capacity history, Battery life estimates, the saved-to
// message) were read from a report made on an English Windows 11.
// What Fettle shows follows crates/fettle-verdicts/src/rules/others.rs (battery_rules) and thresholds.rs;
// the quoted lines are the receipt's own wording (crates/fettle-verdicts/copy/*.json, keys battery.*, and
// the detail labels in crates/fettle-receipt/copy/*.json).
import type { Guide } from "../guides.ts";

const guide: Guide = {
  slug: "battery-health",
  en: {
    title: "How healthy is my laptop battery? — Fettle",
    description: "How to see how much charge your laptop battery still holds with the battery report in Windows, what the numbers mean, and when a new battery makes sense.",
    summary: "What battery wear is, how to read Windows' battery report, and when a new battery makes sense.",
    kicker: "Guide",
    h1: "How healthy is my laptop battery?",
    intro: "Every laptop battery holds a little less charge as it ages. Windows keeps track of how much, and a short report shows it. Here's how to open it, which numbers to read, and what to do next.",
    sections: [
      {
        h: "What battery wear means",
        p: [
          "Windows knows two figures for a battery: the design capacity, what it was built to hold when new, and the full charge capacity, what it holds today when charged to 100%. Comparing the two tells you how worn it is.",
          "Laptop batteries are lithium-ion, and Microsoft calls them consumables: they lose capacity over time and with use. Every charge cycle wears them a little, and so does heat.",
          "Some wear is normal. There's no single number at which a battery is finished: what matters is whether it still lasts as long as you need.",
        ],
      },
      {
        h: "Signs of a worn battery",
        p: [
          "The usual sign is less runtime: for the same kind of work, the laptop needs its charger sooner than it used to.",
          "A busy app or a brighter screen can shorten the runtime too, so look at the numbers before blaming the battery.",
          "Rarely, an old battery swells. If it visibly pushes the case out of shape, Microsoft's advice is to stop using the laptop and have the battery replaced.",
        ],
      },
      {
        h: "Check it with Windows' own tools",
        p: ["Windows has a battery report built in. It works on Windows 10 and 11, doesn't need administrator access, and changes nothing: it only saves the report file."],
        steps: [
          "Press the Windows key and R together, type cmd and press Enter. A Command Prompt window opens.",
          "Type powercfg /batteryreport and press Enter. Windows saves the report as battery-report.html, usually in your user folder, and shows the full path.",
          "Type start battery-report.html and press Enter to open it in your browser.",
          "Under Installed batteries, read DESIGN CAPACITY, FULL CHARGE CAPACITY and CYCLE COUNT.",
        ],
      },
      {
        h: "Reading the numbers",
        p: [
          "Divide the full charge capacity by the design capacity. For example, 36,000 mWh out of 50,000 mWh means the battery holds 72% of what it held when new.",
          "The cycle count is roughly how many full charges the battery has been through. Not every battery reports it; a dash means it wasn't available.",
          "Further down, Battery capacity history shows how the full charge capacity has changed over the weeks, and Battery life estimates compares today's runtime with what it would give at its design capacity.",
        ],
      },
      {
        h: "What to do",
        p: [
          "If the battery still lasts as long as you need, there's nothing to do. On a laptop that mostly stays plugged in, a worn battery matters even less.",
          "When it no longer gets you through the day, a new battery brings the runtime back. On many laptops the battery sits inside the case, so the maker or a repair shop is the easiest route; ask for one made for your model.",
          "To slow the wear, Microsoft gives this advice for its Surface laptops, which use the same kind of battery: keep the laptop out of the heat (direct sun, a hot car), don't leave it on the charger for very long stretches, and charge it to about 50% before putting it away for a while. Keep drivers and firmware up to date too. Some laptops can also stop charging at 80% when they stay plugged in: on Surface that's Smart Charging, and some models have a \"Limit to 80%\" option in the Surface app.",
        ],
      },
      {
        h: "How Fettle shows it",
        p: [
          "On a laptop, Fettle's receipt has a Battery section. Fettle reads the same two figures, design capacity and full charge capacity, and turns them into one line. It only reads: it doesn't change any battery or power setting.",
          "At 80% or more, the line is marked OK and says, with your number: \"Your battery holds 87% of the capacity it had when new.\"",
          "Below 80% it's marked WATCH, and below 60% ACT. Both say \"Your battery holds 72% of the capacity it had when new, so expect less runtime than it used to give.\" and suggest: \"Replace the battery to get the runtime back.\" The percentage is rounded down and never shown above 100%, because a new battery can report a little more than its design capacity.",
          "Underneath are the numbers: Original capacity, Full charge and, when the battery reports them, Charge cycles. The cycle count is for reference; the verdict comes from capacity alone.",
          "On a desktop or all-in-one without a battery, there's no Battery section. If the battery doesn't report its original capacity, Fettle doesn't guess: it says \"The battery didn't report its original capacity.\"",
        ],
      },
    ],
  },
  it: {
    title: "La batteria del mio portatile è in salute? — Fettle",
    description: "Come vedere quanta carica tiene ancora la batteria del portatile con il rapporto batteria di Windows, cosa vogliono dire i numeri e quando conviene cambiarla.",
    summary: "Cos'è l'usura della batteria, come leggere il rapporto batteria di Windows e quando conviene cambiarla.",
    kicker: "Guida",
    h1: "La batteria del mio portatile è in salute?",
    intro: "Con gli anni, ogni batteria di portatile tiene un po' meno carica. Windows tiene il conto di quanta ne ha persa e te lo mostra in un breve rapporto. Ecco come aprirlo, quali numeri leggere e cosa fare dopo.",
    sections: [
      {
        h: "Cosa vuol dire usura della batteria",
        p: [
          "Windows conosce due valori della batteria: la capacità di progetto, cioè quanto doveva contenere da nuova, e la capacità a carica completa, cioè quanto contiene oggi quando è al 100%. Confrontandoli capisci quanto è usurata.",
          "Le batterie dei portatili sono agli ioni di litio e Microsoft le definisce materiali di consumo: perdono capacità con il tempo e con l'uso. Ogni ciclo di carica le consuma un po', e il calore anche.",
          "Un po' di usura è normale. Non c'è una soglia oltre la quale la batteria è da buttare: conta se dura ancora quanto ti serve.",
        ],
      },
      {
        h: "Come si riconosce una batteria usurata",
        p: [
          "Il segnale più comune è meno autonomia: a parità di lavoro, il portatile chiede il caricabatterie prima di una volta.",
          "Anche un'app che lavora molto o uno schermo più luminoso accorciano l'autonomia, quindi guarda i numeri prima di dare la colpa alla batteria.",
          "Di rado una batteria vecchia si gonfia. Se deforma visibilmente la scocca, Microsoft consiglia di smettere di usare il portatile e di far sostituire la batteria.",
        ],
      },
      {
        h: "Controlla con gli strumenti di Windows",
        p: ["Windows ha già un rapporto sulla batteria. Funziona su Windows 10 e 11, non servono i permessi di amministratore e non cambia nulla: salva soltanto il file del rapporto."],
        steps: [
          "Premi insieme il tasto Windows e R, scrivi cmd e premi Invio. Si apre il Prompt dei comandi.",
          "Scrivi powercfg /batteryreport e premi Invio. Windows salva il rapporto come battery-report.html, di solito nella tua cartella utente, e ne mostra il percorso completo.",
          "Scrivi start battery-report.html e premi Invio per aprirlo nel browser.",
          "Nella sezione delle batterie installate (Installed batteries) leggi capacità di progetto, capacità a carica completa e numero di cicli: nel rapporto in inglese sono DESIGN CAPACITY, FULL CHARGE CAPACITY e CYCLE COUNT, su Windows in italiano possono comparire tradotte.",
        ],
      },
      {
        h: "Come leggere i numeri",
        p: [
          "Dividi la capacità a carica completa per quella di progetto. Per esempio, 36.000 mWh su 50.000 mWh vuol dire che la batteria tiene il 72% di quanto teneva da nuova.",
          "Il numero di cicli dice, più o meno, quante cariche complete ha fatto la batteria. Non tutte le batterie lo comunicano: un trattino vuol dire che il valore non c'è.",
          "Più in basso, la cronologia della capacità (Battery capacity history) mostra come è cambiata la capacità a carica completa settimana dopo settimana, e le stime di durata (Battery life estimates) confrontano l'autonomia di oggi con quella che darebbe alla capacità di progetto.",
        ],
      },
      {
        h: "Cosa fare",
        p: [
          "Se la batteria dura ancora quanto ti serve, non c'è niente da fare. Su un portatile che sta quasi sempre attaccato alla corrente, una batteria usurata pesa ancora meno.",
          "Quando non ti basta più per la giornata, una batteria nuova riporta l'autonomia di prima. In molti portatili la batteria è dentro la scocca, quindi la strada più semplice è il produttore o un centro di assistenza; chiedi una batteria fatta per il tuo modello.",
          "Per rallentare l'usura, Microsoft dà questi consigli per i suoi Surface, che usano lo stesso tipo di batteria: tieni il portatile lontano dal calore (sole diretto, auto al sole), non lasciarlo attaccato al caricabatterie per periodi molto lunghi e, prima di metterlo via per un po', lascialo carico a circa metà. Tieni aggiornati anche driver e firmware. Alcuni portatili possono anche fermare la carica all'80% quando restano collegati: sui Surface lo fa la Ricarica intelligente, e alcuni modelli hanno l'opzione \"Limita all'80%\" nell'app Surface.",
        ],
      },
      {
        h: "Come lo mostra Fettle",
        p: [
          "Su un portatile, lo scontrino di Fettle ha una sezione Batteria. Fettle legge gli stessi due valori, capacità di progetto e capacità a carica completa, e li riassume in una riga. Legge e basta: non cambia nessuna impostazione della batteria o del risparmio energia.",
          "Dall'80% in su la riga è segnata OK e dice, con il tuo numero: \"Capacità della batteria: 87% di quella originale.\"",
          "Sotto l'80% è segnata OCCHIO, sotto il 60% AGISCI. In entrambi i casi dice \"Capacità della batteria: 72% di quella originale. Aspettati meno autonomia di quando era nuova.\" e consiglia: \"Sostituisci la batteria per riavere l'autonomia di prima.\" La percentuale è arrotondata per difetto e non supera mai il 100%, perché una batteria nuova può indicare un po' più della sua capacità di progetto.",
          "Sotto la riga ci sono i numeri: Capacità originale, Carica piena e, quando la batteria li comunica, Cicli di carica. I cicli sono solo un'informazione in più: il giudizio si basa soltanto sulla capacità.",
          "Su un PC fisso o un all-in-one senza batteria, la sezione Batteria non c'è. Se la batteria non indica la sua capacità originale, Fettle non tira a indovinare e dice: \"La batteria non ha indicato la sua capacità originale.\"",
        ],
      },
    ],
  },
};

export default guide;
