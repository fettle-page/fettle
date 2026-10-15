import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { home } from "../src/i18n/home.ts";
import { page } from "./dist-helpers.ts";

// Features of the Windows app the site must mention. Names match the app's own
// buttons (apps/windows/copy/en.json and it.json): "Copy", "Save", "My shared links".
const appCopy = (locale: "en" | "it") => JSON.parse(readFileSync(new URL(`../../windows/copy/${locale}.json`, import.meta.url), "utf8")) as Record<string, string>;
const css = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8");
const text = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

describe("the app's extra features on the site", () => {
  it("the technicians' list has copying for a forum and saving the full report", () => {
    expect(home("en").tech4).toBe("Copy as text for Discord or Reddit, or save the full report as JSON");
    expect(home("it").tech4).toBe("Copia come testo per Discord o Reddit, o salva il report completo in JSON");
    for (const [file, line] of [["index.html", home("en").tech4], ["it.html", home("it").tech4]] as const) {
      const list = /<ul class="checks"[^>]*>([\s\S]*?)<\/ul>/.exec(page(file))![1]!;
      expect(list.match(/<li>/g), file).toHaveLength(4);
      expect(text(list), file).toContain(line);
    }
  });

  it("the fourth tick is drawn in turn, like the other three", () => {
    expect(css).toMatch(/\.checks li:nth-child\(4\)::after \{ animation-range: cover 33% cover 45%; \}/);
  });

  it("a question says how to post the receipt on a forum, without private details", () => {
    for (const [locale, press] of [["en", "Press"], ["it", "Premi"]] as const) {
      const forum = faqs(locale).find((f) => f.id === "forum");
      expect(forum, locale).toBeDefined();
      const answer = forum!.a.join(" ");
      // The app's button is just "Copy" / "Copia": the site names it as the app shows it.
      expect(answer, locale).toContain(`${press} ${appCopy(locale)["export.copy"]} `);
      expect(answer, locale).not.toMatch(/Copy for Discord|Copia per Discord/);
      expect(answer, locale).toContain("JSON");
    }
    expect(faqs("en").find((f) => f.id === "forum")!.a[0]).toMatch(/^Yes\. Press Copy and the receipt goes on your clipboard as text, ready to paste into Discord, Reddit or a forum\. /);
    expect(faqs("it").find((f) => f.id === "forum")!.a[0]).toMatch(/^Sì\. Premi Copia e lo scontrino finisce negli appunti come testo, pronto da incollare su Discord, Reddit o un forum\. /);
    expect(faqs("en").find((f) => f.id === "forum")!.a.join(" ")).toContain("serial numbers, the PC's name, your name, username and email address, and network addresses");
  });

  it("the forum answer says how to save the full report, with the app's own button and file type names", () => {
    for (const locale of ["en", "it"] as const) {
      const app = appCopy(locale);
      const answer = faqs(locale).find((f) => f.id === "forum")!.a.join(" ");
      expect(answer, locale).toContain(`"${app["export.filter_json"]}"`);
      expect(answer, locale).toContain(locale === "en" ? `press ${app["export.json"]} and choose` : `premi ${app["export.json"]} e scegli`);
    }
    expect(faqs("en").find((f) => f.id === "forum")!.a.at(-1)).toBe(
      "If someone who knows PCs asks for the details, you can also save the full report as JSON and send them the file: press Save and choose \"Report for experts (JSON)\" as the file type.",
    );
    expect(faqs("it").find((f) => f.id === "forum")!.a.at(-1)).toBe(
      "Se qualcuno esperto di PC ti chiede i dettagli, puoi anche salvare il report completo in JSON e mandargli il file: premi Salva e scegli \"Report per esperti (JSON)\" come tipo di file.",
    );
  });

  it("the startup note claims only what the scan reads: apps, not services or scheduled tasks", () => {
    // crates/fettle-collect/src/sources/startup.rs reads the Run and RunOnce keys and the Startup folders.
    expect(home("en").trayStartup).toBe("The apps that start with Windows.");
    expect(home("it").trayStartup).toBe("Le app che partono insieme a Windows.");
  });

  it("describes battery health as the app does: capacity compared with new, not charge", () => {
    for (const locale of ["en", "it"] as const) expect(home(locale).trayBattery, locale).toBe(`${appCopy(locale)["blank.battery"]}.`);
    const readme = readFileSync(new URL("../../../README.md", import.meta.url), "utf8");
    expect(readme).toContain("| **Battery** | How much it holds compared with new, and how many cycles it has done. |");
  });

  it("the share answer says the app keeps a list of your links", () => {
    expect(faqs("en").find((f) => f.id === "share")!.a.join(" ")).toContain("My shared links");
    expect(faqs("it").find((f) => f.id === "share")!.a.join(" ")).toContain("I miei link condivisi");
  });
});
