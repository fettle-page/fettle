import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ModelSection, ReceiptModel } from "@fettle/brain";
import { Workbench, worstLevel } from "../src/Workbench.tsx";
import { battery, memory, model, nvme, startup, status, storage } from "./helpers/report.ts";

const html = (m: ReceiptModel) => renderToStaticMarkup(<Workbench model={m} />);
const tabs = (s: string) => [...s.matchAll(/<button[^>]*role="tab"[^>]*>([\s\S]*?)<\/button>/g)].map((m) => m[0]);
const section = (levels: ModelSection["lines"][number]["level"][]): ModelSection => ({
  id: "storage",
  title: "Storage",
  lines: levels.map((level, i) => ({ id: `l${i}`, level, tag: "", levelText: "", headline: "" })),
  details: [],
});

describe("worstLevel", () => {
  it("act soon beats everything", () => expect(worstLevel(section(["fine", "act_soon", "keep_an_eye"]))).toBe("act_soon"));
  it("keep an eye beats couldn't check", () => expect(worstLevel(section(["couldnt_check", "keep_an_eye"]))).toBe("keep_an_eye"));
  it("couldn't check beats fine", () => expect(worstLevel(section(["fine", "couldnt_check"]))).toBe("couldnt_check"));
  it("fine beats a note", () => expect(worstLevel(section(["info", "fine"]))).toBe("fine"));
});

describe("<Workbench>", () => {
  const busy = model({ storage: storage([nvme(3)]), battery: battery(50000, 25000), startup: startup(18), memory: memory({ modules: [{ capacity_bytes: 16 * 2 ** 30 }] }) });

  it("lists every next step as a checkbox", () => {
    const out = html(busy);
    expect((out.match(/type="checkbox"/g) ?? []).length).toBe(busy.actions.length);
    for (const a of busy.actions) expect(out).toContain(a.replaceAll("'", "&#x27;"));
  });

  it("has one tab per section with numbers, in receipt order", () => {
    const withDetails = busy.sections.filter((s) => s.details.length > 0).map((s) => s.title);
    const titleOf = (t: string) => t.replace(/<span class="sr-only">[^<]*<\/span>/, "").replace(/<[^>]+>/g, "").trim();
    expect(tabs(html(busy)).map(titleOf)).toEqual(withDetails);
  });

  it("opens on the section that needs action first", () => {
    const selected = tabs(html(busy)).find((t) => t.includes('aria-selected="true"')) ?? "";
    expect(selected).toContain("Battery");
    expect(selected).toContain('data-level="act_soon"');
    expect(selected).toContain('<span class="sr-only">');
  });

  it("falls back to the first section when nothing needs attention", () => {
    const calm = model({ storage: storage([nvme(3)]), startup: startup(2) });
    expect(tabs(html(calm)).find((t) => t.includes('aria-selected="true"'))).toContain("Storage");
  });

  it("says so when there is nothing to do", () => {
    expect(html(model({ storage: storage([nvme(3)]) }))).toContain("Nothing to do right now.");
  });

  it("has no numbers block when no section has numbers", () => {
    const base = model({ battery: battery(50000, 25000) });
    const out = html({ ...base, sections: base.sections.map((s) => ({ ...s, details: [] })) });
    expect(out).toContain("Replace the battery");
    expect(out).not.toContain('role="tablist"');
    expect(out).not.toContain("The numbers");
  });

  it("renders nothing when there is nothing to show", () => {
    expect(html(model({ storage: status("failed") }))).toBe("");
  });

  it("speaks Italian", () => {
    const out = html(model({ startup: startup(18) }, "it"));
    expect(out).toContain("Da fare");
    expect(out).toContain("I numeri");
  });
});
