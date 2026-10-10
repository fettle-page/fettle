import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { home } from "../src/i18n/home.ts";
import { pages as copy } from "../src/i18n/pages.ts";
import { release } from "../src/release.ts";
import { DIST, page, pages, text } from "./dist-helpers.ts";

// SignPath Foundation asks for a code signing policy on the project's site (https://signpath.org/terms):
// how releases are built, who does what, a privacy statement, and its attribution once it signs.
describe("the code signing policy page", () => {
  const en = text(page("code-signing.html"));
  const it_ = text(page("it/code-signing.html"));

  it("says plainly whether releases are signed, and how to check a download either way", () => {
    if (release.signed) {
      for (const t of [en, it_]) expect(t).toContain("SHA-256");
      expect(en).toContain("Free code signing provided by SignPath.io, certificate by SignPath Foundation.");
      expect(it_).toContain("certificato di SignPath Foundation");
      return;
    }
    expect(en).toContain("aren't signed yet");
    expect(en).toContain("SHA-256");
    expect(it_).toContain("non sono ancora firmate");
    for (const t of [en, it_]) expect(t).not.toContain("certificate by SignPath Foundation");
  });

  it("has SignPath's attribution ready, word for word, for when it signs", () => {
    expect(copy("en").codeSigning.signed.p).toContain("Free code signing provided by SignPath.io, certificate by SignPath Foundation.");
  });

  it("doesn't promise more review than a one-person project gives", () => {
    expect(en).not.toContain("Every change is reviewed");
    expect(it_).not.toContain("Ogni modifica viene rivista");
    expect(en).toContain("Changes from anyone else are reviewed");
    expect(it_).toContain("Le modifiche di chiunque altro vengono riviste");
  });

  it("names the approver and how releases are built", () => {
    for (const t of [en, it_]) {
      expect(t).toContain("candygoogy");
      expect(t).toContain("GitHub Actions");
    }
  });

  it("is honest about the update check instead of promising Fettle never connects on its own", () => {
    for (const t of [en, it_]) expect(t).not.toMatch(/will not transfer any information/i);
    expect(en).toMatch(/update check/);
    expect(it_).toMatch(/controllo degli aggiornamenti/);
    expect(page("code-signing.html")).toMatch(/href="\/privacy"/);
    expect(page("it/code-signing.html")).toMatch(/href="\/it\/privacy"/);
  });

  it("is linked from every page's footer, and from the download page", () => {
    for (const [file, html] of pages()) {
      if (/^(share|404)/.test(file) || !html.includes('class="foot-links"')) continue;
      expect(html, file).toMatch(html.includes('<html lang="it"') ? /href="\/it\/code-signing"/ : /href="\/code-signing"/);
    }
    expect(page("download.html")).toMatch(/<main[\s\S]*href="\/code-signing"[\s\S]*<\/main>/);
    expect(page("it/download.html")).toMatch(/<main[\s\S]*href="\/it\/code-signing"[\s\S]*<\/main>/);
  });

  // SignPath's terms ask for the words "Code signing policy" on the home page and the download page.
  it("is called the Code signing policy on the home page and the download page", () => {
    for (const file of ["index.html", "download.html"]) {
      expect(page(file), file).toMatch(/<a href="\/code-signing"[^>]*>Code signing policy<\/a>/);
    }
    for (const file of ["it.html", "it/download.html"]) {
      expect(page(file), file).toMatch(/<a href="\/it\/code-signing"[^>]*>Policy di firma del codice<\/a>/);
    }
  });

  it("is in the sitemap in both languages", () => {
    const sitemap = readFileSync(join(DIST, "sitemap-0.xml"), "utf8");
    expect(sitemap).toMatch(/\/code-signing</);
    expect(sitemap).toMatch(/\/it\/code-signing</);
  });

  // SignPath's terms: install instructions come with uninstall instructions.
  it("the download page says how to remove each download", () => {
    const en = text(page("download.html"));
    const it_ = text(page("it/download.html"));
    expect(en).toMatch(/Windows Settings > Apps/);
    expect(en).toMatch(/delete its folder/);
    expect(it_).toMatch(/Impostazioni di Windows > App/);
    expect(it_).toMatch(/elimina la sua cartella/);
  });

  it("once signed, no page still says Fettle will apply for signing", () => {
    if (!release.signed) return;
    for (const locale of ["en", "it"] as const) {
      const all = JSON.stringify([home(locale), copy(locale), faqs(locale)]);
      expect(all).not.toMatch(/will apply|chiederà la firma/);
    }
  });
});
