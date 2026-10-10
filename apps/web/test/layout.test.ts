import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const site = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** The body of the first top-level rule whose selector list is exactly `selector`. */
const rule = (selector: string) => new RegExp(`(?:^|\\})\\s*${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`, "m").exec(site)?.[1] ?? "";

describe("one content width", () => {
  it("defines one content width, wider on big screens", () => {
    expect(rule(":root")).toMatch(/--wrap-max:\s*1320px/);
    expect(site).toMatch(/@media \(min-width: 1800px\)\s*\{[^@]*--wrap-max:\s*1560px/);
  });

  it("the nav, the page and the footer share it", () => {
    for (const selector of [".nav", ".wrap", ".foot-slip"]) expect(rule(selector), selector).toContain("var(--wrap-max)");
    expect(site).not.toMatch(/max-width:\s*1320px/);
  });
});
