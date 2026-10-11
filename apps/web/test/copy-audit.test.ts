// The website copy points from the 2026-10-11 audit: what each page must (and mustn't) say.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { releases } from "../src/content/changelog.ts";
import { faqs } from "../src/content/faq.ts";
import { home } from "../src/i18n/home.ts";
import { pages } from "../src/i18n/pages.ts";

const faq = (locale: "en" | "it", id: string) => faqs(locale).find((f) => f.id === id)!.a.join(" ");
const privacy = (locale: "en" | "it") => JSON.stringify(pages(locale).privacy.sections);
// strings.ts holds both languages; checking all of it for either one is stricter, never looser.
const stringsTs = readFileSync(new URL("../src/i18n/strings.ts", import.meta.url), "utf8");
const everything = (locale: "en" | "it") => JSON.stringify([pages(locale), faqs(locale), home(locale)]) + stringsTs;

describe("the Windows warnings", () => {
  it("name the Italian SmartScreen dialog by its real title", () => {
    expect(pages("it").download.warning.join(" ")).toContain('"Windows ha protetto il PC"');
    expect(everything("it")).not.toContain("PC protetto da Windows");
  });

  it("also explain the browser's own download warning, on the download page and in the FAQ", () => {
    for (const text of [pages("en").download.warning.join(" "), faq("en", "warning")]) {
      expect(text).toContain("isn't commonly downloaded");
      expect(text).toContain('"Keep anyway"');
    }
    for (const text of [pages("it").download.warning.join(" "), faq("it", "warning")]) {
      expect(text).toContain('"Mantieni comunque"');
    }
  });
});

describe("Italian wording", () => {
  it("says the update check sends nothing about the PC, not nothing to it", () => {
    expect(everything("it")).not.toContain("senza inviare nulla sul tuo PC");
    expect(privacy("it")).toContain("senza inviare alcuna informazione sul tuo PC");
  });

  it("drops the calques for 'by default'", () => {
    expect(everything("it")).not.toMatch(/in partenza/);
  });

  it("joins a negative list with né", () => {
    expect(faq("it", "forum")).toContain("email né indirizzi di rete");
  });
});

describe("what a share link carries", () => {
  it("is the report behind the receipt, as the privacy page says, not just the receipt", () => {
    for (const locale of ["en", "it"] as const) {
      expect(faq(locale, "share")).not.toMatch(/^Only the receipt|^Solo lo scontrino/);
      expect(home(locale).privP2).not.toMatch(/only the receipt you see|solo lo scontrino che vedi/);
    }
    expect(faq("en", "share")).toContain("report behind the receipt");
    expect(home("en").privP2).toContain("report behind the receipt");
  });
});

describe("the portable download", () => {
  it("says it checks for updates too", () => {
    expect(pages("en").download.formats[1]).toMatch(/update check/);
    expect(pages("it").download.formats[1]).toMatch(/controllo degli aggiornamenti/);
  });
});

describe("the privacy page", () => {
  it("says shared reports stay in the EU, and passes are renewed on use", () => {
    expect(privacy("en")).toMatch(/EU data centres/);
    expect(privacy("it")).toMatch(/data center nell'UE/);
    expect(privacy("en")).toMatch(/24 hours after it was last used/);
    expect(privacy("it")).toMatch(/24 ore dall'ultimo utilizzo/);
  });
});

describe("password-protected links", () => {
  // The report is stored as uploaded; only the password is hashed. Nothing may say it's encrypted.
  it("are never called encrypted", () => {
    expect(everything("en")).not.toMatch(/encrypt/i);
    expect(everything("it")).not.toMatch(/cifrat|crittograf/i);
  });
});

describe("what stays out of every receipt, file and link", () => {
  // The privacy pass hides full names, email addresses, the Windows domain and server names too.
  it("is listed in full on the privacy page", () => {
    const en = privacy("en");
    for (const item of ["your name and email address", "domain", "names of servers on your network"]) expect(en).toContain(item);
    const it_ = privacy("it");
    for (const item of ["il tuo nome e il tuo indirizzo email", "dominio", "nomi dei server della tua rete"]) expect(it_).toContain(item);
  });

  it("includes your name and email wherever the site sums it up", () => {
    for (const text of [faq("en", "share"), faq("en", "forum"), home("en").privP1]) expect(text).toMatch(/your name/);
    for (const text of [faq("en", "share"), faq("en", "forum"), home("en").privP1]) expect(text).toMatch(/email/);
    for (const text of [faq("it", "share"), faq("it", "forum"), home("it").privP1]) expect(text).toMatch(/il tuo nome/);
    for (const text of [faq("it", "share"), faq("it", "forum"), home("it").privP1]) expect(text).toMatch(/email/);
  });
});

describe("the privacy page's counting", () => {
  it("names everything counted per address each minute, and is dated for this change", () => {
    expect(privacy("en")).toContain("share page views, report reads and deletions");
    expect(privacy("it")).toContain("le visite alle pagine condivise, le letture e le eliminazioni dei report");
    expect(pages("en").privacy.updated).toBe("Last updated: 11 October 2026");
    expect(pages("it").privacy.updated).toMatch(/11 ottobre 2026/);
  });
});

describe("the Italian wording after review", () => {
  const changelogIt = JSON.stringify(releases.map((r) => r.notes.it));
  const all = everything("it") + changelogIt;

  it("says where the receipt comes from without a calque", () => {
    expect(all).not.toContain("dietro lo scontrino");
    expect(faq("it", "share")).toContain("Il report da cui nasce lo scontrino");
  });

  it("puts the browser's warning first in the sentence", () => {
    expect(all).not.toContain("avvisarti prima che il file");
    expect(pages("it").download.warning.join(" ")).toContain("Prima ancora, il browser potrebbe avvisarti che il file");
    expect(faq("it", "warning")).toContain("Prima ancora, il browser potrebbe avvisarti che il file");
  });

  it("doesn't put 'di serie' right above 'Numero di serie' on the home slip", () => {
    expect(home("it").slipHead).toBe("Nascosto in automatico");
  });

  it("says what is left out of a GUID-ending startup app's name", () => {
    expect(changelogIt).not.toContain("condivisa senza,");
    expect(changelogIt).toContain("condivisa senza il GUID");
  });
});

describe("the server's own privacy pass", () => {
  it("is on the privacy page: the server checks each upload again before storing it", () => {
    expect(privacy("en")).toContain("Our server checks it again before storing it");
    expect(privacy("it")).toContain("Il nostro server lo ricontrolla prima di conservarlo");
  });
});
