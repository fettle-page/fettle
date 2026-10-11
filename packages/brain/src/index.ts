// Browser-safe: locales, copy lookups and types. The wasm loaders are `@fettle/brain/node` and `/worker`.
export { acceptedLanguages, LOCALES, localeOf, pickLocale, toLocale, type Locale } from "./locale.ts";
export { text } from "./text.ts";
export { uiText } from "./ui-text.ts";
export type * from "./types.ts";
