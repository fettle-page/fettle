import { describe, expect, it } from "vitest";
import { guides } from "../src/content/guides.ts";
import { page, text } from "./dist-helpers.ts";

const sitemap = page("sitemap-0.xml");
const main = (html: string) => html.slice(html.indexOf('<main id="main">'), html.indexOf("</main>"));
const footLinks = (html: string) => /<nav class="foot-links"[^>]*>([\s\S]*?)<\/nav>/.exec(html)?.[1] ?? "";

describe("the guides", () => {
  it("each come in both languages, part for part", () => {
    expect(guides.length).toBeGreaterThan(0);
    expect(new Set(guides.map((g) => g.slug)).size).toBe(guides.length);
    for (const g of guides) {
      expect(g.slug, g.slug).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/);
      expect(g.it.sections.length, g.slug).toBe(g.en.sections.length);
      expect(g.en.sections.length, g.slug).toBeGreaterThan(1);
      g.en.sections.forEach((s, i) => {
        const other = g.it.sections[i]!;
        expect(other.p.length, `${g.slug} section ${i}`).toBe(s.p.length);
        expect(other.steps?.length, `${g.slug} section ${i} steps`).toBe(s.steps?.length);
        expect(other.link?.href, `${g.slug} section ${i} link`).toBe(s.link?.href);
      });
      for (const c of [g.en, g.it]) {
        for (const field of [c.title, c.description, c.summary, c.kicker, c.h1, c.intro]) expect(field.trim().length, g.slug).toBeGreaterThan(2);
        expect(c.title, g.slug).toMatch(/ — Fettle$/);
      }
    }
  });
});

describe.each([["guides"], ...guides.map((g) => [`guides/${g.slug}`])])("/%s", (path) => {
  const en = page(`${path}.html`);
  const it_ = page(`it/${path}.html`);

  it("is built in both languages, each switching to the other", () => {
    expect(en).toMatch(/<html lang="en"/);
    expect(it_).toMatch(/<html lang="it"/);
    expect(en).toMatch(new RegExp(`href="/it/${path}" hreflang="it"`));
    expect(it_).toMatch(new RegExp(`href="/${path}" hreflang="en"`));
  });

  it("is in the sitemap in both languages", () => {
    expect(sitemap).toContain(`<loc>https://fettle.page/${path}</loc>`);
    expect(sitemap).toContain(`<loc>https://fettle.page/it/${path}</loc>`);
  });

  it("leaves no English sentence on the Italian page", () => {
    const sentences = [...main(en).matchAll(/<(?:p|li)[^>]*>([\s\S]*?)<\/(?:p|li)>/g)].map(([, s]) => text(s!).trim()).filter((s) => s.length > 30);
    for (const s of sentences) expect(text(it_), s).not.toContain(s);
  });
});

describe("the guides page", () => {
  it("links every guide, in each language", () => {
    for (const g of guides) {
      expect(page("guides.html")).toContain(`href="/guides/${g.slug}"`);
      expect(page("it/guides.html")).toContain(`href="/it/guides/${g.slug}"`);
    }
  });
});

describe("each guide", () => {
  it.each(guides.map((g) => g.slug))("%s ends with the way to download Fettle and back to the guides", (slug) => {
    for (const [file, prefix] of [[`guides/${slug}.html`, ""], [`it/guides/${slug}.html`, "/it"]] as const) {
      const links = /<p class="guide-links">([\s\S]*?)<\/p>/.exec(page(file))?.[1] ?? "";
      expect(links, file).toContain(`href="${prefix}/download"`);
      expect(links, file).toContain(`href="${prefix}/guides"`);
    }
  });
});

describe("a link inside a guide", () => {
  const links = guides.flatMap((g) => g.en.sections.flatMap((s, i) => (s.link ? [{ slug: g.slug, i, href: s.link.href }] : [])));

  it("exists somewhere, so this test checks something", () => {
    expect(links.length).toBeGreaterThan(0);
  });

  it.each(links)("$slug section $i points to a guide that exists, in each language", ({ slug, i, href }) => {
    expect(guides.map((g) => `/guides/${g.slug}`), href).toContain(href);
    const section = (html: string) => [...main(html).matchAll(/<section[^>]*>([\s\S]*?)<\/section>/g)][i]?.[1] ?? "";
    expect(section(page(`guides/${slug}.html`))).toContain(`href="${href}"`);
    expect(section(page(`it/guides/${slug}.html`))).toContain(`href="/it${href}"`);
  });
});

describe("the footer", () => {
  it("links to the guides in each language", () => {
    expect(footLinks(page("index.html"))).toContain('<a href="/guides">Guides</a>');
    expect(footLinks(page("it.html"))).toContain('<a href="/it/guides">Guide</a>');
  });
});
