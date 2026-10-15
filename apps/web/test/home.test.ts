import { readFileSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { home } from "../src/i18n/home.ts";
import { DIST, page, text } from "./dist-helpers.ts";

const en = page("index.html");
const it_ = page("it.html");

describe("landing page", () => {
  it("asks the question in both languages", () => {
    expect(text(en)).toContain("Is your PC in fine fettle?");
    expect(text(it_)).toContain("Il tuo PC è in forma?");
  });

  it("has a complete live receipt even before (or without) JavaScript", () => {
    for (const key of ["screen", "threads", "memory", "system", "disk", "battery"]) expect(en).toContain(`data-fact="${key}"`);
    expect(text(en)).toContain("That's all a browser can see.");
  });

  it("has no stamp on the browser receipt", () => {
    for (const html of [en, it_]) expect(html).not.toContain("desk-stamp");
    expect(text(en)).not.toContain("in 5 seconds");
  });

  it("tags what a browser can't see with the receipt's own ?? tag", () => {
    for (const html of [en, it_]) {
      for (const key of ["disk", "battery"]) {
        const cell = html.slice(html.indexOf(`data-fact="${key}"`), html.indexOf("</dd>", html.indexOf(`data-fact="${key}"`)));
        expect(cell, key).toContain("tag-couldnt_check");
        expect(cell, key).toContain("??");
      }
    }
  });

  it("shows a real sample receipt, rendered at build time", () => {
    const sample = en.slice(en.indexOf('id="sample"'));
    expect(sample).toContain('class="fettle-receipt');
    expect(text(sample)).toContain("Needs a little fettling.");
    expect(text(it_.slice(it_.indexOf('id="sample"')))).toContain("Serve una sistemata.");
  });

  it("the sample is a Quick check that admits what it couldn't read", () => {
    const receipt = (html: string) => {
      const start = html.indexOf('class="fettle-receipt', html.indexOf('id="sample"'));
      return html.slice(start, html.indexOf("</article>", start));
    };
    for (const [html, scan, summary] of [
      [en, "Quick check (no administrator)", "Needs a little fettling."],
      [it_, "Controllo rapido (senza amministratore)", "Serve una sistemata."],
    ] as const) {
      const r = receipt(html);
      expect(r.match(/data-level="couldnt_check"/g) ?? []).toHaveLength(1);
      expect(text(r)).toContain(scan);
      expect(text(r)).toContain(summary);
      expect(text(r)).toContain("ST1000LM035");
    }
  });

  it("covers every section the app checks", () => {
    const tray = text(en.slice(en.indexOf('id="how"')));
    for (const word of ["Storage", "Battery", "Memory", "Windows", "Startup", "Graphics"]) expect(tray).toContain(word);
  });

  for (const [locale, file, download] of [["en", "index.html", "/download"], ["it", "it.html", "/it/download"]] as const) {
    const ending = () => {
      const html = page(file);
      return html.slice(html.indexOf('class="faq-teaser'), html.indexOf("</section>", html.indexOf('class="faq-teaser')));
    };
    const unquote = (s: string) => s.replace(/&#39;/g, "'").replace(/&#x27;/g, "'").replace(/&amp;/g, "&");

    it(`${locale}: ends with three questions printed open`, () => {
      const end = ending();
      const slip = end.slice(end.indexOf('class="slip questions-slip"'), end.indexOf("</figure>"));
      const shown = unquote(text(slip));
      const q = (id: string) => faqs(locale).find((f) => f.id === id)!.q;
      expect(end).not.toContain("<details");
      for (const id of ["admin", "changes", "mac"]) expect(shown, id).toContain(q(id));
      expect(shown).not.toContain(q("free"));
    });

    it(`${locale}: shows technicians the real A4 printout, not a sketch`, () => {
      const html = page(file);
      const tech = html.slice(html.indexOf('class="tech-stack"'), html.indexOf("</section>", html.indexOf('class="tech-stack"')));
      expect(tech).not.toContain("sheet-rule");
      expect(tech).toMatch(/class="sheet sheet-front print-sheet"[^>]*aria-hidden="true"/);
      expect(tech).toMatch(/class="sheet sheet-front print-sheet"[^>]*\binert\b/);
      expect(tech).not.toMatch(/\sid="/);
      const shown = unquote(text(tech));
      expect(shown).toContain(locale === "en" ? "Needs a little fettling." : "Serve una sistemata.");
      expect(shown).toContain("ST1000LM035");
      expect(page(file).match(/class="fettle-receipt/g) ?? []).toHaveLength(1);
    });

    it(`${locale}: lists the other questions under the download slip`, () => {
      const end = ending();
      const more = end.slice(end.indexOf('class="slip more-slip"'));
      const faq = download.replace("download", "faq");
      for (const id of ["free", "warning", "antivirus", "share", "temps", "windows10", "languages"]) {
        expect(more, id).toContain(`href="${faq}#${id}"`);
      }
      for (const id of ["admin", "changes", "mac"]) expect(more, id).not.toContain(`#${id}"`);
      expect(more).toContain(`href="${faq}"`);
      expect(end.slice(end.lastIndexOf("</figure>"))).not.toMatch(/class="text-link"/);
    });

    it(`${locale}: ends with a download button`, () => {
      const end = ending();
      expect(end).toMatch(new RegExp(`class="btn btn-primary"[^>]*href="${download}"|href="${download}"[^>]*class="btn btn-primary"`));
      // text() reads no-break spaces (which keep each dot with its word) as plain ones.
      expect(unquote(text(end))).toContain(home(locale).endFine.replace(/ /g, " "));
    });
  }

  it("keeps its JavaScript tiny", () => {
    const scripts = [...en.matchAll(/<script[^>]*\bsrc="([^"]+)"/g)].map(([, src]) => src!);
    const gz = scripts.reduce((sum, src) => sum + gzipSync(readFileSync(join(DIST, src))).byteLength, 0);
    expect(scripts.length).toBeGreaterThan(0);
    expect(gz).toBeLessThan(10_000);
  });

  it("is honest that the app isn't out yet", () => {
    expect(text(en)).toContain("Technician mode");
    expect(text(en)).toMatch(/coming in v1\.1/i);
  });
});
