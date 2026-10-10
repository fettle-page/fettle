import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { pages } from "../src/i18n/pages.ts";
import { DIST } from "./dist-helpers.ts";
import { latestVersion, release } from "../src/release.ts";
import { appVersion } from "../src/app-version.ts";

// The Windows app's update check: every hour, day (the default) or week, only with its switch on, it GETs
// /app/latest.json from this site, with no query string, cookies or custom headers.

describe("the app's update check", () => {
  it("finds the latest version and where to get it", () => {
    const file = join(DIST, "app", "latest.json");
    expect(existsSync(file)).toBe(true);
    expect(JSON.parse(readFileSync(file, "utf8"))).toEqual({
      version: latestVersion(release, appVersion), // the published version, or apps/windows/Cargo.toml's before any release
      page: { en: "https://fettle.page/download", it: "https://fettle.page/it/download" },
    });
    for (const p of ["download.html", "it/download.html"]) expect(existsSync(join(DIST, p)), p).toBe(true);
  });

  it("is cached for an hour, not a year: a new version has to show up", () => {
    const headers = readFileSync(new URL("../public/_headers", import.meta.url), "utf8");
    expect(headers).toMatch(/^\/app\/\*\n {2}Cache-Control: public, max-age=3600$/m);
  });

  it("the privacy page says the check asks this website and sends nothing about the PC", () => {
    const en = pages("en").privacy.sections[0]!.p[0]!;
    const it_ = pages("it").privacy.sections[0]!.p[0]!;
    expect(en).toContain("by default once a day (you can choose every hour or every week), it asks this website for the latest version number, sending nothing about your PC");
    expect(it_).toContain("di norma una volta al giorno (puoi scegliere ogni ora o ogni settimana), chiede a questo sito il numero dell'ultima versione, senza inviare nulla sul tuo PC");
    for (const s of [en, it_]) expect(s).not.toContain("GitHub");
  });
});
