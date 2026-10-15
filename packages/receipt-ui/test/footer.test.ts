import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../src/receipt.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

// Measured in Playwright: the longest stamp ("Controllato" with its date) is about 15.3em of the footer's
// font, so on a phone-width receipt a 46% barcode ran under it. The barcode gives way instead.
describe("the receipt's footer", () => {
  it("keeps the barcode clear of the stamp on a narrow receipt", () => {
    const barcode = /\.fettle-receipt \.barcode\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(barcode).toMatch(/width:\s*min\(46%,\s*calc\(100% - 16\.5em\)\)/);
  });
});
