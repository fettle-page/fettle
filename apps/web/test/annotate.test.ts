// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { initAnnotate } from "../src/scripts/annotate.ts";

// Notes and receipt point at each other.
function fixture() {
  document.body.innerHTML = `
    <div class="annotated">
      <aside class="notes">
        <p class="margin-note" data-note-for="machine">reads the parts list</p>
        <p class="margin-note" data-note-for="storage">reads the drives</p>
      </aside>
      <div class="annotated-receipt">
        <div class="r-head">CPU · Board</div>
        <div class="r-summary"><h2>Needs a little fettling.</h2></div>
        <section data-section="storage"><h3>Storage</h3><p class="line" data-level="fine">fine</p><p class="line" data-level="couldnt_check">can't read</p><details class="fold"><summary>details</summary></details></section>
      </div>
      <ul class="callouts">
        <li class="callout" data-callout="unchecked">Says what it couldn't check.</li>
        <li class="callout" data-callout="plain">Plain words first.</li>
      </ul>
    </div>`;
  initAnnotate(document);
}

const $ = (s: string) => document.querySelector<HTMLElement>(s)!;
const over = (el: Element, type: "pointerover" | "pointerout", pointerType = "mouse", relatedTarget: Element | null = null) =>
  el.dispatchEvent(Object.assign(new Event(type, { bubbles: true }), { pointerType, relatedTarget }));
const pointed = () => [...document.querySelectorAll(".is-pointed")].map((e) => (e.hasAttribute("data-note-for") ? "margin-note" : e.tagName.toLowerCase()));

describe("notes and receipt point at each other", () => {
  beforeEach(() => {
    fixture();
    Element.prototype.scrollIntoView = vi.fn(); // jsdom has none; a tap may bring a part into view
  });

  it("pointing at a note marks its section", () => {
    over($('[data-note-for="storage"]'), "pointerover");
    expect($('[data-note-for="storage"]').classList.contains("is-pointed")).toBe(true);
    expect($('[data-section="storage"]').classList.contains("is-pointed")).toBe(true);
    over($('[data-note-for="storage"]'), "pointerout");
    expect(document.querySelectorAll(".is-pointed")).toHaveLength(0);
  });

  it("the machine note marks the receipt's top block", () => {
    over($('[data-note-for="machine"]'), "pointerover");
    expect($(".r-head").classList.contains("is-pointed")).toBe(true);
  });

  it("pointing at a section marks its note", () => {
    over($('[data-section="storage"] h3'), "pointerover");
    expect($('[data-note-for="storage"]').classList.contains("is-pointed")).toBe(true);
    expect($('[data-section="storage"]').classList.contains("is-pointed")).toBe(true);
  });

  it("moving inside a section keeps the mark", () => {
    over($('[data-section="storage"] h3'), "pointerover");
    over($('[data-section="storage"] h3'), "pointerout", "mouse", $('[data-section="storage"] .line[data-level="fine"]'));
    expect($('[data-note-for="storage"]').classList.contains("is-pointed")).toBe(true);
  });

  it("a callout marks the line that proves it", () => {
    over($('[data-callout="unchecked"]'), "pointerover");
    expect($('.line[data-level="couldnt_check"]').classList.contains("is-pointed")).toBe(true);
    expect($('[data-callout="unchecked"]').classList.contains("is-pointed")).toBe(true);
    over($('[data-callout="unchecked"]'), "pointerout");
    over($('[data-callout="plain"]'), "pointerover");
    expect($(".r-summary h2").classList.contains("is-pointed")).toBe(true);
  });

  it("a tap toggles the mark", () => {
    $('[data-note-for="storage"]').click();
    expect($('[data-section="storage"]').classList.contains("is-pointed")).toBe(true);
    $('[data-note-for="storage"]').click();
    expect(document.querySelectorAll(".is-pointed")).toHaveLength(0);
  });

  it("a tap brings the marked part into view when it's off screen", () => {
    Element.prototype.scrollIntoView = vi.fn();
    Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
    $(".r-summary h2").getBoundingClientRect = () => ({ top: 900, bottom: 940 }) as DOMRect;
    $('[data-callout="plain"]').click();
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledTimes(1);
    expect(vi.mocked(Element.prototype.scrollIntoView).mock.calls[0]![0]).toMatchObject({ block: "center" });
  });

  it("a tap doesn't scroll when the part is already on screen, and pointing never scrolls", () => {
    Element.prototype.scrollIntoView = vi.fn();
    Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
    $('[data-section="storage"]').getBoundingClientRect = () => ({ top: 300, bottom: 500 }) as DOMRect;
    $(".r-summary h2").getBoundingClientRect = () => ({ top: 900, bottom: 940 }) as DOMRect;
    $('[data-note-for="storage"]').click();
    over($('[data-callout="plain"]'), "pointerover");
    expect(Element.prototype.scrollIntoView).not.toHaveBeenCalled();
  });

  it("touch pointers don't mark on pointer over", () => {
    over($('[data-note-for="storage"]'), "pointerover", "touch");
    expect(document.querySelectorAll(".is-pointed")).toHaveLength(0);
  });

  it("only one thing is marked at a time", () => {
    over($('[data-note-for="machine"]'), "pointerover");
    over($('[data-note-for="machine"]'), "pointerout", "mouse", $('[data-note-for="storage"]'));
    over($('[data-note-for="storage"]'), "pointerover");
    expect(pointed().sort()).toEqual(["margin-note", "section"].sort());
  });
});
