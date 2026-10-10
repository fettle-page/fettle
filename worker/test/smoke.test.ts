import { env, exports } from "cloudflare:workers";
import { describe, expect, it } from "vitest";

describe("worker scaffold", () => {
  it("unknown API paths answer 404 as JSON", async () => {
    const res = await exports.default.fetch(new Request("http://example.com/api/nope"));
    expect(res.status).toBe(404);
    expect(res.headers.get("content-type")).toBe("application/json; charset=utf-8");
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    expect(await res.json()).toEqual({ error: "not_found" });
  });

  it("the R2 and rate-limit bindings exist locally", () => {
    expect(env.REPORTS).toBeDefined();
    expect(typeof env.UPLOAD_LIMITER.limit).toBe("function");
    expect(typeof env.UNLOCK_LIMITER.limit).toBe("function");
  });
});
