// A self-healing cutting mat: click or drag on empty mat and a thin cut follows the pointer, then
// closes up and is gone (site.css, .cut.healing). Mouse and trackpad only; off for reduced motion.
// Nothing is saved, and anything that starts on text or paper behaves exactly as before.

export const HEAL_MS = 1500;
export const MAX_CUTS = 8;
/** A click without a drag makes a short cut this long, at the knife's angle. */
export const SHORT_CUT = 18;
export const KNIFE_DEG = -28;
const MIN_STEP = 3;
const MAX_POINTS = 60;
const MAX_LENGTH = 900;
const SVG = "http://www.w3.org/2000/svg";

/** Text, links, controls and paper: a pointer that starts on any of these is never a cut. */
const CONTENT =
  "a, button, input, label, summary, p, h1, h2, h3, li, dt, dd, figure, img, svg, .fettle-receipt, .slip, .legend, .live, " +
  ".doc-sheet, .foot-slip, .nav, .tech-stack, .seal, .callout, .margin-note";

/** True when the pointer is on bare mat, not on anything you'd read, click or select. */
export function isEmptyMat(target: Element): boolean {
  return target.closest(".mat") !== null && target.closest(CONTENT) === null;
}

type Point = [number, number];

/** Wires the cuts layer (svg.cuts inside .mat). Does nothing on touch-only devices or for reduced motion. */
export function initCuts(doc: Document, win: Window): void {
  const layer = doc.querySelector<SVGSVGElement>("svg.cuts");
  const mat = layer?.closest<HTMLElement>(".mat");
  if (!layer || !mat) return;
  if (!win.matchMedia?.("(pointer: fine)").matches || win.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return;

  let points: Point[] = [];
  let length = 0;
  let cut: SVGGElement | null = null;
  let frame = 0;

  // On the mat, not the window: the layer scrolls with the page, so a healing cut stays where it was drawn.
  const at = (e: MouseEvent): Point => {
    const box = mat.getBoundingClientRect();
    return [Math.round(e.clientX - box.left), Math.round(e.clientY - box.top)];
  };
  const draw = () => {
    frame = 0;
    const d = points.map(([x, y], i) => `${i ? "L" : "M"}${x} ${y}`).join(" ");
    for (const p of cut?.querySelectorAll("path") ?? []) p.setAttribute("d", d);
  };

  doc.addEventListener("pointerdown", (e) => {
    if ((e as PointerEvent).pointerType !== "mouse" || e.button !== 0 || e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) return;
    if (!(e.target instanceof Element) || !isEmptyMat(e.target)) return;
    e.preventDefault(); // no text selection starts on the mat
    // That also cancels what a click away normally does, so do it here: drop the selection, let go of focus.
    doc.getSelection()?.removeAllRanges();
    const focused = doc.activeElement;
    if (focused instanceof HTMLElement && focused !== doc.body) focused.blur();
    while (layer.querySelectorAll("g.cut").length >= MAX_CUTS) layer.querySelector("g.cut")?.remove();
    cut = doc.createElementNS(SVG, "g") as SVGGElement;
    cut.setAttribute("class", "cut");
    for (const kind of ["cut-light", "cut-dark"]) {
      const path = doc.createElementNS(SVG, "path");
      path.setAttribute("class", kind);
      cut.append(path);
    }
    layer.append(cut);
    points = [at(e)];
    length = 0;
    draw();
  });

  doc.addEventListener("pointermove", (e) => {
    if (!cut) return;
    // The button is already up (its release happened outside the window): end the cut here.
    if (!(e.buttons & 1)) return finish();
    if (points.length >= MAX_POINTS || length >= MAX_LENGTH) return;
    const [x, y] = at(e);
    const [px, py] = points[points.length - 1]!;
    const step = Math.hypot(x - px, y - py);
    if (step < MIN_STEP) return;
    points.push([x, y]);
    length += step;
    // Drawn at most once a frame; every point is kept.
    if (!frame) frame = win.requestAnimationFrame ? win.requestAnimationFrame(draw) : (draw(), 0);
  });

  const finish = () => {
    if (!cut) return;
    if (points.length === 1) {
      // A click: a short nick at the knife's angle, centred where you clicked.
      const [x, y] = points[0]!;
      const a = (KNIFE_DEG * Math.PI) / 180;
      const dx = (Math.cos(a) * SHORT_CUT) / 2;
      const dy = (Math.sin(a) * SHORT_CUT) / 2;
      points = [
        [Math.round((x - dx) * 10) / 10, Math.round((y - dy) * 10) / 10],
        [Math.round((x + dx) * 10) / 10, Math.round((y + dy) * 10) / 10],
      ];
    }
    if (frame) win.cancelAnimationFrame?.(frame);
    draw();
    const healing = cut;
    cut = null;
    void healing.getBoundingClientRect(); // lay it out first, so healing transitions from the drawn cut
    healing.classList.add("healing");
    win.setTimeout(() => healing.remove(), HEAL_MS);
  };
  doc.addEventListener("pointerup", finish);
  doc.documentElement.addEventListener("pointerleave", finish);
  doc.addEventListener("pointercancel", finish);
  win.addEventListener("blur", finish);
}

if (typeof document !== "undefined") initCuts(document, window);
