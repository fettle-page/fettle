// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { initLegend } from "../src/scripts/legend.ts";

// The legend explains the receipt: point at a tag to see its lines on the sample,
// click to pin, Esc or a second click to clear.
const LEVELS = ["fine", "info", "keep_an_eye", "act_soon", "couldnt_check"];

function fixture() {
  document.body.innerHTML = `
    <section id="how">
      <div class="legend"><dl>
        ${LEVELS.map((l) => `<div><dt><button type="button" class="legend-pick" data-level="${l}" aria-pressed="false" aria-controls="sample" disabled>${l}<span class="sr-only" data-legend-show hidden>. Show</span></button></dt><dd>meaning <span class="legend-none" hidden>Not on this sample.</span></dd></div>`).join("")}
      </dl></div>
      <div id="sample">
        ${["fine", "info", "keep_an_eye", "couldnt_check"].map((l) => `<p class="line" data-level="${l}">${l}</p>`).join("")}
      </div>
    </section>`;
  initLegend(document);
}

const how = () => document.getElementById("how")!;
const pick = (level: string) => document.querySelector<HTMLButtonElement>(`.legend-pick[data-level="${level}"]`)!;
const point = (el: Element, type: "pointerenter" | "pointerleave", pointerType = "mouse") =>
  el.dispatchEvent(Object.assign(new Event(type), { pointerType }));
const esc = () => {
  const e = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
  document.dispatchEvent(e);
  return e;
};

describe("the legend explains the receipt", () => {
  beforeEach(() => {
    fixture();
    Element.prototype.scrollIntoView = vi.fn();
  });

  it("switches the tag buttons on, with their words for screen readers", () => {
    for (const l of LEVELS) {
      expect(pick(l).disabled, l).toBe(false);
      expect(pick(l).querySelector<HTMLElement>("[data-legend-show]")!.hidden, l).toBe(false);
    }
  });

  it("previews on pointer enter, clears on leave", () => {
    point(pick("info"), "pointerenter");
    expect(how().dataset.show).toBe("info");
    point(pick("info"), "pointerleave");
    expect(how().hasAttribute("data-show")).toBe(false);
  });

  it("ignores touch pointers for previews", () => {
    point(pick("info"), "pointerenter", "touch");
    expect(how().hasAttribute("data-show")).toBe(false);
  });

  it("a click pins, a second click unpins", () => {
    pick("fine").click();
    expect(how().dataset.show).toBe("fine");
    expect(pick("fine").getAttribute("aria-pressed")).toBe("true");
    pick("fine").click();
    expect(how().hasAttribute("data-show")).toBe(false);
    expect(pick("fine").getAttribute("aria-pressed")).toBe("false");
  });

  it("pinning another row moves the pin", () => {
    pick("fine").click();
    pick("info").click();
    expect(how().dataset.show).toBe("info");
    expect(pick("fine").getAttribute("aria-pressed")).toBe("false");
    expect(pick("info").getAttribute("aria-pressed")).toBe("true");
  });

  it("a preview wins while pointing, then the pin returns", () => {
    pick("fine").click();
    point(pick("keep_an_eye"), "pointerenter");
    expect(how().dataset.show).toBe("keep_an_eye");
    point(pick("keep_an_eye"), "pointerleave");
    expect(how().dataset.show).toBe("fine");
  });

  it("Esc clears a pin and nothing else", () => {
    pick("info").click();
    const e = esc();
    expect(how().hasAttribute("data-show")).toBe(false);
    expect(pick("info").getAttribute("aria-pressed")).toBe("false");
    expect(e.defaultPrevented).toBe(false);
    expect(esc().defaultPrevented).toBe(false); // nothing pinned: nothing happens
    expect(how().hasAttribute("data-show")).toBe(false);
  });

  it("says when a level isn't on the sample", () => {
    for (const l of LEVELS) {
      const none = pick(l).closest("div")!.querySelector(".legend-none")!;
      expect(none.hasAttribute("hidden"), l).toBe(l !== "act_soon");
    }
  });

  it("pinning scrolls the first match into view when none is visible", () => {
    Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
    const lines = [...document.querySelectorAll<HTMLElement>(".line")];
    for (const line of lines) line.getBoundingClientRect = () => ({ top: 2000, bottom: 2020 }) as DOMRect;
    point(pick("keep_an_eye"), "pointerenter"); // a hover never scrolls
    point(pick("keep_an_eye"), "pointerleave");
    expect(Element.prototype.scrollIntoView).not.toHaveBeenCalled();
    pick("keep_an_eye").click();
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledTimes(1);
    expect(vi.mocked(Element.prototype.scrollIntoView).mock.calls[0]![0]).toMatchObject({ block: "center" });
  });

  it("doesn't scroll when a match is already on screen", () => {
    Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
    for (const line of document.querySelectorAll<HTMLElement>(".line")) line.getBoundingClientRect = () => ({ top: 300, bottom: 320 }) as DOMRect;
    pick("info").click();
    expect(Element.prototype.scrollIntoView).not.toHaveBeenCalled();
  });
});
