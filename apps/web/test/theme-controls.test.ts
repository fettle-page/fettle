// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextVisibleTheme } from "../src/scripts/theme.ts";

describe("nextVisibleTheme", () => {
  it("skips the choice that would look the same as now", () => {
    expect(nextVisibleTheme("auto", false)).toBe("dark"); // device light: Auto already looks Light
    expect(nextVisibleTheme("auto", true)).toBe("light");
    expect(nextVisibleTheme("light", false)).toBe("dark");
    expect(nextVisibleTheme("dark", true)).toBe("oled");
    expect(nextVisibleTheme("oled", true)).toBe("auto");
    expect(nextVisibleTheme("oled", false)).toBe("auto");
  });
});

describe("site theme controls", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.body.innerHTML = `
      <button data-theme-toggle data-label="Theme" data-auto="Auto" data-light="Light" data-dark="Dark" data-oled="OLED" hidden><span data-theme-name>Auto</span></button>
      <span data-theme-status aria-live="polite"></span>
      <fieldset data-theme-picker hidden>
        <input type="radio" name="theme" value="auto"><input type="radio" name="theme" value="light">
        <input type="radio" name="theme" value="dark"><input type="radio" name="theme" value="oled">
      </fieldset>`;
    window.matchMedia = ((q: string) => ({ matches: q.includes("dark"), media: q, addEventListener() {}, removeEventListener() {} })) as never;
  });

  const label = () => document.querySelector("[data-theme-name]")!.textContent;
  const checked = () => (document.querySelector('input[name="theme"]:checked') as HTMLInputElement | null)?.value;

  it("shows itself, cycles, and announces the new theme", async () => {
    await import("../src/scripts/theme-controls.ts");
    const button = document.querySelector<HTMLButtonElement>("[data-theme-toggle]")!;
    expect(button.hidden).toBe(false);
    button.click(); // device dark: Auto → Light
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(label()).toBe("Light");
    expect(checked()).toBe("light");
    expect(document.querySelector("[data-theme-status]")!.textContent).toBe("Theme: Light");
  });

  it("follows a change made in another tab", async () => {
    await import("../src/scripts/theme-controls.ts");
    localStorage.setItem("fettle-theme", "oled");
    window.dispatchEvent(new StorageEvent("storage", { key: "fettle-theme", newValue: "oled" }));
    expect(document.documentElement.dataset.theme).toBe("oled");
    expect(label()).toBe("OLED");
    expect(checked()).toBe("oled");
  });

  it("re-syncs when the page comes back from the back/forward cache", async () => {
    await import("../src/scripts/theme-controls.ts");
    localStorage.setItem("fettle-theme", "dark");
    window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(label()).toBe("Dark");
  });

  describe("the switch is drawn over the old theme", () => {
    const finishers: (() => void)[] = [];
    const fakeTransition = () =>
      vi.fn((update: () => void) => {
        update();
        return { finished: new Promise<void>((done) => finishers.push(done)) };
      });
    const pick = (value: string) => {
      const radio = document.querySelector<HTMLInputElement>(`input[value="${value}"]`)!;
      radio.checked = true;
      radio.dispatchEvent(new Event("change", { bubbles: true }));
    };
    const settle = () => new Promise((r) => setTimeout(r, 0));
    const vt = () => document.documentElement.dataset.vt;

    beforeEach(() => {
      finishers.length = 0;
      delete document.documentElement.dataset.vt;
    });
    afterEach(() => {
      delete (document as { startViewTransition?: unknown }).startViewTransition;
    });

    it("draws the new theme over the old", async () => {
      const start = fakeTransition();
      Object.assign(document, { startViewTransition: start });
      await import("../src/scripts/theme-controls.ts");
      pick("oled");
      expect(start).toHaveBeenCalledTimes(1);
      expect(vt()).toBe("theme-spot"); // the footer picker spreads from its swatch
      expect(document.documentElement.dataset.theme).toBe("oled");
      expect(checked()).toBe("oled");
      expect(document.querySelector("[data-theme-status]")!.textContent).toBe("Theme: OLED");
      finishers[0]!();
      await settle();
      expect(vt()).toBeUndefined();
    });

    it("keeps data-vt until the latest switch finishes", async () => {
      Object.assign(document, { startViewTransition: fakeTransition() });
      await import("../src/scripts/theme-controls.ts");
      pick("light");
      pick("dark");
      finishers[0]!();
      await settle();
      expect(vt()).toBe("theme-spot");
      finishers[1]!();
      await settle();
      expect(vt()).toBeUndefined();
    });

    /** Puts each footer radio in a label with the given box, as in the real picker. */
    const labelled = (boxes: Record<string, { left: number; top: number; width: number; height: number }>) => {
      for (const radio of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
        const label = document.createElement("label");
        radio.replaceWith(label);
        label.append(radio);
        const b = boxes[radio.value] ?? { left: 0, top: 0, width: 0, height: 0 };
        label.getBoundingClientRect = () => ({ ...b, right: b.left + b.width, bottom: b.top + b.height, x: b.left, y: b.top, toJSON() {} }) as DOMRect;
      }
      Object.defineProperty(window, "innerWidth", { value: 1440, configurable: true });
      Object.defineProperty(window, "innerHeight", { value: 900, configurable: true });
    };
    const vars = () => ["--vt-x", "--vt-y", "--vt-r"].map((v) => document.documentElement.style.getPropertyValue(v));

    it("the footer spreads from its swatch", async () => {
      labelled({ oled: { left: 100, top: 700, width: 80, height: 32 } });
      Object.assign(document, { startViewTransition: fakeTransition() });
      await import("../src/scripts/theme-controls.ts");
      pick("oled");
      expect(vt()).toBe("theme-spot");
      // 10% to spare: on a phone the page behind the browser's toolbar is taller than the window.
      expect(vars()).toEqual(["140px", "716px", `${Math.ceil(Math.hypot(1300, 716) * 1.1)}px`]);
    });

    it("the nav button keeps the top-down wipe", async () => {
      Object.assign(document, { startViewTransition: fakeTransition() });
      await import("../src/scripts/theme-controls.ts");
      document.querySelector<HTMLButtonElement>("[data-theme-toggle]")!.click();
      expect(vt()).toBe("theme");
    });

    it("the spread starts at the checked swatch", async () => {
      labelled({ light: { left: 300, top: 600, width: 60, height: 30 }, dark: { left: 400, top: 650, width: 60, height: 30 } });
      Object.assign(document, { startViewTransition: fakeTransition() });
      await import("../src/scripts/theme-controls.ts");
      pick("light");
      pick("dark");
      expect(vars().slice(0, 2)).toEqual(["430px", "665px"]);
    });

    // While a change runs, Chromium sends clicks to <html>; a click over a theme control still counts.
    const clickHtml = (x: number, y: number) =>
      document.documentElement.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, clientX: x, clientY: y }));
    const boxOf = (el: Element, b: { left: number; top: number; width: number; height: number }) =>
      (el.getBoundingClientRect = () => ({ ...b, right: b.left + b.width, bottom: b.top + b.height, x: b.left, y: b.top, toJSON() {} }) as DOMRect);

    it("a click on the nav button during a change still counts", async () => {
      const start = fakeTransition();
      Object.assign(document, { startViewTransition: start });
      await import("../src/scripts/theme-controls.ts");
      const button = document.querySelector<HTMLButtonElement>("[data-theme-toggle]")!;
      boxOf(button, { left: 1000, top: 40, width: 80, height: 30 });
      button.click(); // device dark: Auto → Light
      expect(document.documentElement.dataset.theme).toBe("light");
      clickHtml(1040, 55); // the second click lands on <html> while the first change runs
      expect(start).toHaveBeenCalledTimes(2);
      expect(document.documentElement.dataset.theme).toBe("dark");
    });

    it("a click on a footer swatch during a change picks it", async () => {
      labelled({ light: { left: 300, top: 600, width: 60, height: 30 }, oled: { left: 500, top: 600, width: 60, height: 30 } });
      Object.assign(document, { startViewTransition: fakeTransition() });
      await import("../src/scripts/theme-controls.ts");
      pick("light");
      clickHtml(530, 615);
      expect(document.documentElement.dataset.theme).toBe("oled");
      expect(checked()).toBe("oled");
    });

    it("a click elsewhere during a change does nothing", async () => {
      const start = fakeTransition();
      Object.assign(document, { startViewTransition: start });
      await import("../src/scripts/theme-controls.ts");
      const button = document.querySelector<HTMLButtonElement>("[data-theme-toggle]")!;
      boxOf(button, { left: 1000, top: 40, width: 80, height: 30 });
      button.click();
      clickHtml(5, 5);
      expect(start).toHaveBeenCalledTimes(1);
      expect(document.documentElement.dataset.theme).toBe("light");
    });

    it("switches instantly without view transitions", async () => {
      await import("../src/scripts/theme-controls.ts");
      pick("dark");
      expect(document.documentElement.dataset.theme).toBe("dark");
      expect(vt()).toBeUndefined();
    });

    it("switches instantly under reduced motion", async () => {
      const start = fakeTransition();
      Object.assign(document, { startViewTransition: start });
      window.matchMedia = ((q: string) => ({ matches: q.includes("dark") || q.includes("reduce"), media: q, addEventListener() {}, removeEventListener() {} })) as never;
      await import("../src/scripts/theme-controls.ts");
      pick("oled");
      expect(start).not.toHaveBeenCalled();
      expect(document.documentElement.dataset.theme).toBe("oled");
      expect(vt()).toBeUndefined();
    });
  });
});
