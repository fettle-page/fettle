import { describe, expect, it } from "vitest";
import { browserFacts, estimateZoom, type BrowserEnv } from "../src/scripts/browser-facts.ts";

const base: BrowserEnv = { screen: { width: 1920, height: 1080 }, userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)" };
const value = (env: BrowserEnv, key: string, locale: "en" | "it" = "en") => browserFacts(env, locale).find((f) => f.key === key)?.value;

describe("browserFacts", () => {
  it("lists the same six lines in order", () => {
    expect(browserFacts(base, "en").map((f) => f.key)).toEqual(["screen", "threads", "memory", "system", "disk", "battery"]);
  });

  it("reads Chrome on Windows 11", () => {
    const env = { ...base, hardwareConcurrency: 16, deviceMemory: 8, uaPlatform: "Windows", platformVersion: "15.0.0" };
    expect(value(env, "screen")).toBe("1920 × 1080");
    expect(value(env, "threads")).toBe("16");
    expect(value(env, "memory")).toBe("8 GB+");
    expect(value(env, "system")).toBe("Windows 11");
  });

  it("shows the screen's real pixels, not the scaled ones (4K at 150%)", () => {
    expect(value({ ...base, screen: { width: 2560, height: 1440 }, pixelRatio: 1.5 }, "screen")).toBe("3840 × 2160");
  });

  it("rounds an uneven scale (1080p at 125% with a 110% page zoom)", () => {
    expect(value({ ...base, screen: { width: 1396, height: 785 }, pixelRatio: 1.375 }, "screen")).toBe("1920 × 1080");
  });

  // Chrome and Edge: page zoom changes devicePixelRatio but not screen.width.
  it("a zoomed page in Chrome still shows the real screen (1080p at 100%, zoomed to 110%)", () => {
    expect(value({ ...base, screen: { width: 1920, height: 1080 }, pixelRatio: 1.1, zoom: 1.1 }, "screen")).toBe("1920 × 1080");
  });

  it("a zoomed page in Chrome still shows the real screen (4K at 150%, zoomed to 125%)", () => {
    expect(value({ ...base, screen: { width: 2560, height: 1440 }, pixelRatio: 1.875, zoom: 1.25 }, "screen")).toBe("3840 × 2160");
  });

  it("reads Chrome's page zoom from the window and page widths", () => {
    expect(estimateZoom(1936, 1920, 1)).toBe(1); // a maximised window's invisible borders
    expect(estimateZoom(1936, 1760, 1.1)).toBe(1.1);
    expect(estimateZoom(1280, 853, 1.5)).toBe(1.5);
    expect(estimateZoom(1280, 1707, 0.75)).toBe(0.75);
    expect(estimateZoom(2576, 2048, 1.875)).toBe(1.25); // 4K at 150%, zoomed to 125%
  });

  it("uses Chrome's exact third steps, so a 33% zoom isn't 1% off", () => {
    expect(estimateZoom(1936, 5760, 1 / 3)).toBe(1 / 3);
    expect(estimateZoom(1936, 2880, 2 / 3)).toBe(2 / 3);
  });

  it("doesn't take a side panel or docked DevTools for zoom", () => {
    expect(estimateZoom(1920, 1620, 1)).toBe(1);
    expect(estimateZoom(1936, 1544, 1)).toBe(1); // Chrome's side panel: would read as 125%
    expect(estimateZoom(1936, 1280, 1)).toBe(1); // DevTools docked at the side: would read as 150%
    expect(estimateZoom(0, 1920, 1)).toBe(1);
  });

  it("still sees zoom with Edge's narrow sidebar open (1080p, zoomed to 110%)", () => {
    expect(estimateZoom(1936, 1702, 1.1)).toBe(1.1);
  });

  it("leaves Safari alone: its zoom doesn't change the pixel ratio", () => {
    expect(estimateZoom(1440, 1152, 2)).toBe(1); // a MacBook at 2x, page zoomed to 125%
  });

  it("reads Chrome on Windows 10", () => {
    expect(value({ ...base, uaPlatform: "Windows", platformVersion: "10.0.0" }, "system")).toBe("Windows 10");
  });

  it("says what Firefox can't see, in the app's voice", () => {
    const env = { ...base, hardwareConcurrency: 8, userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0" };
    expect(value(env, "memory")).toBeNull();
    expect(value(env, "system")).toBe("Windows 10 or 11");
    expect(value(env, "system", "it")).toBe("Windows 10 o 11");
  });

  it("knows a Mac isn't Windows", () => {
    const env = { ...base, userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_6) AppleWebKit/605.1.15 Safari/605.1.15" };
    expect(value(env, "system")).toBe("Not Windows");
    expect(value(env, "system", "it")).toBe("Non Windows");
  });

  it("rounds small memory and never sees disk or battery health", () => {
    const env = { ...base, deviceMemory: 4 };
    expect(value(env, "memory")).toBe("~4 GB");
    expect(value(env, "disk")).toBeNull();
    expect(value(env, "battery")).toBeNull();
  });
});
