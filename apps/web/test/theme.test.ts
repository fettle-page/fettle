// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { applyTheme, nextTheme, readTheme, STORAGE_KEY, THEME_COLOR } from "../src/scripts/theme.ts";
import { pages as copy } from "../src/i18n/pages.ts";

// A plain path: jsdom replaces the global URL, which node:fs rejects.
const earlyScript = readFileSync(join(import.meta.dirname, "../public/theme.js"), "utf8");

function metas() {
  document.head.innerHTML =
    '<meta name="theme-color" content="#E8EDE7" media="(prefers-color-scheme: light)">' +
    '<meta name="theme-color" content="#1F6A45" media="(prefers-color-scheme: dark)">';
}
const metaColors = () => [...document.querySelectorAll('meta[name="theme-color"]')].map((m) => m.getAttribute("content"));
const broken = { getItem: () => { throw new Error("blocked"); }, setItem: () => { throw new Error("blocked"); }, removeItem: () => { throw new Error("blocked"); } };

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  metas();
});
afterEach(() => localStorage.clear());

describe("theme logic", () => {
  it("cycles Auto → Light → Dark → OLED → Auto", () => {
    expect(nextTheme("auto")).toBe("light");
    expect(nextTheme("light")).toBe("dark");
    expect(nextTheme("dark")).toBe("oled");
    expect(nextTheme("oled")).toBe("auto");
  });

  it("reads a stored choice, and treats anything else as Auto", () => {
    expect(readTheme(localStorage)).toBe("auto");
    localStorage.setItem(STORAGE_KEY, "oled");
    expect(readTheme(localStorage)).toBe("oled");
    localStorage.setItem(STORAGE_KEY, "pink");
    expect(readTheme(localStorage)).toBe("auto");
    expect(readTheme(broken)).toBe("auto");
    expect(readTheme(null)).toBe("auto");
  });

  it("applies a choice to the page, the storage and the browser bar colour", () => {
    applyTheme(document, "oled", localStorage);
    expect(document.documentElement.dataset.theme).toBe("oled");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("oled");
    expect(metaColors()).toEqual([THEME_COLOR.oled, THEME_COLOR.oled]);
  });

  it("goes back to following the device on Auto", () => {
    applyTheme(document, "light", localStorage);
    applyTheme(document, "auto", localStorage);
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
    expect(metaColors()).toEqual([THEME_COLOR.light, THEME_COLOR.dark]);
  });

  it("still changes the page when storage is blocked (private windows)", () => {
    expect(() => applyTheme(document, "dark", broken)).not.toThrow();
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});

describe("the early script (public/theme.js)", () => {
  const run = () => new Function(earlyScript)();

  it("applies a stored choice before the page paints", () => {
    localStorage.setItem(STORAGE_KEY, "oled");
    run();
    expect(document.documentElement.dataset.theme).toBe("oled");
    expect(metaColors()).toEqual([THEME_COLOR.oled, THEME_COLOR.oled]);
  });

  it("ignores missing or unknown values", () => {
    run();
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    localStorage.setItem(STORAGE_KEY, "<script>");
    run();
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  describe("arrivals", () => {
    afterEach(() => document.documentElement.removeAttribute("data-arrival"));
    const reveal = (viewTransition: object | null) => window.dispatchEvent(Object.assign(new Event("pagereveal"), { viewTransition }));

    it("marks an arrival from another Fettle page", () => {
      run();
      reveal({});
      expect(document.documentElement.dataset.arrival).toBe("page");
    });

    it("leaves a fresh visit unmarked", () => {
      run();
      reveal(null);
      expect(document.documentElement.hasAttribute("data-arrival")).toBe(false);
    });
  });

  it("agrees with theme.ts on the bar colours", () => {
    for (const [theme, colour] of Object.entries(THEME_COLOR)) expect(earlyScript, theme).toContain(`${theme}: "${colour}"`);
  });
});

describe("privacy page", () => {
  it("says the theme choice is remembered in the browser", () => {
    expect(JSON.stringify(copy("en").privacy.sections)).toMatch(/theme/);
    expect(JSON.stringify(copy("it").privacy.sections)).toMatch(/tema/);
  });
});
