// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { pickNotFound } from "../src/scripts/not-found.ts";

// One 404 page serves every missing address; an /it/ address switches it to Italian (not-found.test.ts
// checks the page itself).
describe("pickNotFound", () => {
  const fixture = () => {
    document.documentElement.lang = "en";
    document.title = "Not found — Fettle";
    document.body.innerHTML = `<section data-nf="en">en</section><section data-nf="it" data-title="Pagina non trovata — Fettle" hidden>it</section>`;
    return [document.querySelector<HTMLElement>('[data-nf="en"]')!, document.querySelector<HTMLElement>('[data-nf="it"]')!] as const;
  };

  it("an Italian address shows the Italian page", () => {
    for (const path of ["/it/nope", "/it", "/it/faq/old"]) {
      const [en, it_] = fixture();
      pickNotFound(document, path);
      expect([en.hidden, it_.hidden], path).toEqual([true, false]);
      expect(document.documentElement.lang).toBe("it");
      expect(document.title).toBe("Pagina non trovata — Fettle");
    }
  });

  it("anything else stays English", () => {
    for (const path of ["/nope", "/italy", "/its-gone", "/"]) {
      const [en, it_] = fixture();
      pickNotFound(document, path);
      expect([en.hidden, it_.hidden], path).toEqual([false, true]);
      expect(document.documentElement.lang).toBe("en");
    }
  });
});
