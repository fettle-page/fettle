// FAQ answers ease open and shut. Chrome does it in CSS (interpolate-size, site.css); Firefox and Safari
// can't animate to "auto" yet and would snap open, so here the same ease is played from the measured
// heights. Off for people who ask for less motion: the answers just open, as the browser does it.

export const OPEN_MS = 220;
export const WORDS_MS = 180;

/** Wires the ease where the CSS can't do it. Returns whether it did. */
export function initFaqEase(doc: Document, win: Window & typeof globalThis): boolean {
  if (win.CSS?.supports?.("interpolate-size", "allow-keywords")) return false;
  if (win.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return false;
  const easing = win.getComputedStyle(doc.documentElement).getPropertyValue("--ease-out").trim() || "ease-out";
  const running = new WeakMap<HTMLDetailsElement, Animation>();
  const closing = new WeakSet<HTMLDetailsElement>();

  const play = (item: HTMLDetailsElement, from: number, to: number, done: () => void) => {
    running.get(item)?.cancel();
    item.style.overflow = "clip";
    const run = item.animate([{ height: `${from}px` }, { height: `${to}px` }], { duration: OPEN_MS, easing });
    running.set(item, run);
    run.finished.then(
      () => {
        running.delete(item);
        item.style.overflow = "";
        done();
      },
      () => {}, // cancelled: the next ease takes over
    );
  };

  doc.addEventListener("click", (e) => {
    const summary = e.target instanceof win.Element ? (e.target as Element).closest("summary") : null;
    const item = summary?.parentElement;
    if (!(item instanceof win.HTMLDetailsElement) || !item.classList.contains("faq-item") || item.querySelector("summary") !== summary) return;
    e.preventDefault();
    const from = item.getBoundingClientRect().height;
    if (item.open && !closing.has(item)) {
      // Down to the question alone, then really close.
      closing.add(item);
      item.open = false;
      const to = item.getBoundingClientRect().height;
      item.open = true;
      play(item, from, to, () => {
        closing.delete(item);
        item.open = false;
      });
      return;
    }
    const wasClosed = !item.open;
    closing.delete(item);
    item.open = true;
    play(item, from, item.getBoundingClientRect().height, () => {});
    if (wasClosed) {
      item.querySelector(".faq-answer")?.animate([{ opacity: 0, translate: "0 4px" }, { opacity: 1, translate: "0 0" }], { duration: WORDS_MS, easing });
    }
  });
  return true;
}

if (typeof document !== "undefined") initFaqEase(document, window);
