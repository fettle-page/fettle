import { describe, expect, it } from "vitest";
import { home } from "../src/i18n/home.ts";
import { pages } from "../src/i18n/pages.ts";
import { faqs } from "../src/content/faq.ts";

/** Every string in a copy object, however deeply nested. */
const strings = (v: unknown): string[] =>
  typeof v === "string" ? [v] : Array.isArray(v) ? v.flatMap(strings) : v && typeof v === "object" ? Object.values(v).flatMap(strings) : [];

// The update check is on by default, so "sends nothing" must name it.
describe("claims about what the app sends", () => {
  const cases = [
    { locale: "en", claim: /sends nothing/i, update: /update/i },
    { locale: "it", claim: /non invia nulla/i, update: /aggiornament/i },
  ] as const;

  for (const { locale, claim, update } of cases) {
    it(`${locale}: every "sends nothing" line mentions the update check`, () => {
      const lines = strings([home(locale), pages(locale)]).filter((s) => claim.test(s));
      expect(lines.length).toBeGreaterThan(0);
      for (const s of lines) expect(s).toMatch(update);
    });
  }
});

// Checked against the Windows app's code on 2026-10-10 (apps/windows): what the site says it does, it does.
describe("claims about the Windows app", () => {
  const all = (locale: "en" | "it") => strings([home(locale), pages(locale), faqs(locale)]);

  it("a check takes a few seconds (the app says about a second; it waits 4 s at most)", () => {
    expect(home("en").lede).toContain("takes a few seconds");
    expect(home("it").lede).toContain("dura pochi secondi");
    for (const locale of ["en", "it"] as const) expect(all(locale).join(" "), locale).not.toMatch(/five seconds|cinque secondi/);
  });

  it("private details are never on a receipt: there is no switch that puts them there", () => {
    // The app's only switch shows the PC's name and serials on your own screen.
    for (const locale of ["en", "it"] as const) expect(all(locale).join(" "), locale).not.toMatch(/unless you turn them on|a meno che tu non li attivi/);
    expect(pages("en").privacy.sections[0]!.p.join(" ")).toContain("never appear on receipts, in saved files or in share links");
    expect(pages("it").privacy.sections[0]!.p.join(" ")).toContain("non compaiono mai negli scontrini, nei file salvati o nei link condivisi");
  });

  // The real packages (1.0.4): installer 12.5 MB, portable zip 15.4 MB (14.6 MiB), unpacked 37.7 MB plus the uninstaller once installed.
  it("the downloads are their real sizes: about 12 MB installer, 15 MB portable zip, 40 MB on disk", () => {
    const [installer, portable] = pages("en").download.formats;
    expect(installer).toContain("about 12 MB");
    expect(portable).toContain("Portable zip, about 15 MB");
    expect(pages("en").download.needs).toContain("About 40 MB of disk space");
    const [installerIt, portableIt] = pages("it").download.formats;
    expect(installerIt).toContain("circa 12 MB");
    expect(portableIt).toContain("Zip portatile, circa 15 MB");
    expect(pages("it").download.needs).toContain("Circa 40 MB di spazio su disco");
    expect(pages("en").download.description).toContain("an installer (about 12 MB) or a portable zip (about 15 MB)");
    expect(pages("it").download.description).toContain("un installer (circa 12 MB) o uno zip portatile (circa 15 MB)");
  });

  it("says the report behind the receipt is what's uploaded for a share link", () => {
    for (const [locale, line] of [
      ["en", "The report behind the receipt is uploaded only when you ask for a share link."],
      ["it", "Il report da cui nasce lo scontrino viene caricato solo quando chiedi un link di condivisione."],
    ] as const) {
      expect(all(locale).join(" "), locale).toContain(line);
      expect(all(locale).join(" "), locale).not.toMatch(/A receipt is uploaded|Lo scontrino viene caricato/);
    }
  });

  it("calls the everyday check by the app's name for it, the Quick check", () => {
    expect(faqs("en").find((f) => f.id === "admin")!.a[0]).toBe("No. The Quick check runs as a regular user.");
    expect(faqs("it").find((f) => f.id === "admin")!.a[0]).toBe("No. Il Controllo rapido funziona come utente standard.");
    for (const locale of ["en", "it"] as const) expect(all(locale).join(" "), locale).not.toMatch(/normal check|controllo normale/i);
  });

  it("says when the 24-hour pass runs out in plain Italian", () => {
    const privacy = pages("it").privacy.sections.flatMap((s) => s.p).join(" ");
    expect(privacy).toContain("il permesso scade 24 ore dopo l'ultimo utilizzo");
    expect(privacy).not.toContain("24 ore dall'ultimo");
  });

  it("doesn't call itself light: the app draws with the graphics card and uses about 215 MB doing it (user's choice)", () => {
    for (const locale of ["en", "it"] as const) {
      expect(all(locale).join(" "), locale).not.toMatch(/\blight and\b|\blightweight\b|\bleggero\b|\bleggera\b/i);
    }
    // Avoiding kernel drivers is about safety: nothing Fettle installs reaches into Windows itself.
    expect(faqs("en").find((f) => f.id === "temps")!.a.join(" ")).toContain("so nothing it installs reaches into Windows itself");
    expect(faqs("it").find((f) => f.id === "temps")!.a.join(" ")).toContain("così nulla di ciò che installa entra nel cuore di Windows");
  });

  it("there's an update check, not automatic updates, and no single portable .exe", () => {
    for (const locale of ["en", "it"] as const) {
      const text = all(locale).join(" ");
      // 15 MB is the portable zip now (see the sizes above); a single portable .exe is what must never be offered.
      expect(text, locale).not.toMatch(/\.exe[^.]*15 MB/);
      expect(text, locale).not.toMatch(/automatic updates|aggiornamenti automatici/);
      expect(text, locale).not.toMatch(/Portable \.exe|Exe portatile/);
    }
  });
});
