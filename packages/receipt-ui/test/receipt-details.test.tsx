import type { Facts } from "@fettle/report-schema";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Receipt } from "../src/Receipt.tsx";
import { model, nvme, ok, status, storage, startup } from "./helpers/report.ts";

const uptime = (seconds: number, locale = "en") => {
  const facts: Facts = { windows: ok({ uptime_seconds: seconds }) };
  const windows = model(facts, locale).sections.find((s) => s.id === "windows");
  return windows?.details[0]?.rows.find((r) => r.label.startsWith(locale === "en" ? "Since" : "Dall"))?.value;
};

describe("receipt details", () => {
  it("uptime under an hour, one hour, and plural hours read naturally", () => {
    expect(uptime(1200)).toBe("under an hour");
    expect(uptime(3600)).toBe("1 hour");
    expect(uptime(7200)).toBe("2 hours");
    expect(uptime(1200, "it")).toBe("meno di un'ora");
    expect(uptime(3600, "it")).toBe("1 ora");
  });

  it("each details fold is named after its section for screen readers", () => {
    const html = renderToStaticMarkup(<Receipt model={model({ storage: storage([nvme(3)]), startup: startup(3) })} />);
    expect(html).toMatch(/<summary>details<span class="sr-only">: Storage<\/span><\/summary>/);
    expect(html).toMatch(/<summary>details<span class="sr-only">: Startup<\/span><\/summary>/);
  });

  it("the receipt is labelled by its summary heading, not a repeated aria-label", () => {
    const html = renderToStaticMarkup(<Receipt model={model({ storage: storage([nvme(3)]) })} />);
    expect(html).not.toMatch(/<article[^>]*aria-label=/);
    const id = /<article[^>]*aria-labelledby="([^"]+)"/.exec(html)?.[1];
    expect(id).toBeTruthy();
    expect(html).toContain(`<h2 id="${id}">`);
  });

  it("a partial scan is stamped as partial, not checked", () => {
    const m = model({ storage: status("failed") });
    expect(m.tone).toBe("partial");
    expect(m.labels.stamp).toBe("Partial");
    expect(model({ storage: status("failed") }, "it").labels.stamp).toBe("Parziale");
    expect(model({ storage: storage([nvme(3)]) }).labels.stamp).toBe("Checked");
  });
});
