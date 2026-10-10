import { describe, expect, it } from "vitest";
import { alternates, localeFromPath, localePath } from "../src/i18n/paths.ts";

describe("locale paths", () => {
  it("keeps English at the root and Italian under /it", () => {
    expect(localePath("en", "/faq")).toBe("/faq");
    expect(localePath("it", "/faq")).toBe("/it/faq");
    expect(localePath("en", "/")).toBe("/");
    expect(localePath("it", "/")).toBe("/it");
  });

  it("gives both language versions of a page", () => {
    expect(alternates("/privacy")).toEqual({ en: "/privacy", it: "/it/privacy" });
    expect(alternates("/")).toEqual({ en: "/", it: "/it" });
  });

  it("reads the language from a path", () => {
    expect(localeFromPath("/it/faq")).toBe("it");
    expect(localeFromPath("/it")).toBe("it");
    expect(localeFromPath("/faq")).toBe("en");
    expect(localeFromPath("/italy")).toBe("en");
  });
});
