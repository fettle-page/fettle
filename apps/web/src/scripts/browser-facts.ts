// What a web page can honestly learn about a PC: very little. The hero receipt prints exactly
// this, then says the rest needs the app. Pure, so it's tested without a browser.
import type { Locale } from "../i18n/paths.ts";

export type FactKey = "screen" | "threads" | "memory" | "system" | "disk" | "battery";

export type BrowserEnv = {
  /** In CSS pixels, which Windows display scaling and page zoom shrink. */
  screen: { width: number; height: number };
  /** devicePixelRatio: turns CSS pixels back into the screen's real pixels. */
  pixelRatio?: number;
  /** The page zoom (1 = 100%). Chrome and Edge fold it into pixelRatio but not into screen. */
  zoom?: number;
  hardwareConcurrency?: number;
  /** Chromium only, and capped at 8. */
  deviceMemory?: number;
  /** From User-Agent Client Hints (Chromium only): "Windows", "macOS"… */
  uaPlatform?: string;
  /** Client hints platformVersion: 13+ means Windows 11; the plain UA says "Windows NT 10.0" for both. */
  platformVersion?: string;
  userAgent: string;
};

export type Fact = { key: FactKey; value: string | null };

const SYSTEM_TEXT: Record<Locale, { either: string; notWindows: string }> = {
  en: { either: "Windows 10 or 11", notWindows: "Not Windows" },
  it: { either: "Windows 10 o 11", notWindows: "Non Windows" },
};

function system(env: BrowserEnv, locale: Locale): string {
  if (env.uaPlatform === "Windows" && env.platformVersion) {
    return Number.parseInt(env.platformVersion, 10) >= 13 ? "Windows 11" : "Windows 10";
  }
  return /Windows/.test(env.userAgent) ? SYSTEM_TEXT[locale].either : SYSTEM_TEXT[locale].notWindows;
}

/** The screen's real pixels. The browser has already rounded the scaled size, so a scaled one
 *  snaps to the nearest even number (real resolutions are even): 785 × 1.375 is 1080, not 1079. */
function realPixels(css: number, ratio: number | undefined, zoom: number | undefined): number {
  const scale = (ratio || 1) / (zoom || 1); // Windows display scaling alone
  if (scale === 1) return css;
  return Math.round((css * scale) / 2) * 2;
}

/** Chrome's zoom steps (its thirds are exact thirds). */
const ZOOM_STEPS = [0.25, 1 / 3, 0.5, 2 / 3, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5];
/** Display scaling a screen can have: Windows' 100–400% steps, a Mac's 2x. */
const SCALE_STEPS = [1, 1.25, 1.5, 1.75, 2, 2.25, 2.5, 3, 3.5, 4];
const isScaleStep = (scale: number) => SCALE_STEPS.some((s) => Math.abs(scale - s) / s <= 0.01);

/**
 * The page zoom, from the window's width over the page's: in Chrome and Edge only the page width
 * changes with zoom (in Firefox both do, so the ratio stays 1 and its numbers are already right).
 * A side panel or docked DevTools narrows the page too, so a zoom only counts if it leaves a real
 * display scaling (pixelRatio / zoom): at 100% scaling a "125% zoom" from a side panel would leave
 * 80%, which no screen uses. Safari doesn't fold zoom into pixelRatio, so it fails the same check.
 */
export function estimateZoom(outerWidth: number, innerWidth: number, pixelRatio = 1): number {
  if (!(outerWidth > 0 && innerWidth > 0)) return 1;
  const ratio = outerWidth / innerWidth;
  const zoom = ZOOM_STEPS.filter((z) => isScaleStep(pixelRatio / z)).reduce((best, z) => (Math.abs(z - ratio) < Math.abs(best - ratio) ? z : best), 1);
  return Math.abs(zoom - ratio) / zoom <= 0.06 ? zoom : 1;
}

export function browserFacts(env: BrowserEnv, locale: Locale): Fact[] {
  const memory = env.deviceMemory ? (env.deviceMemory >= 8 ? "8 GB+" : `~${env.deviceMemory} GB`) : null;
  return [
    { key: "screen", value: `${realPixels(env.screen.width, env.pixelRatio, env.zoom)} × ${realPixels(env.screen.height, env.pixelRatio, env.zoom)}` },
    { key: "threads", value: env.hardwareConcurrency ? String(env.hardwareConcurrency) : null },
    { key: "memory", value: memory },
    { key: "system", value: system(env, locale) },
    { key: "disk", value: null },
    { key: "battery", value: null },
  ];
}
