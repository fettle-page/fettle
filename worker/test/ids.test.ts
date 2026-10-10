import { describe, expect, it } from "vitest";
import { isValidId, keyFor, newId, ttlOfId, type Ttl } from "../src/ids.ts";

const SHAPE = /^[wmy][023456789abcdefghjkmnpqrstuvwxyz]{16}$/;
const TTLS: Ttl[] = ["7d", "30d", "1y"];

describe("report ids", () => {
  it("start with the expiry letter, then 16 random characters", () => {
    expect(newId("7d")).toMatch(SHAPE);
    expect(newId("7d")[0]).toBe("w");
    expect(newId("30d")[0]).toBe("m");
    expect(newId("1y")[0]).toBe("y");
  });

  it("carry their expiry and R2 key", () => {
    for (const t of TTLS) {
      const id = newId(t);
      expect(ttlOfId(id)).toBe(t);
      expect(keyFor(id)).toBe(`${t}/${id}`);
    }
  });

  it("don't repeat", () => {
    const ids = new Set(Array.from({ length: 2000 }, () => newId("30d")));
    expect(ids.size).toBe(2000);
  });

  it("validate their own shape only", () => {
    expect(isValidId(newId("30d"))).toBe(true);
    for (const bad of ["", "m3kx9f2q7pa8hd4w", "m3kx9f2q7pa8hd4wzz", "x3kx9f2q7pa8hd4wz", "M3KX9F2Q7PA8HD4WZ", "m3kx9f2q7pa8hd4w1", "../3kx9f2q7pa8hd4"]) {
      expect(isValidId(bad), bad).toBe(false);
    }
  });
});
