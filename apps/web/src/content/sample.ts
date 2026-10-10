// The landing page's sample receipt: a tired 2018 laptop. The model is built once per locale at
// build time by the Rust receipt (@fettle/brain), as the share Worker and the Windows app build theirs.
import type { ReceiptModel } from "@fettle/brain";
import { brain } from "@fettle/brain/node";
import { parseReport } from "@fettle/report-schema";
import type { Locale } from "../i18n/paths.ts";
import raw from "./sample-report.json" with { type: "json" };

const parsed = parseReport(raw);
if (!parsed.ok) throw new Error(`sample-report.json is not a valid report: ${parsed.error}`);
const report = parsed.report;

const models = new Map<Locale, ReceiptModel>();

export function sampleModel(locale: Locale): ReceiptModel {
  let model = models.get(locale);
  if (!model) {
    // UTC, so the built page is the same whichever machine builds it.
    model = brain.receiptFor(report, locale, "UTC").model;
    models.set(locale, model);
  }
  return model;
}
