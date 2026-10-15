// The legend explains the receipt: pointing at a tag shows its lines on the sample (site.css,
// #how[data-show]); a click pins it, a second click or Esc clears it. Without JavaScript the legend
// simply reads as a legend: the tag buttons stay switched off until this script runs.
import { revealIfHidden } from "./reveal.ts";

/** Wires the legend's tag buttons to the sample receipt. Does nothing on pages without them. */
export function initLegend(doc: Document): void {
  const how = doc.getElementById("how");
  const picks = [...doc.querySelectorAll<HTMLButtonElement>(".legend-pick")];
  if (!how || picks.length === 0) return;
  const sample = doc.getElementById("sample") ?? how;
  const lines = (level: string) => [...sample.querySelectorAll<HTMLElement>(`.line[data-level="${level}"]`)];

  let pinned: string | null = null;
  let previewing: string | null = null;

  const render = () => {
    const show = previewing ?? pinned;
    if (show) how.dataset.show = show;
    else delete how.dataset.show;
    for (const b of picks) b.setAttribute("aria-pressed", String(b.dataset.level === pinned));
  };

  for (const b of picks) {
    const level = b.dataset.level!;
    b.disabled = false;
    const show = b.querySelector<HTMLElement>("[data-legend-show]");
    if (show) show.hidden = false;
    const none = b.closest("div")?.querySelector<HTMLElement>(".legend-none");
    if (none) none.hidden = lines(level).length > 0;
    b.addEventListener("pointerenter", (e) => {
      if (e.pointerType !== "mouse") return;
      previewing = level;
      render();
    });
    b.addEventListener("pointerleave", (e) => {
      if (e.pointerType !== "mouse") return;
      previewing = null;
      render();
    });
    b.addEventListener("click", () => {
      pinned = pinned === level ? null : level;
      render();
      if (pinned) revealIfHidden(lines(pinned), doc.defaultView); // on a pin, never on a hover
    });
  }

  doc.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || pinned === null) return;
    pinned = null;
    render();
  });
}

if (typeof document !== "undefined") initLegend(document);
