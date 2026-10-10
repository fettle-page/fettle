// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ReceiptView } from "../src/ReceiptView.tsx";
import { battery, model, report, startup } from "./helpers/report.ts";
import { brain } from "@fettle/brain/node";

afterEach(cleanup);

const later = () => {
  const r = { ...report({ startup: startup(18), battery: battery(50000, 25000) }), collected_at: "2026-10-08T10:00:00+02:00" };
  return brain.receiptFor(r, "en", { offsetMinutes: 120 }).model;
};

describe("ReceiptView across reports", () => {
  const first = model({ battery: battery(50000, 25000), startup: startup(18) });

  it("a new report clears the ticks", () => {
    const { rerender } = render(<ReceiptView model={first} />);
    fireEvent.click(screen.getAllByRole("checkbox")[0]!);
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(true);
    rerender(<ReceiptView model={later()} />);
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(false);
  });

  it("switching language keeps the ticks", () => {
    const { rerender } = render(<ReceiptView model={first} />);
    fireEvent.click(screen.getAllByRole("checkbox")[0]!);
    rerender(<ReceiptView model={model({ battery: battery(50000, 25000), startup: startup(18) }, "it")} />);
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(true);
  });
});
