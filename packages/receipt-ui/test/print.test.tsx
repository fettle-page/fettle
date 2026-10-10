// @vitest-environment jsdom
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Receipt } from "../src/Receipt.tsx";
import { model, startup, storage, nvme } from "./helpers/report.ts";

afterEach(cleanup);

const folds = (root: HTMLElement) => [...root.querySelectorAll("details")];
const fire = (type: "beforeprint" | "afterprint") => act(() => void window.dispatchEvent(new Event(type)));

describe("printing unfolds the details", () => {
  const m = model({ storage: storage([nvme(3)]), startup: startup(3) });

  it("opens every fold before print and closes them after", () => {
    const { container } = render(<Receipt model={m} />);
    expect(folds(container)).toHaveLength(2);
    expect(folds(container).every((d) => !d.open)).toBe(true);
    fire("beforeprint");
    expect(folds(container).every((d) => d.open)).toBe(true);
    fire("afterprint");
    expect(folds(container).every((d) => !d.open)).toBe(true);
  });

  it("leaves a fold the reader opened open after printing", () => {
    const { container } = render(<Receipt model={m} />);
    const [first, second] = folds(container);
    first!.open = true;
    fire("beforeprint");
    fire("afterprint");
    expect(first!.open).toBe(true);
    expect(second!.open).toBe(false);
  });
});
