import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { appVersion } from "../src/app-version.ts";
import { checkChangelog, releases, type Release as ChangelogEntry } from "../src/content/changelog.ts";
import { checkRelease, downloads, latestVersion, links, release, type Release } from "../src/release.ts";
import { DIST, page, pages, text } from "./dist-helpers.ts";

// These follow the switch in src/release.ts: they hold before the release, on release day and after it.
const full: Release = { repo: "https://github.com/fettle-page/fettle", contact: "hello@fettle.page", published: "1.0.0", signed: false };
const dated: ChangelogEntry[] = [{ version: "1.0.0", date: "2026-11-02", notes: { en: ["First release."], it: ["Prima versione."] } }];

describe("the release switch", () => {
  it("is set in a way the site can honour (the site build checks this too)", () => {
    expect(() => checkRelease(release, releases)).not.toThrow();
  });

  it("accepts a complete release", () => {
    expect(() => checkRelease(full, dated)).not.toThrow();
  });

  it.each([
    ["no repo", { ...full, repo: null }, /repo/i],
    ["no contact", { ...full, contact: null }, /contact/i],
    ["a repo that isn't a GitHub page", { ...full, repo: "https://gitlab.com/a/fettle" }, /github/i],
    ["a script in place of the repo", { ...full, repo: "javascript:alert(1)" }, /github/i],
    ["a clone address in place of the repo page", { ...full, repo: "https://github.com/a/fettle.git" }, /github/i],
    ["dots in place of names", { ...full, repo: "https://github.com/./.." }, /github/i],
    ["a contact that isn't an email address", { ...full, contact: "nope" }, /email/i],
    ["a version that isn't x.y.z", { ...full, published: "v1.0.0" }, /version/i],
  ])("refuses to publish with %s", (_, r, message) => {
    expect(() => checkRelease(r as Release, dated)).toThrow(message);
  });

  it("checks the repo and contact even before publishing, so a typo shows up early", () => {
    const off: Release = { repo: null, contact: null, published: null, signed: false };
    expect(() => checkRelease({ ...off, repo: "javascript:alert(1)" }, [])).toThrow(/github/i);
    expect(() => checkRelease({ ...off, contact: "nope" }, [])).toThrow(/email/i);
  });

  it("refuses to publish a version the changelog hasn't dated", () => {
    expect(() => checkRelease(full, [{ ...dated[0]!, date: null }])).toThrow(/changelog/i);
    expect(() => checkRelease({ ...full, published: "1.0.1" }, dated)).toThrow(/1\.0\.1/);
  });
});

describe("the version the site offers", () => {
  it("is the published one, so bumping the app's version doesn't advertise files that aren't out yet", () => {
    expect(latestVersion(full, "1.0.1")).toBe("1.0.0");
    expect(latestVersion({ ...full, published: null }, "1.0.1")).toBe("1.0.1");
  });

  it("is what the update check reads", () => {
    expect(JSON.parse(readFileSync(join(DIST, "app", "latest.json"), "utf8")).version).toBe(latestVersion(release, appVersion));
  });
});

describe("downloads", () => {
  it("offers nothing until a version is published", () => {
    expect(downloads({ ...full, published: null })).toBeNull();
  });

  it("links the published version's installer, zip and checksums on GitHub Releases", () => {
    expect(downloads(full)).toEqual({
      version: "1.0.0",
      installer: "https://github.com/fettle-page/fettle/releases/download/v1.0.0/Fettle-Setup-1.0.0.exe",
      portable: "https://github.com/fettle-page/fettle/releases/download/v1.0.0/Fettle-1.0.0-portable.zip",
      checksums: "https://github.com/fettle-page/fettle/releases/download/v1.0.0/SHA256SUMS.txt",
      all: "https://github.com/fettle-page/fettle/releases",
    });
  });

  it("are on the built download page exactly when a version is published", () => {
    const files = downloads(release);
    for (const p of ["download.html", "it/download.html"]) {
      if (files) {
        for (const url of [files.installer, files.portable, files.checksums, files.all]) expect(page(p), p).toContain(`href="${url}"`);
      } else {
        expect(page(p), p).not.toContain("releases/download");
        expect(page(p), p).toContain("is-waiting");
      }
    }
  });
});

