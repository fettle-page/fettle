// Brings something you asked to see into view: if none of the elements is on screen, the first one
// scrolls to the middle (smoothly, or at once for people who ask for less motion). Used after a click
// or tap, never on a hover.

/** Scrolls the first element to the centre unless one of them is already visible. */
export function revealIfHidden(els: Element[], win: Window | null): void {
  if (!els.length || !win) return;
  const h = win.innerHeight;
  const visible = els.some((el) => {
    const r = el.getBoundingClientRect();
    return r.bottom > 0 && r.top < h;
  });
  if (visible) return;
  const still = win.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  els[0]!.scrollIntoView({ block: "center", behavior: still ? "auto" : "smooth" });
}
