// /open's later chunk, imported only once a file is picked: zod, React, the receipt and its styles,
// and the brain loader (which fetches the wasm only for a file that passed the checks).
// The receipt's layout styles come as a hashed file of their own: imported as CSS, Astro would add
// them to the page up front as a second stylesheet (scripts/inline-css.mjs allows one).
import viewCss from "@fettle/receipt-ui/view.css?url";
import { loadBrain, type Brain } from "@fettle/brain/browser";
import { ReceiptView } from "@fettle/receipt-ui";
import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { Locale } from "../i18n/paths.ts";
import { openFile, type OpenError } from "./read.ts";

let root: Root | null = null;
let brain: Promise<Brain> | null = null;
// Compiled once per visit; a failed download is tried again with the next file.
const load = () =>
  (brain ??= loadBrain().catch((e: unknown) => {
    brain = null;
    throw e;
  }));

let styled: Promise<void> | null = null;
function viewStyles(): Promise<void> {
  return (styled ??= new Promise((resolve) => {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = viewCss;
    // Drawn either way: without these styles the receipt is still readable, just narrower.
    link.onload = link.onerror = () => resolve();
    document.head.append(link);
  }));
}

/** Draws the file's receipt into `target`, replacing any earlier one; the error otherwise. */
export async function show(file: Blob, locale: Locale, target: HTMLElement): Promise<OpenError | null> {
  const opened = await openFile(file, locale, { offsetMinutes: -new Date().getTimezoneOffset() }, load);
  if (!opened.ok) return opened.error;
  await viewStyles();
  root ??= createRoot(target);
  root.render(
    <StrictMode>
      <ReceiptView model={opened.model} />
    </StrictMode>,
  );
  return null;
}
