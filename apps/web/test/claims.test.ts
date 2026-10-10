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

  // The real packages (1.0.0): installer 11.9 MB, portable zip 14.4 MB, the app 34 MB installed.
  it("the downloads are their real sizes: about 12 MB installer, 14 MB portable zip, 35 MB on disk", () => {
    const [installer, portable] = pages("en").download.formats;
    expect(installer).toContain("about 12 MB");
    expect(portable).toContain("Portable zip, about 14 MB");
    expect(pages("en").download.needs).toContain("About 35 MB of disk space");
    const [installerIt, portableIt] = pages("it").download.formats;
    expect(installerIt).toContain("circa 12 MB");
    expect(portableIt).toContain("Zip portatile, circa 14 MB");
    expect(pages("it").download.needs).toContain("Circa 35 MB di spazio su disco");
    expect(pages("en").download.description).toContain("an installer (about 12 MB) or a portable zip (about 14 MB)");
    expect(pages("it").download.description).toContain("un installer (circa 12 MB) o uno zip portatile (circa 14 MB)");
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
      expect(text, locale).not.toContain("15 MB");
      expect(text, locale).not.toMatch(/automatic updates|aggiornamenti automatici/);
      expect(text, locale).not.toMatch(/Portable \.exe|Exe portatile/);
    }
  });
});
