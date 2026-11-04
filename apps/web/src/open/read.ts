// What /open does with a picked file: read it here, check it, and only then load the receipt.
// Nothing leaves the browser. Loaded with the page's later chunk (it carries zod), never up front.
import type { Locale, ReceiptModel } from "@fettle/brain";
import type { Brain, Zone } from "@fettle/brain/core";
import { parseReport, SCHEMA_VERSION } from "@fettle/report-schema";

export const OPEN_ERRORS = ["not_json", "not_fettle", "newer", "unreadable", "offline"] as const;
export type OpenError = (typeof OPEN_ERRORS)[number];
export type Opened = { ok: true; model: ReceiptModel } | { ok: false; error: OpenError };

/** A real report is a few KB, and one with every list full is about 40 KB: anything this big isn't one. */
export const MAX_FILE_BYTES = 1_048_576;

type Receipts = Pick<Brain, "receiptFor">;
const fail = (error: OpenError): Opened => ({ ok: false, error });
const isObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

/** `load` gives the brain; it runs only for a file that is a report (the wasm is fetched then). */
export async function openFile(file: Blob, locale: Locale, zone: Zone, load: () => Promise<Receipts>): Promise<Opened> {
  if (file.size > MAX_FILE_BYTES) return fail("not_fettle");
  let text: string;
  try {
    text = await file.text();
  } catch {
    return fail("unreadable");
  }
  let data: unknown;
  try {
    // A byte order mark, as some Windows editors add when a file is saved again.
    data = JSON.parse(text.replace(/^﻿/, ""));
  } catch {
    return fail("not_json");
  }
  // Before the schema: a newer report may not fit this version's shape at all.
  if (isObject(data) && typeof data.schema_version === "number" && data.schema_version > SCHEMA_VERSION) return fail("newer");
  const parsed = parseReport(data);
  if (!parsed.ok) return fail("not_fettle");
  let brain: Receipts;
  try {
    brain = await load();
  } catch {
    return fail("offline");
  }
  try {
    return { ok: true, model: brain.receiptFor(parsed.report, locale, zone).model };
  } catch {
    return fail("unreadable");
  }
}
