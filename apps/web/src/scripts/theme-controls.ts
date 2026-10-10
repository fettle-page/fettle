// Wires the nav button and the footer picker to the theme. Both stay hidden without JavaScript.
import { applyTheme, nextVisibleTheme, prefersDark, readTheme, safeStorage, STORAGE_KEY, type Theme } from "./theme.ts";

const storage = safeStorage();
const button = document.querySelector<HTMLButtonElement>("[data-theme-toggle]");
const picker = document.querySelector<HTMLFieldSetElement>("[data-theme-picker]");
const status = document.querySelector<HTMLElement>("[data-theme-status]");

let current: Theme = readTheme(storage);

function nameOf(theme: Theme): string {
  return button?.dataset[theme] ?? theme;
}

/** Updates the controls; `announce` also tells screen readers (only for a change the visitor made). */
function show(theme: Theme, announce = false) {
  const name = nameOf(theme);
  if (button) {
    const nameEl = button.querySelector("[data-theme-name]");
    if (nameEl) nameEl.textContent = name;
    button.setAttribute("aria-label", `${button.dataset.label}: ${name}`);
  }
  for (const radio of picker?.querySelectorAll<HTMLInputElement>('input[name="theme"]') ?? []) radio.checked = radio.value === theme;
  if (announce && status) status.textContent = `${button?.dataset.label ?? "Theme"}: ${name}`;
}

let latest: object | null = null;

/**
 * Applies a choice. Where the browser can, the new theme is drawn over the old one (site.css, data-vt):
 * from the nav it's drawn down from the top; from the footer it spreads from the swatch you picked.
 */
function choose(theme: Theme, origin?: { x: number; y: number }) {
  current = theme;
  const update = () => {
    applyTheme(document, theme, storage);
    show(theme, true);
  };
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (typeof document.startViewTransition !== "function" || reduce) return update();
  const root = document.documentElement;
  if (origin) {
    // The circle has to reach the farthest corner, with 10% to spare: on a phone the page behind the
    // browser's toolbar is taller than the window.
    const r = Math.ceil(Math.hypot(Math.max(origin.x, innerWidth - origin.x), Math.max(origin.y, innerHeight - origin.y)) * 1.1);
    root.style.setProperty("--vt-x", `${origin.x}px`);
    root.style.setProperty("--vt-y", `${origin.y}px`);
    root.style.setProperty("--vt-r", `${r}px`);
  }
  root.dataset.vt = origin ? "theme-spot" : "theme";
  const transition = document.startViewTransition(update);
  latest = transition;
  // A quick second switch starts a new transition; only the latest one clears the flag.
  void transition.finished.finally(() => {
    if (latest === transition) delete root.dataset.vt;
  });
}

/** Picks up a choice made elsewhere: another tab, or this page restored from the back/forward cache. */
function resync() {
  const stored = readTheme(storage);
  if (stored === current) return;
  current = stored;
  applyTheme(document, stored, null);
  show(stored);
}

show(current);
if (button) {
  button.hidden = false;
  button.addEventListener("click", () => choose(nextVisibleTheme(current, prefersDark())));
}
if (picker) {
  picker.hidden = false;
  picker.addEventListener("change", (e) => {
    const radio = e.target as HTMLInputElement;
    // The swatch that's now checked, whether it was clicked, tapped or reached with the arrow keys.
    const box = (radio.closest("label") ?? radio).getBoundingClientRect();
    choose(radio.value as Theme, { x: box.left + box.width / 2, y: box.top + box.height / 2 });
  });
}
// While a theme change runs, Chromium sends clicks to <html> (the page is a picture for a moment), so a
// quick second click on a theme control would be lost: a click over one still counts.
document.addEventListener(
  "click",
  (e) => {
    const root = document.documentElement;
    if (e.target !== root || !root.dataset.vt?.startsWith("theme")) return;
    const over = (el: Element) => {
      const b = el.getBoundingClientRect();
      return b.width > 0 && e.clientX >= b.left && e.clientX <= b.right && e.clientY >= b.top && e.clientY <= b.bottom;
    };
    if (button && !button.hidden && over(button)) return button.click();
    for (const label of picker?.querySelectorAll("label") ?? []) {
      if (over(label)) return label.querySelector<HTMLInputElement>('input[name="theme"]')?.click();
    }
  },
  true,
);
window.addEventListener("storage", (e) => {
  if (e.key === STORAGE_KEY || e.key === null) resync();
});
window.addEventListener("pageshow", (e) => {
  if (e.persisted) resync();
});
