// The receipt's own labels (tags, section names, the workbench), straight from the Rust crate's copy.
// Its own module, so the share page's browser bundle carries these and not the verdict lines.
import en from "../../../crates/fettle-receipt/copy/en.json" with { type: "json" };
import it from "../../../crates/fettle-receipt/copy/it.json" with { type: "json" };
import type { Locale } from "./locale.ts";

export const UI: Record<Locale, Record<string, string>> = { en, it };

/** A receipt label; English when Italian lacks it; throws on a key that doesn't exist. */
export function uiText(key: string, locale: Locale): string {
  const value = UI[locale][key] ?? UI.en[key];
  if (value === undefined) throw new Error(`unknown copy key: ${key}`);
  return value;
}
