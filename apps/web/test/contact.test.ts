import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { release } from "../src/release.ts";
import { DIST } from "./dist-helpers.ts";

describe("contact", () => {
  it("is hello@fettle.page, the public mailbox (Fastmail)", () => {
    expect(release.contact).toBe("hello@fettle.page");
  });

  it("links the source at github.com/fettle-page/fettle", () => {
    expect(release.repo).toBe("https://github.com/fettle-page/fettle");
  });
});

// RFC 9116: where security researchers look for a way to report a problem.
describe("/.well-known/security.txt", () => {
  const file = join(DIST, ".well-known", "security.txt");
  const fields = () =>
    Object.fromEntries(
      readFileSync(file, "utf8")
        .split(/\r?\n/)
        .filter((l) => l && !l.startsWith("#"))
        .map((l) => [l.slice(0, l.indexOf(":")), l.slice(l.indexOf(":") + 1).trim()]),
    );

  it("is in the built site", () => {
    expect(existsSync(file)).toBe(true);
  });

  it("points at security@fettle.page, in English or Italian, at its canonical address", () => {
    const f = fields();
    expect(f.Contact).toBe("mailto:security@fettle.page");
    expect(f["Preferred-Languages"]).toBe("en, it");
    expect(f.Canonical).toBe("https://fettle.page/.well-known/security.txt");
  });

  it("hasn't expired, and expires within a year as the RFC asks (renew it before then)", () => {
    const expires = new Date(fields().Expires!);
    const days = (expires.getTime() - Date.now()) / 86_400_000;
    expect(fields().Expires).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/);
    expect(days).toBeGreaterThan(0);
    expect(days).toBeLessThanOrEqual(366);
  });
});
