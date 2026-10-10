import { describe, expect, it } from "vitest";
import { faqs } from "../src/content/faq.ts";
import { page } from "./dist-helpers.ts";

function jsonLd(html: string): any {
  const m = /<script type="application\/ld\+json">([\s\S]*?)<\/script>/.exec(html);
  if (!m) throw new Error("no JSON-LD");
  return JSON.parse(m[1]!);
}

describe("structured data", () => {
  it("describes the site and the app on the landing pages, truthfully", () => {
    for (const [file, lang] of [["index.html", "en"], ["it.html", "it"]] as const) {
      const graph = jsonLd(page(file))["@graph"];
      const site = graph.find((n: any) => n["@type"] === "WebSite");
      const app = graph.find((n: any) => n["@type"] === "SoftwareApplication");
      expect(site).toMatchObject({ name: "Fettle", url: "https://fettle.page/", inLanguage: lang });
      expect(app).toMatchObject({
        name: "Fettle",
        operatingSystem: "Windows 10, Windows 11",
        applicationCategory: "UtilitiesApplication",
        isAccessibleForFree: true,
        license: "https://opensource.org/licenses/MIT",
        offers: { "@type": "Offer", price: "0", priceCurrency: "EUR" },
      });
      // Nothing we can't back up: no ratings, no reviews, no download link before release.
      expect(app.aggregateRating).toBeUndefined();
      expect(app.downloadUrl).toBeUndefined();
    }
  });

  it("marks up the FAQ with the same questions the page shows", () => {
    for (const [file, locale] of [["faq.html", "en"], ["it/faq.html", "it"]] as const) {
      const data = jsonLd(page(file));
      expect(data["@type"]).toBe("FAQPage");
      expect(data.mainEntity.map((q: any) => q.name)).toEqual(faqs(locale).map((f) => f.q));
      expect(data.mainEntity[0].acceptedAnswer.text).toBe(faqs(locale)[0]!.a.join(" "));
    }
  });
});