describe("the changelog", () => {
  it("has the 1.0.0 notes in both languages", () => {
    const first = releases.find((r) => r.version === "1.0.0");
    expect(first?.notes.en.length).toBeGreaterThan(3);
    expect(first?.notes.it.length).toBe(first?.notes.en.length);
  });

  it("says what the Deep check reads as the app does: disk health and the security chip", () => {
    const first = releases.find((r) => r.version === "1.0.0")!;
    expect(first.notes.en.find((n) => n.includes("Deep check"))).toContain("the health of your disks and the security chip");
    expect(first.notes.it.find((n) => n.includes("Controllo approfondito"))).toContain("lo stato dei dischi e il chip di sicurezza");
  });

  it("shows dated entries and hides undated ones", () => {
    const html = page("changelog.html");
    const headings = [...html.matchAll(/<h2[^>]*>([\s\S]*?)<\/h2>/g)].map(([, h]) => text(h!).trim());
    for (const r of releases) {
      const shown = headings.some((h) => h.startsWith(`${r.version} `));
      expect(shown, r.version).toBe(r.date !== null);
    }
    if (releases.every((r) => r.date === null)) expect(text(html)).toContain("No releases yet");
  });
});

describe("the changelog's dates", () => {
  const entry = (date: string | null): ChangelogEntry => ({ version: "1.0.5", date, notes: { en: [], it: [] } });

  it("are all real days written YYYY-MM-DD (the site build checks this too)", () => {
    expect(() => checkChangelog(releases)).not.toThrow();
    expect(() => checkChangelog([entry(null), entry("2026-10-15"), entry("2028-02-29")])).not.toThrow();
  });

  it("fail with a plain message naming the version and the bad value", () => {
    for (const bad of ["2026-13-01", "2026-02-30", "2026-10-1", "15/10/2026", "2026-10-15T00:00:00Z", ""]) {
      expect(() => checkChangelog([entry(bad)]), bad).toThrow(`changelog: 1.0.5 has date ${JSON.stringify(bad)}, expected a real day as YYYY-MM-DD`);
    }
  });

  it("are checked when the changelog page is built, before any date is formatted", () => {
    const view = readFileSync(new URL("../src/views/Changelog.astro", import.meta.url), "utf8");
    expect(view).toMatch(/checkChangelog\(releases\);[\s\S]*new Date\(r\.date\)/);
  });
});

describe("links to the source and a contact", () => {
  it("are empty until set", () => {
    expect(links({ ...full, repo: null, contact: null })).toEqual({ source: null, contact: null, email: null });
  });

  it("point at the repo and a mail link once set", () => {
    expect(links(full)).toEqual({ source: "https://github.com/fettle-page/fettle", contact: "mailto:hello@fettle.page", email: "hello@fettle.page" });
  });

  it("are read by the footer, the seal, the FAQ and the privacy page", () => {
    for (const file of ["components/Footer.astro", "views/Home.astro", "components/FaqList.astro", "views/Privacy.astro"]) {
      expect(readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8"), file).toMatch(/links\(release\)/);
    }
  });

  it("appear on the built site exactly when set", () => {
    const { source, contact, email } = links(release);
    const home = page("index.html");
    if (source) {
      expect(home).toContain(`href="${source}"`);
      expect(text(home)).toContain("Read the source on GitHub");
    } else {
      for (const [file] of pages()) expect(page(file), file).not.toMatch(/href="https:\/\/github\.com/);
    }
    if (contact) {
      expect(page("privacy.html")).toContain(`href="${contact}"`);
      expect(page("faq.html")).toContain(`href="${contact}"`);
      expect(text(page("it/privacy.html"))).toContain(`Scrivi a ${email}`);
    } else {
      for (const [file] of pages()) expect(page(file), file).not.toMatch(/href="mailto:/);
      expect(text(page("privacy.html"))).toContain("Contact details will be added here before the first release.");
      expect(text(page("it/privacy.html"))).toContain("I contatti verranno aggiunti qui prima della prima versione.");
    }
  });
});
