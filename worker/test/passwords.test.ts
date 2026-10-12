import { describe, expect, it } from "vitest";
import { hashPassword, hashPasswordV1, passwordMatches } from "../src/passwords.ts";

const PEPPER = "test-password-pepper-not-a-real-secret";

describe("share passwords", () => {
  it("hash with a random 16-byte salt, as version 2", async () => {
    const a = await hashPassword("hunter22", PEPPER);
    expect(a.salt).toMatch(/^[A-Za-z0-9_-]{22}$/);
    expect(a.hash).toMatch(/^[0-9a-f]{64}$/);
    expect(a.version).toBe("2");
    expect((await hashPassword("hunter22", PEPPER)).salt).not.toBe(a.salt);
  });

  it("are deterministic for a given salt and secret", async () => {
    const salt = new Uint8Array(16);
    expect((await hashPassword("hunter22", PEPPER, salt)).hash).toBe((await hashPassword("hunter22", PEPPER, salt)).hash);
  });

  // With the server's secret mixed in, a stolen hash can't be guessed at without the secret too.
  it("depend on the server's secret, unlike the old scheme", async () => {
    const salt = new Uint8Array(16);
    const peppered = (await hashPassword("hunter22", PEPPER, salt)).hash;
    expect((await hashPassword("hunter22", "another-secret", salt)).hash).not.toBe(peppered);
    expect((await hashPasswordV1("hunter22", salt)).hash).not.toBe(peppered);
  });

  it("match only the right password", async () => {
    const stored = await hashPassword("hunter22", PEPPER);
    expect(await passwordMatches("hunter22", stored, PEPPER)).toBe(true);
    expect(await passwordMatches("hunter23", stored, PEPPER)).toBe(false);
    expect(await passwordMatches("hunter22", stored, "another-secret")).toBe(false);
    expect(await passwordMatches("hunter22", { ...stored, hash: "zz" }, PEPPER)).toBe(false);
    expect(await passwordMatches("hunter22", { ...stored, salt: "!!" }, PEPPER)).toBe(false);
  });

  it("still check a hash stored the old way (no version), without the secret", async () => {
    const old = await hashPasswordV1("hunter22");
    expect(await passwordMatches("hunter22", old, PEPPER)).toBe(true);
    expect(await passwordMatches("hunter22", old, undefined)).toBe(true);
    expect(await passwordMatches("hunter23", old, PEPPER)).toBe(false);
  });

  it("never check a version 2 hash without the secret, or a version they don't know", async () => {
    const stored = await hashPassword("hunter22", PEPPER);
    expect(await passwordMatches("hunter22", stored, undefined)).toBe(false);
    expect(await passwordMatches("hunter22", { ...stored, version: "3" }, PEPPER)).toBe(false);
  });
});
