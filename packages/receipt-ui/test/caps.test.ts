import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { uiText as ui } from "@fettle/brain";

const css = (file: string) => readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8");
const rule = (source: string, selector: string) => {
  const at = source.indexOf(`${selector} {`);
  if (at < 0) throw new Error(`no rule for ${selector}`);
  return source.slice(at, source.indexOf("}", at));
};

// Screen readers can spell words typed in capitals letter by letter, so the text is written in
// sentence case and only the CSS shows it in capitals.
describe("section titles", () => {
  it("are written in sentence case", () => {
    expect(ui("section.storage", "en")).toBe("Storage");
    expect(ui("section.system", "en")).toBe("This PC");
    expect(ui("section.storage", "it")).toBe("Spazio di archiviazione");
    expect(ui("section.system", "it")).toBe("Questo PC");
  });

  it("still look like capitals on the receipt and the workbench tabs", () => {
    expect(rule(css("receipt.css"), ".fettle-receipt .r-section h3")).toContain("text-transform: uppercase");
    expect(rule(css("view.css"), '.fettle-bench [role="tab"]')).toContain("text-transform: uppercase");
  });
});
