import { describe, expect, it } from "vitest";
import { call } from "./helpers.ts";

describe("the website through the Worker", () => {
  it("serves the landing page", async () => {
    const res = await call("/");
    expect(res.status).toBe(200);
    expect(await res.text()).toContain("fine fettle");
  });

  it("shows the site's own 404 page for unknown paths", async () => {
    const res = await call("/nope");
    expect(res.status).toBe(404);
    expect(await res.text()).toContain("This page isn");
  });
});
