// /open up front: the file picker and drag and drop, nothing more. Reading, checking and drawing
// live in ./show.tsx, imported only once a file is picked, so the page itself stays light.
import type { Locale } from "../i18n/paths.ts";
import { openText } from "./copy.ts";

const locale: Locale = document.documentElement.lang === "it" ? "it" : "en";
const drop = document.querySelector<HTMLElement>("[data-open-drop]");
const input = document.querySelector<HTMLInputElement>("[data-open-input]");
const label = document.querySelector<HTMLElement>("[data-open-label]");
const message = document.querySelector<HTMLElement>("[data-open-message]");
const result = document.querySelector<HTMLElement>("[data-open-result]");
const receipt = document.querySelector<HTMLElement>("[data-open-receipt]");
const heading = result?.querySelector<HTMLElement>("h2");

function say(text: string, error = false) {
  if (!message) return;
  message.textContent = text;
  message.classList.toggle("is-error", error);
}

let busy = false;

async function take(file: File) {
  if (busy || !receipt || !result) return;
  busy = true;
  drop?.setAttribute("aria-busy", "true");
  say(openText(locale, "reading"));
  let error: string | null;
  try {
    const { show } = await import("./show.tsx");
    error = await show(file, locale, receipt);
  } catch {
    error = "offline";
  }
  busy = false;
  drop?.removeAttribute("aria-busy");
  if (error) {
    // An earlier receipt would look like this file's: take it away.
    result.hidden = true;
    say(openText(locale, `error.${error}` as Parameters<typeof openText>[1]), true);
    return;
  }
  result.hidden = false;
  say(openText(locale, "opened", { name: file.name }));
  if (label) label.textContent = openText(locale, "again");
  heading?.focus();
}

if (drop && input) {
  // Hidden in the HTML: without JavaScript the picker could do nothing (a <noscript> line says so).
  const pick = document.querySelector<HTMLElement>("[data-open-pick]");
  if (pick) pick.hidden = false;
  input.addEventListener("change", () => {
    const file = input.files?.[0];
    // Cleared, so picking the same file again still opens it.
    input.value = "";
    if (file) take(file);
  });
  // A file dropped anywhere on the page opens here; the browser would otherwise leave the page to show it.
  const carriesFiles = (e: DragEvent) => e.dataTransfer?.types.includes("Files") ?? false;
  let depth = 0;
  document.addEventListener("dragenter", (e) => {
    if (!carriesFiles(e)) return;
    depth += 1;
    drop.classList.add("is-over");
  });
  document.addEventListener("dragleave", (e) => {
    if (!carriesFiles(e)) return;
    depth = Math.max(0, depth - 1);
    if (depth === 0) drop.classList.remove("is-over");
  });
  document.addEventListener("dragover", (e) => {
    if (!carriesFiles(e)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  });
  document.addEventListener("drop", (e) => {
    if (!carriesFiles(e)) return;
    e.preventDefault();
    depth = 0;
    drop.classList.remove("is-over");
    const file = e.dataTransfer?.files[0];
    if (file) take(file);
  });
}
