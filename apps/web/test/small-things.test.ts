import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { page, text } from "./dist-helpers.ts";

const src = (rel: string) => readFileSync(new URL(`../${rel}`, import.meta.url), "utf8");
const root = (rel: string) => readFileSync(new URL(`../../../${rel}`, import.meta.url), "utf8");

describe("small things from the reviews", () => {
  it("404: switches to the Italian home, has no language alternates and isn't indexed", () => {
    const html = page("404.html");
    expect(html).toContain('<meta name="robots" content="noindex">');
    expect(html).not.toContain("hreflang=\"it\" href=\"https://");
    expect(html).toMatch(/href="\/it" hreflang="it"/);
    expect(html).not.toContain("/it/404");
  });

  it("live receipt: without JavaScript, disk and battery already say they can't be seen", () => {
    for (const [file, missing] of [["index.html", "can&#39;t see"], ["it.html", "non visibile"]] as const) {
      const html = page(file);
      for (const key of ["disk", "battery"]) expect(html, `${file} ${key}`).toMatch(new RegExp(`data-fact="${key}"[^>]*class="na"[^>]*>\\s*${missing}\\s*<`));
      expect(html).toMatch(/data-fact="screen"[^>]*>–</);
    }
  });

  it("live receipt script has no leftover printing state", () => {
    const code = src("src/scripts/live-receipt.ts");
    expect(code).not.toContain("data-state");
    expect(code).not.toContain("dataset.state");
  });

  it("download: the waiting button isn't marked up as a disabled control", () => {
    expect(page("download.html")).not.toContain("aria-disabled");
  });

  it("legend: tags come from the receipt's own copy, with words for screen readers", () => {
    const legend = (html: string) => html.slice(html.indexOf('class="legend"'), html.indexOf("</dl>", html.indexOf('class="legend"')));
    expect(legend(page("index.html"))).toContain('<span class="sr-only">Couldn&#39;t check</span>');
    expect(legend(page("it.html"))).toContain('<span class="sr-only">Non verificato</span>');
    expect(src("src/views/Home.astro")).not.toMatch(/OCCHIO|AGISCI|WATCH/);
  });

  it("footer year comes from the build, not a literal", () => {
    expect(src("src/components/Footer.astro")).not.toMatch(/year\s*=\s*\d{4}/);
    expect(text(page("index.html"))).toContain(`© ${new Date().getFullYear()} Fettle`);
  });

  it("leftovers are gone", () => {
    expect(root(".gitignore")).not.toContain("apps/share");
    expect(src("public/_headers")).not.toContain("interest-cohort");
  });

  it("dividers on paper follow the paper, so they show on dimmed OLED paper", () => {
    expect(src("src/styles/site.css")).not.toMatch(/#B9B4A8|#D3CFC4|#A9A59C|#BDB8AC/i);
  });

  it("the light tape has a hairline edge on the pale mat", () => {
    expect(root("packages/receipt-ui/src/themes.css")).toMatch(/\[data-theme="light"\] \{[^}]*--t-tape-light-ring:/);
    expect(src("src/styles/site.css")).toMatch(/\.tape\.light \{[^}]*var\(--t-tape-light-ring\)/);
  });

  it("ruler numbers are decoration drawn by CSS, not text", () => {
    const ruler = page("index.html").match(/<div class="ruler"[^>]*>([\s\S]*?)<\/div>/)![1]!;
    expect(ruler).toContain('<span data-n="5"></span>');
    expect(ruler.replace(/<[^>]+>/g, "").trim()).toBe("");
  });

  it("the hero receipt slides in without fading (visible, so painted, from the start)", () => {
    const css = src("src/styles/site.css");
    const feed = /@keyframes fettle-feed \{([\s\S]*?)\n\}/.exec(css)![1]!;
    expect(feed).not.toContain("opacity");
  });

  it("text-page headings use the text face, not the display cut meant for big headlines", () => {
    expect(src("src/styles/site.css")).toMatch(/\.prose h2 \{[^}]*font-family: var\(--font-display\)/);
  });

  it("the theme switch announces changes to screen readers", () => {
    expect(page("index.html")).toMatch(/<span[^>]*aria-live="polite"[^>]*data-theme-status/);
  });

  it("leaves no English on the Italian landing page", () => {
    const main = (html: string) => html.slice(html.indexOf('<main id="main">'), html.indexOf("</main>"));
    const paragraphs = [...main(page("index.html")).matchAll(/<p[^>]*>([\s\S]*?)<\/p>/g)]
      .map(([, inner]) => text(inner!).trim())
      .filter((p) => p.length > 30);
    const italian = text(page("it.html"));
    expect(paragraphs.length).toBeGreaterThan(5);
    for (const p of paragraphs) expect(italian, p).not.toContain(p);
  });
});
