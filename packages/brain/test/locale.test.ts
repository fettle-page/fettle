import { describe, expect, it } from "vitest";
import { acceptedLanguages, localeOf, pickLocale, toLocale } from "../src/index.ts";

// One way to choose the language, used by the share server (Accept-Language) and the share page in
// the browser (navigator.languages): exact language tags, never a prefix of a longer word.
describe("localeOf", () => {
  it("reads the language of a tag, in any case and with any region", () => {
    expect(localeOf("it")).toBe("it");
    expect(localeOf("IT")).toBe("it");
    expect(localeOf("it-IT")).toBe("it");
    expect(localeOf("it_CH")).toBe("it");
    expect(localeOf(" en-GB ")).toBe("en");
  });

  it("doesn't take a longer word for a language", () => {
    expect(localeOf("items")).toBeNull();
    expect(localeOf("english")).toBeNull();
    expect(localeOf("de-DE")).toBeNull();
    expect(localeOf("")).toBeNull();
  });

  it("is what toLocale falls back from", () => {
    expect(toLocale("items")).toBe("en");
    expect(toLocale("it-IT")).toBe("it");
  });
});

describe("acceptedLanguages", () => {
  it("orders by q-value, keeps the header's order for ties, and drops q=0", () => {
    expect(acceptedLanguages("en;q=0.5, it;q=0.9, de")).toEqual(["de", "it", "en"]);
    expect(acceptedLanguages("it, en")).toEqual(["it", "en"]);
    expect(acceptedLanguages("en;q=0, it")).toEqual(["it"]);
    expect(acceptedLanguages("it;q=oops, en")).toEqual(["en"]);
  });

  it("takes an empty or missing header as no preference", () => {
    expect(acceptedLanguages("")).toEqual([]);
    expect(acceptedLanguages(null)).toEqual([]);
  });
});

describe("pickLocale", () => {
  it("lets ?lang= win when it names a language Fettle speaks", () => {
    expect(pickLocale("it", ["en-GB"])).toBe("it");
    expect(pickLocale("IT", [])).toBe("it");
    expect(pickLocale("en", ["it"])).toBe("en");
  });

  it("ignores a ?lang= it doesn't recognise, such as items", () => {
    expect(pickLocale("items", [])).toBe("en");
    expect(pickLocale("items", ["it-IT"])).toBe("it");
  });

  it("otherwise takes the first preferred language Fettle speaks, then English", () => {
    expect(pickLocale(null, ["de-DE", "it-IT", "en"])).toBe("it");
    expect(pickLocale(null, acceptedLanguages("en;q=0, it;q=0.2"))).toBe("it");
    expect(pickLocale(null, acceptedLanguages("en;q=0.5, it;q=0.9"))).toBe("it");
    expect(pickLocale(null, ["de-DE"])).toBe("en");
    expect(pickLocale("", [])).toBe("en");
  });
});
