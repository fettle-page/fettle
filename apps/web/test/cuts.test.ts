// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HEAL_MS, initCuts, isEmptyMat, MAX_CUTS, SHORT_CUT } from "../src/scripts/cuts.ts";

// Self-healing cuts on the mat.
function fixture({ fine = true, reduce = false, matTop = 0 } = {}) {
  document.body.innerHTML = `
    <div class="mat">
      <svg class="cuts" aria-hidden="true"></svg>
      <main><section class="hero"><div class="wrap"><p>Is your PC in fine fettle?</p><a href="/download">Download</a></div>
        <div class="slip"><span>hidden</span></div><div class="fettle-receipt"><div class="line">x</div></div></section></main>
    </div>`;
  window.matchMedia = ((q: string) => ({ matches: q.includes("pointer: fine") ? fine : q.includes("reduce") ? reduce : false, media: q, addEventListener() {}, removeEventListener() {} })) as never;
  document.querySelector<HTMLElement>(".mat")!.getBoundingClientRect = () => ({ left: 0, top: matTop, right: 1440, bottom: 5000, width: 1440, height: 5000, x: 0, y: matTop, toJSON() {} }) as DOMRect;
  initCuts(document, window);
}

const $ = (s: string) => document.querySelector<HTMLElement>(s)!;
const ev = (type: string, target: EventTarget, x: number, y: number, pointerType = "mouse", extra: MouseEventInit = {}) => {
  const e = Object.assign(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0, ...extra }), { pointerType });
  target.dispatchEvent(e);
  return e;
};
const cuts = () => [...document.querySelectorAll("svg.cuts g.cut")];
/** The points of a cut's first path, from its "M x y L x y …" data. */
const points = (g: Element) => [...(g.querySelector("path")!.getAttribute("d") ?? "").matchAll(/(-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);

describe("self-healing cuts", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("content starts nothing", () => {
    fixture();
    for (const s of ["p", "a", ".slip span", ".fettle-receipt .line"]) expect(isEmptyMat($(s)), s).toBe(false);
    for (const s of ["section", ".wrap", ".mat"]) expect(isEmptyMat($(s)), s).toBe(true);
  });

  it("a click on empty mat makes one short cut at the knife's angle", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 300, 200);
    ev("pointerup", document, 300, 200);
    expect(cuts()).toHaveLength(1);
    expect(cuts()[0]!.querySelectorAll("path")).toHaveLength(2);
    const [[x1, y1], [x2, y2]] = points(cuts()[0]!) as [[number, number], [number, number]];
    expect(Math.hypot(x2 - x1, y2 - y1)).toBeCloseTo(SHORT_CUT, 0);
    expect((Math.atan2(y2 - y1, x2 - x1) * 180) / Math.PI).toBeCloseTo(-28, 0);
    expect((x1 + x2) / 2).toBeCloseTo(300, 0);
    expect((y1 + y2) / 2).toBeCloseTo(200, 0);
  });

  it("a drag draws along the pointer", () => {
    fixture();
    const down = ev("pointerdown", $(".wrap"), 100, 100);
    expect(down.defaultPrevented).toBe(true); // no text selection starts on the mat
    ev("pointermove", document, 110, 100, "mouse", { buttons: 1 });
    ev("pointermove", document, 120, 100, "mouse", { buttons: 1 });
    ev("pointermove", document, 130, 100, "mouse", { buttons: 1 });
    ev("pointerup", document, 130, 100);
    expect(points(cuts()[0]!)).toEqual([[100, 100], [110, 100], [120, 100], [130, 100]]);
  });

  it("clicking empty mat still clears a selection and takes focus away, as a click away does", () => {
    fixture();
    const range = document.createRange();
    range.selectNodeContents($("p"));
    getSelection()!.addRange(range);
    $("a").focus();
    expect(document.activeElement).toBe($("a"));
    ev("pointerdown", $(".wrap"), 300, 200);
    ev("pointerup", document, 300, 200);
    expect(getSelection()!.rangeCount).toBe(0);
    expect(document.activeElement).toBe(document.body);
  });

  it("a drag that starts on content selects text as usual", () => {
    fixture();
    const down = ev("pointerdown", $("p"), 100, 100);
    ev("pointermove", document, 140, 100);
    ev("pointerup", document, 140, 100);
    expect(down.defaultPrevented).toBe(false);
    expect(cuts()).toHaveLength(0);
  });

  it("cuts stay put when the page scrolls", () => {
    fixture({ matTop: -500 }); // scrolled 500 px down: the mat's top is above the window
    ev("pointerdown", $(".wrap"), 300, 100);
    ev("pointerup", document, 300, 100);
    const [[, y1], [, y2]] = points(cuts()[0]!) as [[number, number], [number, number]];
    expect((y1 + y2) / 2).toBeCloseTo(600, 0); // drawn on the mat, so it scrolls with it
  });

  it("a move with the button already up ends the cut (a release outside the window was missed)", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 100, 100);
    ev("pointermove", document, 120, 100, "mouse", { buttons: 1 });
    ev("pointermove", document, 140, 100, "mouse", { buttons: 0 });
    expect(cuts()[0]!.classList.contains("healing")).toBe(true);
    ev("pointermove", document, 200, 100, "mouse", { buttons: 0 });
    expect(points(cuts()[0]!)).toEqual([[100, 100], [120, 100]]);
  });

  it("leaving the page ends the cut", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 100, 100);
    ev("pointermove", document, 120, 100, "mouse", { buttons: 1 });
    document.documentElement.dispatchEvent(Object.assign(new MouseEvent("pointerleave"), { pointerType: "mouse" }));
    expect(cuts()[0]!.classList.contains("healing")).toBe(true);
  });

  it("a cut heals and is gone", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 300, 200);
    ev("pointerup", document, 300, 200);
    expect(cuts()[0]!.classList.contains("healing")).toBe(true);
    vi.advanceTimersByTime(HEAL_MS + 50);
    expect(cuts()).toHaveLength(0);
  });

  it("at most 8 cuts", () => {
    fixture();
    for (let i = 0; i < MAX_CUTS + 1; i++) {
      ev("pointerdown", $(".wrap"), 100 + i * 30, 200);
      ev("pointerup", document, 100 + i * 30, 200);
    }
    expect(cuts()).toHaveLength(MAX_CUTS);
  });

  it("modifier keys and other buttons make no cut", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 300, 200, "mouse", { ctrlKey: true });
    ev("pointerup", document, 300, 200);
    ev("pointerdown", $(".wrap"), 300, 200, "mouse", { button: 2 });
    ev("pointerup", document, 300, 200);
    expect(cuts()).toHaveLength(0);
  });

  it("nothing for touch, coarse pointers or reduced motion", () => {
    fixture();
    ev("pointerdown", $(".wrap"), 300, 200, "touch");
    ev("pointerup", document, 300, 200, "touch");
    expect(cuts()).toHaveLength(0);
    fixture({ fine: false });
    ev("pointerdown", $(".wrap"), 300, 200);
    ev("pointerup", document, 300, 200);
    expect(cuts()).toHaveLength(0);
    fixture({ reduce: true });
    ev("pointerdown", $(".wrap"), 300, 200);
    ev("pointerup", document, 300, 200);
    expect(cuts()).toHaveLength(0);
  });
});
