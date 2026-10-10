import { describe, expect, it } from "vitest";
import { answerId } from "../src/scripts/faq-open.ts";
import { page } from "./dist-helpers.ts";

// A link like /faq#temps opens that answer: the answers are closed <details>.
describe("opening the answer a link points at", () => {
  it("reads the answer's id from the link", () => {
    expect(answerId("#temps")).toBe("temps");
    expect(answerId("#windows10")).toBe("windows10");
  });

  it("ignores an empty or odd hash", () => {
    expect(answerId("")).toBeNull();
    expect(answerId("#")).toBeNull();
    expect(answerId("#a b")).toBeNull();
  });

  it("the FAQ pages load the script, and the landing pages don't", () => {
    for (const file of ["faq.html", "it/faq.html"]) expect(page(file), file).toMatch(/<script type="module" src="\/_astro\/Faq\.astro[^"]*\.js"/);
    for (const file of ["index.html", "it.html"]) expect(page(file), file).not.toContain("/_astro/Faq.astro");
  });
});
