import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../src/receipt.css", import.meta.url), "utf8");
const rule = (selector: string) => {
  const at = css.indexOf(`${selector} {`);
  if (at < 0) throw new Error(`no rule for ${selector}`);
  return css.slice(at, css.indexOf("}", at));
};

// A long value (a board name like "MSI MAG X870E TOMAHAWK WIFI (MS-7E59)") used to squeeze its label
// until it broke mid-word: "Sched / a / madre". The label stays whole; the value wraps.
describe("receipt key/value rows", () => {
  it("never break a label", () => {
    expect(rule(".fettle-receipt .kv dt > span")).toContain("white-space: nowrap");
  });

  it("keep the label's box at least as wide as the label", () => {
    expect(rule(".fettle-receipt .kv dt")).not.toContain("min-width: 0");
  });

  it("let the value give way and wrap", () => {
    expect(rule(".fettle-receipt .kv dd")).toContain("flex: 0 1 auto");
  });
});
