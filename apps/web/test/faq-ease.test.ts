// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { initFaqEase, OPEN_MS, WORDS_MS } from "../src/scripts/faq-ease.ts";

// Chrome eases FAQ answers open with CSS alone (interpolate-size, site.css). Firefox and Safari can't
// animate to "auto" yet, so they snapped open; this script gives them the same ease.
type Run = { el: Element; frames: Keyframe[]; opts: KeyframeAnimationOptions; finish: () => void; cancelled: boolean };
let runs: Run[];

function setup({ interpolate = false, reduce = false } = {}) {
  // A fresh page each time: the script listens on the document.
  const document = window.document.implementation.createHTMLDocument("faq");
  document.body.innerHTML = `<details class="faq-item" id="temps"><summary>Why no temperatures?</summary><div class="faq-answer"><p>Drivers.</p></div></details>`;
  document.documentElement.style.setProperty("--ease-out", "cubic-bezier(0.2, 0.7, 0.3, 1)");
  window.matchMedia = ((q: string) => ({ matches: q.includes("reduce") ? reduce : false, media: q, addEventListener() {}, removeEventListener() {} })) as never;
  window.CSS = { supports: (p: string, v?: string) => p === "interpolate-size" && v === "allow-keywords" && interpolate } as never;
  runs = [];
  Element.prototype.animate = function (this: Element, frames: Keyframe[], opts: KeyframeAnimationOptions) {
    let resolve!: () => void;
    let reject!: (e: unknown) => void;
    const finished = new Promise<void>((res, rej) => ((resolve = res), (reject = rej)));
    finished.catch(() => {});
    const run: Run = { el: this, frames, opts, finish: () => resolve(), cancelled: false };
    runs.push(run);
    return { finished, cancel: () => ((run.cancelled = true), reject(new DOMException("cancelled", "AbortError"))) } as unknown as Animation;
  } as never;
  const item = document.querySelector<HTMLDetailsElement>(".faq-item")!;
  // Closed, it's as tall as its question; open, the answer adds 120px.
  item.getBoundingClientRect = () => ({ height: item.open ? 180 : 60 }) as DOMRect;
  return { wired: initFaqEase(document, window), item, summary: item.querySelector("summary")! };
}
const click = (el: Element) => el.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
const heights = (r: Run) => r.frames.map((f) => f.height);
const settle = () => new Promise((r) => setTimeout(r, 0));

describe("FAQ answers ease open where CSS can't do it", () => {
  beforeEach(() => {
    runs = [];
  });

  it("opens the answer at once and eases its height from the question to the full answer", () => {
    const { item, summary } = setup();
    expect(click(summary)).toBe(false); // the browser's own snap is replaced
    expect(item.open).toBe(true);
    const grow = runs.find((r) => r.el === item)!;
    expect(heights(grow)).toEqual(["60px", "180px"]);
    expect(grow.opts).toMatchObject({ duration: OPEN_MS, easing: "cubic-bezier(0.2, 0.7, 0.3, 1)" });
  });

  it("brings the words in as it opens, as Chrome does", () => {
    const { summary } = setup();
    click(summary);
    const words = runs.find((r) => r.el.classList.contains("faq-answer"))!;
    expect(words.frames[0]).toMatchObject({ opacity: 0, translate: "0 4px" });
    expect(words.opts.duration).toBe(WORDS_MS);
  });

  it("closes only when the ease down has finished", async () => {
    const { item, summary } = setup();
    item.open = true;
    expect(click(summary)).toBe(false);
    const shrink = runs.at(-1)!;
    expect(heights(shrink)).toEqual(["180px", "60px"]);
    expect(item.open).toBe(true);
    shrink.finish();
    await settle();
    expect(item.open).toBe(false);
    expect(item.style.overflow).toBe("");
  });

  it("a second click while closing opens it again from where it is", async () => {
    const { item, summary } = setup();
    item.open = true;
    click(summary);
    const shrink = runs.at(-1)!;
    click(summary);
    expect(shrink.cancelled).toBe(true);
    await settle();
    expect(item.open).toBe(true);
    expect(runs.at(-1)!.el).toBe(item);
  });

  it("leaves Chrome to its CSS, and does nothing for people who ask for less motion", () => {
    for (const opts of [{ interpolate: true }, { reduce: true }]) {
      const { wired, item, summary } = setup(opts);
      expect(wired, JSON.stringify(opts)).toBe(false);
      expect(click(summary)).toBe(true);
      expect(runs).toHaveLength(0);
      expect(item.open).toBe(true); // the browser opens it itself, as before
    }
  });

  it("ignores clicks inside an open answer", () => {
    const { item } = setup();
    item.open = true;
    expect(click(item.querySelector("p")!)).toBe(true);
    expect(runs).toHaveLength(0);
  });
});
