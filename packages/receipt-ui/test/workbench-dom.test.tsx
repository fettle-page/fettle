// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Workbench } from "../src/Workbench.tsx";
import { battery, model, nvme, startup, storage } from "./helpers/report.ts";

afterEach(cleanup);

const m = model({ storage: storage([nvme(3)]), battery: battery(50000, 25000), startup: startup(18) });
const tab = (name: RegExp) => screen.getByRole("tab", { name });
const selected = () => screen.getAllByRole("tab").find((t) => t.getAttribute("aria-selected") === "true");

describe("workbench interaction", () => {
  it("clicking a tab shows that section's numbers", () => {
    render(<Workbench model={m} />);
    fireEvent.click(tab(/Storage/));
    expect(screen.getByRole("tabpanel").textContent).toContain("Wear");
    fireEvent.click(tab(/Startup/));
    expect(screen.getByRole("tabpanel").textContent).toContain("on0");
  });

  it("arrow keys, Home and End move the selection and focus, wrapping", () => {
    render(<Workbench model={m} />);
    const order = screen.getAllByRole("tab").map((t) => t.textContent);
    fireEvent.click(tab(/Storage/));
    fireEvent.keyDown(selected()!, { key: "ArrowLeft" });
    expect(selected()?.textContent).toBe(order.at(-1));
    expect(document.activeElement).toBe(selected());
    fireEvent.keyDown(selected()!, { key: "ArrowRight" });
    expect(selected()?.textContent).toBe(order[0]);
    fireEvent.keyDown(selected()!, { key: "End" });
    expect(selected()?.textContent).toBe(order.at(-1));
    fireEvent.keyDown(selected()!, { key: "Home" });
    expect(selected()?.textContent).toBe(order[0]);
  });

  it("only the selected tab is in the tab order", () => {
    render(<Workbench model={m} />);
    const inOrder = screen.getAllByRole("tab").filter((t) => t.tabIndex === 0);
    expect(inOrder).toEqual([selected()]);
  });

  it("ticking a step marks it done", () => {
    render(<Workbench model={m} />);
    const box = screen.getAllByRole("checkbox")[0]!;
    fireEvent.click(box);
    expect(box.closest("li")?.hasAttribute("data-done")).toBe(true);
    fireEvent.click(box);
    expect(box.closest("li")?.hasAttribute("data-done")).toBe(false);
  });
});
