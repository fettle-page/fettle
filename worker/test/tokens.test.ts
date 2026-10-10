import { describe, expect, it } from "vitest";
import { hashToken, newDeleteToken, tokenMatches } from "../src/tokens.ts";

describe("delete tokens", () => {
  it("are 32 random bytes in base64url", () => {
    const t = newDeleteToken();
    expect(t).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(newDeleteToken()).not.toBe(t);
  });

  it("hash to SHA-256 hex", async () => {
    expect(await hashToken("abc")).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  });

  it("match only their own hash", async () => {
    const t = newDeleteToken();
    const h = await hashToken(t);
    expect(await tokenMatches(t, h)).toBe(true);
    expect(await tokenMatches(newDeleteToken(), h)).toBe(false);
    expect(await tokenMatches(t, "")).toBe(false);
    expect(await tokenMatches(t, h.slice(1))).toBe(false);
  });
});
