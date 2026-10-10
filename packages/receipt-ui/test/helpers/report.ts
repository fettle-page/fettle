import { toLocale } from "@fettle/brain";
import { brain } from "@fettle/brain/node";
import type { Facts, Report } from "@fettle/report-schema";

export * from "./facts.ts";

export const report = (facts: Facts, elevated = false): Report => ({
  schema_version: 1,
  app_version: "0.1.0",
  collected_at: "2026-10-07T14:32:00+02:00",
  elevated,
  facts,
});

/** The receipt the Rust brain builds, as the share page and the app show it. */
export const model = (facts: Facts, locale = "en", elevated = false) =>
  // Rome in October (+02:00), so tests read the same on every machine: 14:32 at +02:00 is 14:32.
  brain.receiptFor(report(facts, elevated), toLocale(locale), { offsetMinutes: 120 }).model;
