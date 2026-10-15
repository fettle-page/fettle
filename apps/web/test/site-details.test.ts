import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page, pages } from "./dist-helpers.ts";

const src = (rel: string) => readFileSync(new URL(`../${rel}`, import.meta.url), "utf8");
const titleOf = (html: string) => html.match(/<title>([^<]*)<\/title>/)?.[1] ?? "";

describe("tab titles", () => {
  it("use an em dash between the name and the rest, never a middle dot", () => {
    expect(titleOf(page("index.html"))).toBe("Fettle — a free check-up for Windows PCs");
    expect(titleOf(page("it.html"))).toBe("Fettle — il controllo gratuito per PC Windows");
    expect(titleOf(page("faq.html"))).toBe("Questions — Fettle");
    expect(titleOf(page("it/faq.html"))).toBe("Domande — Fettle");
    for (const [file, html] of pages()) expect(titleOf(html), file).not.toContain("·");
  });
});

describe("selected text", () => {
  it("is marked with the brand's soft green highlighter on the site and the share page", () => {
    for (const file of ["src/styles/site.css", "src/share/share.css"]) {
      expect(src(file), file).toMatch(/::selection\s*\{[^}]*background:\s*var\(--color-soft\)[^}]*color:\s*var\(--color-ink\)/);
    }
  });
});

describe("footer theme picker", () => {
  it("shows each theme as a swatch on a tape strip, with the radio kept for keyboards and screen readers", () => {
    const html = page("index.html");
    const picker = html.slice(html.indexOf("data-theme-picker"), html.indexOf("</fieldset>"));
    for (const v of ["auto", "light", "dark", "oled"]) {
      expect(picker, v).toMatch(new RegExp(`<input[^>]*class="sr-only"[^>]*value="${v}"|<input[^>]*value="${v}"[^>]*class="sr-only"`));
      expect(picker, v).toContain(`class="swatch swatch-${v}"`);
    }
    const css = src("src/styles/site.css");
    expect(css).not.toMatch(/\.foot-theme input \{[^}]*accent-color/);
    expect(css).toMatch(/\.foot-theme label:has\(input:checked\)/);
    expect(css).toMatch(/\.foot-theme label:has\(input:focus-visible\)/);
  });
});

describe("keyboard focus on label-maker tapes", () => {
  // A tape's torn edges are a clip-path, which also cuts off the focus outline drawn outside it, so a
  // focused tape drops the torn edges and shows the ring (WCAG 2.4.7).
  const body = (css: string, selector: string) =>
    [...css.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^}]*)\}/g)].filter(([, sel]) => sel!.split(/,(?![^(]*\))/).some((s) => s.trim() === selector)).map(([, , b]) => b).join(";");
  it("shows the focus ring on the site's tapes", () => {
    expect(body(src("src/styles/site.css"), ":is(a, button).tape:focus-visible")).toMatch(/clip-path:\s*none/);
  });
  it("shows the focus ring on the share page's tapes", () => {
    const rule = body(src("src/share/share.css"), ".tape:focus-visible");
    expect(rule).toMatch(/clip-path:\s*none/);
    expect(rule).toMatch(/outline:\s*2px solid/);
  });
});

describe("the privacy slip", () => {
  // Each row's word agrees with its label in Italian: "MAC e IP" are plural.
  const rows = (file: string) => {
    const slip = /<figure class="slip privacy-slip"[^>]*>([\s\S]*?)<\/figure>/.exec(page(file))![1]!;
    return [...slip.matchAll(/<dt>([^<]*)<\/dt>\s*<dd>[\s\S]*?<span>([^<]*)<\/span>\s*<\/dd>/g)].map(([, label, word]) => [label!, word!]);
  };
  it("says hidden in the right number, in each language", () => {
    const it_ = rows("it.html");
    expect(it_.find(([label]) => label === "MAC e IP")?.[1]).toBe("nascosti");
    for (const [label, word] of it_.filter(([label]) => label !== "MAC e IP")) expect(word, label).toBe("nascosto");
    expect(rows("it.html").length).toBe(4);
    for (const [label, word] of rows("index.html")) expect(word, label).toBe("hidden");
  });
});

