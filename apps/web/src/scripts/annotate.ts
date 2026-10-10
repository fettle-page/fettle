// The sample's margin notes and principles point at their part of the receipt, and the receipt points
// back: pointing at either marks both (class is-pointed, drawn in site.css). A tap toggles it.
import { revealIfHidden } from "./reveal.ts";

/** What each note or principle points at, inside the sample receipt (the same parts the CSS anchors use). */
export const TARGETS: Record<string, string> = {
  machine: ".r-head",
  storage: '[data-section="storage"]',
  battery: '[data-section="battery"]',
  memory: '[data-section="memory"]',
  windows: '[data-section="windows"]',
  startup: '[data-section="startup"]',
  graphics: '[data-section="graphics"]',
  plain: ".r-summary h2",
  numbers: '[data-section="storage"] .fold',
  unchecked: '.line[data-level="couldnt_check"]',
};

// Inside the receipt the most specific part wins: the ?? line before its section.
const NARROW_FIRST = ["unchecked", "numbers", "plain", "machine", "storage", "battery", "memory", "windows", "startup", "graphics"];

/** Wires the notes, the principles and the sample receipt together. Does nothing without them. */
export function initAnnotate(doc: Document): void {
  const root = doc.querySelector<HTMLElement>(".annotated");
  const receipt = root?.querySelector<HTMLElement>(".annotated-receipt");
  if (!root || !receipt) return;

  let current: string | null = null;
  let lastPointer = "";

  const keyOf = (el: EventTarget | null): string | null => {
    if (!(el instanceof Element) || !root.contains(el)) return null;
    const note = el.closest<HTMLElement>("[data-note-for]");
    if (note) return note.dataset.noteFor ?? null;
    const callout = el.closest<HTMLElement>("[data-callout]");
    if (callout) return callout.dataset.callout ?? null;
    if (!receipt.contains(el)) return null;
    return NARROW_FIRST.find((k) => el.closest(TARGETS[k]!) && receipt.contains(el.closest(TARGETS[k]!))) ?? null;
  };

  const mark = (key: string | null) => {
    if (key === current) return;
    for (const e of root.querySelectorAll(".is-pointed")) e.classList.remove("is-pointed");
    current = key;
    if (!key) return;
    const parts = [
      root.querySelector(`[data-note-for="${key}"]`),
      root.querySelector(`[data-callout="${key}"]`),
      TARGETS[key] ? receipt.querySelector(TARGETS[key]) : null,
    ];
    for (const p of parts) p?.classList.add("is-pointed");
  };

  root.addEventListener("pointerdown", (e) => (lastPointer = e.pointerType));
  root.addEventListener("pointerover", (e) => {
    if (e.pointerType === "mouse") mark(keyOf(e.target));
  });
  root.addEventListener("pointerout", (e) => {
    if (e.pointerType === "mouse") mark(keyOf(e.relatedTarget));
  });
  // A tap (or a click that didn't come from a mouse) on a note or principle toggles its mark.
  root.addEventListener("click", (e) => {
    if (lastPointer === "mouse") return;
    const source = e.target instanceof Element ? e.target.closest<HTMLElement>("[data-note-for], [data-callout]") : null;
    if (!source) return;
    const key = source.dataset.noteFor ?? source.dataset.callout ?? null;
    mark(current === key ? null : key);
    // On a phone the part can be off screen: bring it into view so the tap visibly does something.
    const sel = current ? TARGETS[current] : undefined;
    const part = sel ? receipt.querySelector(sel) : null;
    if (part) revealIfHidden([part], doc.defaultView);
  });
}

if (typeof document !== "undefined") initAnnotate(document);
