import "./zod-config.ts";
import { z } from "zod";
import { ReportSchema } from "./generated.ts";

export { ReportSchema };
export type Report = z.infer<typeof ReportSchema>;
export type Facts = Report["facts"];

/** Must match `SCHEMA_VERSION` in crates/fettle-facts (a test checks this). */
export const SCHEMA_VERSION = 1;

/** Validates untrusted input (an uploaded or opened report) and strips unknown fields. */
export function parseReport(input: unknown): { ok: true; report: Report } | { ok: false; error: string } {
  const result = ReportSchema.safeParse(input);
  if (!result.success) return { ok: false, error: z.prettifyError(result.error) };
  // A newer report has fields this version would silently drop; don't pretend to understand it.
  if (result.data.schema_version > SCHEMA_VERSION) {
    return { ok: false, error: `This report uses a newer format (${result.data.schema_version}). Update Fettle to open it.` };
  }
  return { ok: true, report: result.data };
}