describe("keyboard focus on the FAQ page's questions", () => {
  // There the questions have no side padding, so a ring drawn inside them would cross the first letter.
  it("draws the ring outside the question, clear of its text", () => {
    const css = src("src/styles/site.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const rule = /\.doc-sheet \.faq-item summary:focus-visible\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(rule).toMatch(/outline-offset:\s*[1-9]\d*px/);
  });
});

describe("the share page header on a narrow phone", () => {
  // In Italian at 360 px, the logo, "Scontrino condiviso" and "Chiaro" don't fit on one line.
  it("tightens its gaps rather than pushing the theme button off the screen, and keeps the logo whole", () => {
    const css = src("src/share/share.css").replace(/\/\*[\s\S]*?\*\//g, "");
    expect(/(?:^|\n)\.home\s*\{([^}]*)\}/.exec(css)?.[1]).toMatch(/flex-shrink:\s*0/);
    const narrow = /@media \(max-width: 400px\)\s*\{\s*\.bar\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(narrow).toMatch(/gap:\s*10px/);
  });
});

describe("the footer's theme picker", () => {
  // WebKit draws a floated legend as a caption above the options, so the visible label is an ordinary
  // element in the row and the legend only names the group for screen readers.
  it("names the group with a hidden legend and shows its label in the row, in every browser", () => {
    for (const file of ["index.html", "it/faq.html"]) {
      const picker = /<fieldset class="foot-theme"[^>]*>([\s\S]*?)<\/fieldset>/.exec(page(file))![1]!;
      expect(picker, file).toMatch(/^\s*<legend class="sr-only">[^<]+<\/legend>/);
      expect(picker, file).toMatch(/<div class="foot-theme-row"><span class="foot-theme-label" aria-hidden="true">[^<]+<\/span><div class="foot-theme-options">(\s*<label>[\s\S]*?<\/label>){4}\s*<\/div><\/div>/);
    }
    const css = src("src/styles/site.css");
    expect(css).not.toMatch(/\.foot-theme legend\s*\{[^}]*float/);
  });
});

describe("the free / open source seal", () => {
  // Firefox ignores textLength on a textPath, so each language's ring must come out about the length of
  // the circle (520 units) by itself: English twice round with tighter spacing, Italian (longer) once.
  it("fits each language's ring text round the circle without relying on textLength", () => {
    const ring = (file: string) => /<text class="seal-ring seal-ring-(\w+)"><textPath[^>]*>([^<]*)<\/textPath>/.exec(page(file))!;
    const en = ring("index.html");
    const it_ = ring("it.html");
    expect(en[1]).toBe("en");
    expect(it_[1]).toBe("it");
    expect(en[2].split("Free · open source · no tracking").length - 1).toBe(2);
    expect(it_[2].split("Gratis · open source · nessun tracciamento").length - 1).toBe(1);
    const css = src("src/styles/site.css");
    expect(css).toMatch(/\.seal-ring-en\s*\{\s*letter-spacing:\s*-0\.08\d*em;?\s*\}/);
    expect(css).toMatch(/\.seal-ring-it\s*\{\s*letter-spacing:\s*0\.18\d*em;?\s*\}/);
  });
});

describe("the top bar on phones and tablets", () => {
  // Measured in Playwright (target/polish/navtry.mjs): the logo, theme, language and Download tapes need
  // about 100 px more than a 360 px phone has, and "How it works" broke onto two lines up to 849 px.
  const css = () => src("src/styles/site.css").replace(/\/\*[\s\S]*?\*\//g, "");
  const media = (query: string) => {
    const s = css();
    const at = s.indexOf(`@media (${query})`);
    if (at < 0) return "";
    let depth = 0;
    for (let i = s.indexOf("{", at); i < s.length; i++) {
      if (s[i] === "{") depth++;
      else if (s[i] === "}" && --depth === 0) return s.slice(at, i);
    }
    return "";
  };
  it("never squeezes the logo", () => {
    expect(/(?:^|\n)\.nav-home\s*\{([^}]*)\}/.exec(css())?.[1]).toMatch(/flex-shrink:\s*0/);
  });
  it("keeps every tape on one line, showing the three page tapes only where they fit", () => {
    expect(/(?:^|\n)\.nav \.tape\s*\{([^}]*)\}/.exec(css())?.[1]).toMatch(/white-space:\s*nowrap/);
    expect(media("max-width: 860px")).toMatch(/\.wide-only\s*\{\s*display:\s*none/);
  });
  it("on phones leaves the theme to the footer's picker and tightens the tapes, in one row", () => {
    const phone = media("max-width: 479.98px");
    expect(phone).toMatch(/\.nav-theme\s*\{\s*display:\s*none/);
    expect(phone).toMatch(/\.nav \.tape\s*\{\s*padding-inline:\s*9px/);
    expect(media("max-width: 359.98px")).toMatch(/\.nav-wordmark\s*\{\s*height:\s*24px/);
    for (const file of ["index.html", "it/faq.html"]) expect(page(file), file).toMatch(/<li class="nav-theme"><button[^>]*class="tape theme-toggle"/);
  });
});
