import { describe, expect, it } from "vitest";
import { hashPassword, passwordMatches } from "../src/passwords.ts";

describe("share passwords", () => {
  it("hash with a random 16-byte salt", async () => {
    const a = await hashPassword("hunter22");
    expect(a.salt).toMatch(/^[A-Za-z0-9_-]{22}$/);
    expect(a.hash).toMatch(/^[0-9a-f]{64}$/);
    expect((await hashPassword("hunter22")).salt).not.toBe(a.salt);
  });

  it("are deterministic for a given salt", async () => {
    const salt = new Uint8Array(16);
    expect((await hashPassword("hunter22", salt)).hash).toBe((await hashPassword("hunter22", salt)).hash);
  });

  it("match only the right password", async () => {
    const { salt, hash } = await hashPassword("hunter22");
    expect(await passwordMatches("hunter22", salt, hash)).toBe(true);
    expect(await passwordMatches("hunter23", salt, hash)).toBe(false);
    expect(await passwordMatches("hunter22", salt, "zz")).toBe(false);
    expect(await passwordMatches("hunter22", "!!", hash)).toBe(false);
  });
});
