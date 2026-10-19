import { existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { DIST, page, text } from "./dist-helpers.ts";
import { release } from "../src/release.ts";
import { releases } from "../src/content/changelog.ts";

const PAGES = ["faq", "privacy", "changelog", "download", "code-signing"];

const h1 = (html: string) => text(/<h1[^>]*>([\s\S]*?)<\/h1>/.exec(html)?.[1] ?? "").trim();
const paragraphs = (html: string) =>
  [...html.matchAll(/<p[^>]*>([\s\S]*?)<\/p>/g)].map(([, inner]) => text(inner!).trim()).filter((p) => p.length > 30);

describe.each(PAGES)("/%s", (name) => {
  const en = page(`${name}.html`);
  const it_ = page(`it/${name}.html`);

  it("exists in both languages with a heading", () => {
    expect(h1(en).length).toBeGreaterThan(2);
    expect(h1(it_).length).toBeGreaterThan(2);
    expect(h1(it_)).not.toBe(h1(en));
  });

  it("switches language to the same page", () => {
    expect(en).toMatch(new RegExp(`href="/it/${name}" hreflang="it"`));
    expect(it_).toMatch(new RegExp(`href="/${name}" hreflang="en"`));
  });

  it("leaves no English behind on the Italian page", () => {
    const italian = text(it_);
    // The English page's own sentences (outside the shared nav and footer) must not appear.
    const main = (html: string) => html.slice(html.indexOf('<main id="main">'), html.indexOf("</main>"));
    for (const sentence of paragraphs(main(en))) expect(italian, sentence).not.toContain(sentence);
  });
});

describe("pages", () => {
  it("are honest that the app isn't released yet, until it is", () => {
    if (release.published === null) expect(text(page("download.html"))).toContain("isn't released yet");
    if (releases.every((r) => r.date === null)) expect(text(page("changelog.html"))).toContain("No releases yet");
  });

  it("ask for testers on the download page, with the tester form and the email address", () => {
    for (const name of ["download.html", "it/download.html"]) {
      const html = page(name);
      expect(html, name).toContain(`href="${release.repo}/discussions/new?category=tester-reports"`);
      expect(html, name).toContain(`href="mailto:${release.contact}"`);
    }
  });

  it("explain the share page's only browser storage", () => {
    expect(text(page("privacy.html"))).toMatch(/24-hour pass/);
    expect(text(page("it/privacy.html"))).toMatch(/24 ore/);
  });

  it("have a 404 page with a way home", () => {
    expect(existsSync(join(DIST, "404.html"))).toBe(true);
    expect(page("404.html")).toMatch(/href="\/"/);
  });
});
