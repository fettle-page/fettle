// The receipt's types, generated from the Rust structs (src/generated.ts, `npm run gen:brain-types`).
// Type-only: nothing here puts zod in a browser bundle.
import type { z } from "zod";
import type { ReceiptModelSchema, ReceiptSchema } from "./generated.ts";

export type Receipt = z.infer<typeof ReceiptSchema>;
export type ReceiptModel = z.infer<typeof ReceiptModelSchema>;
export type ModelSection = ReceiptModel["sections"][number];
export type ModelLine = ModelSection["lines"][number];
export type DetailGroup = ModelSection["details"][number];
export type DetailRow = DetailGroup["rows"][number];
export type Level = ModelLine["level"];
export type Tone = ReceiptModel["tone"];
