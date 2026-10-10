import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ReceiptView } from "../src/ReceiptView.tsx";
import { battery, model, nvme, storage } from "./helpers/report.ts";

describe("<ReceiptView>", () => {
  const m = model({ storage: storage([nvme(3)]), battery: battery(50000, 25000) });

  it("lays out the receipt, then the workbench, inside one view", () => {
    const out = renderToStaticMarkup(<ReceiptView model={m} />);
    expect(out).toMatch(/^<div class="fettle-view">/);
    expect(out.indexOf("fettle-receipt")).toBeGreaterThan(-1);
    expect(out.indexOf("fettle-receipt")).toBeLessThan(out.indexOf("fettle-bench"));
  });

  it("passes printing through to the receipt", () => {
    expect(renderToStaticMarkup(<ReceiptView model={m} printing />)).toContain("data-printing");
    expect(renderToStaticMarkup(<ReceiptView model={m} />)).not.toContain("data-printing");
  });
});
