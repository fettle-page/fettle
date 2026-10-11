// The website copy points from the 2026-10-11 audit: what each page must (and mustn't) say.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
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
    expect(faq("it", "forum")).toContain("nome utente né indirizzi di rete");
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
