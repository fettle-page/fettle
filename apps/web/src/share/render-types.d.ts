// What the Worker sees of render.tsx. A declaration boundary, so the Worker's type check doesn't
// pull in browser code (window, document) it has no types for. test/share/render-types.test.ts
// keeps SharePayload here identical to payload.ts.
import type { Locale, ReceiptModel } from "@fettle/brain";

export type SharePayload =
  | { kind: "report"; id: string; expiresAt: string; locale?: Locale; model: ReceiptModel }
  | { kind: "locked"; id: string; expiresAt: string; locale?: Locale }
  | { kind: "missing"; locale?: Locale }
  | { kind: "delete"; id: string; locale?: Locale };

export declare function renderSharePage(payload: SharePayload, locale: Locale): string;
