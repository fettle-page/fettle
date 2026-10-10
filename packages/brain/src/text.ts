// Any copy key (receipt labels and verdict lines), for the website build and the Worker. The same
// JSON files the Rust brain embeds, so there's one source of wording.
import en from "../../../crates/fettle-verdicts/copy/en.json" with { type: "json" };
import it from "../../../crates/fettle-verdicts/copy/it.json" with { type: "json" };
import type { Locale } from "./locale.ts";
import { UI } from "./ui-text.ts";

const VERDICTS: Record<Locale, Record<string, string>> = { en, it };

/** A copy string; English when Italian lacks it; throws on a key that doesn't exist. */
export function text(key: string, locale: Locale): string {
  const value = UI[locale][key] ?? VERDICTS[locale][key] ?? UI.en[key] ?? VERDICTS.en[key];
  if (value === undefined) throw new Error(`unknown copy key: ${key}`);
  return value;
}
