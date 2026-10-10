// Structured data (JSON-LD). Only what the pages show and can back up: no ratings, no reviews, and
// no download link until v1.0 is out. JSON-LD is a data block, so the strict CSP doesn't apply to it.
import type { Locale } from "../i18n/paths.ts";
import type { Faq } from "./faq.ts";

const SITE = "https://fettle.page";

export function homeSchema(locale: Locale, url: string, description: string) {
  return {
    "@context": "https://schema.org",
    "@graph": [
      { "@type": "WebSite", "@id": `${SITE}/#website`, name: "Fettle", url: `${SITE}/`, inLanguage: locale },
      {
        "@type": "SoftwareApplication",
        "@id": `${SITE}/#app`,
        name: "Fettle",
        url,
        description,
        operatingSystem: "Windows 10, Windows 11",
        applicationCategory: "UtilitiesApplication",
        inLanguage: ["en", "it"],
        isAccessibleForFree: true,
        license: "https://opensource.org/licenses/MIT",
        offers: { "@type": "Offer", price: "0", priceCurrency: "EUR" },
      },
    ],
  };
}

export function faqSchema(locale: Locale, items: Faq[]) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    inLanguage: locale,
    mainEntity: items.map((f) => ({
      "@type": "Question",
      name: f.q,
      acceptedAnswer: { "@type": "Answer", text: f.a.join(" ") },
    })),
  };
}

/** JSON for a <script type="application/ld+json">: `<` escaped so the text can't close the element. */
export const ldJson = (data: unknown) => JSON.stringify(data).replaceAll("<", "\\u003c");
