// Fills the hero receipt with what this browser can see (the rows print in with CSS on their own).
// The static HTML is already a complete receipt (every value "–"), so without JS nothing is missing.
import type { Locale } from "../i18n/paths.ts";
import { browserFacts, estimateZoom, type BrowserEnv } from "./browser-facts.ts";

type UAData = { platform?: string; getHighEntropyValues?: (hints: string[]) => Promise<{ platformVersion?: string }> };

async function readEnv(): Promise<BrowserEnv> {
  const nav = navigator as Navigator & { deviceMemory?: number; userAgentData?: UAData };
  const env: BrowserEnv = {
    screen: { width: screen.width, height: screen.height },
    pixelRatio: window.devicePixelRatio || undefined,
    zoom: estimateZoom(window.outerWidth, window.innerWidth, window.devicePixelRatio || 1),
    hardwareConcurrency: nav.hardwareConcurrency || undefined,
    deviceMemory: nav.deviceMemory,
    userAgent: nav.userAgent,
  };
  const ua = nav.userAgentData;
  if (ua?.getHighEntropyValues) {
    try {
      env.uaPlatform = ua.platform;
      env.platformVersion = (await ua.getHighEntropyValues(["platformVersion"])).platformVersion;
    } catch {
      // Hints refused: fall back to the plain user agent.
    }
  }
  return env;
}

async function fill(receipt: HTMLElement) {
  const locale = (document.documentElement.lang === "it" ? "it" : "en") as Locale;
  const missing = receipt.dataset.missing ?? "–";
  for (const fact of browserFacts(await readEnv(), locale)) {
    const cell = receipt.querySelector<HTMLElement>(`[data-fact="${fact.key}"]`);
    if (!cell) continue;
    // Disk and battery are printed as "can't see" with a ?? tag already; keep that markup.
    if (fact.value === null && cell.classList.contains("na")) continue;
    cell.textContent = fact.value ?? missing;
    cell.classList.toggle("na", fact.value === null);
  }
  const time = receipt.querySelector<HTMLElement>("[data-time]");
  if (time) time.textContent = new Date().toLocaleTimeString(locale === "it" ? "it-IT" : "en-GB", { hour: "2-digit", minute: "2-digit" });
}

const receipt = document.querySelector<HTMLElement>("[data-live-receipt]");
// The rows print in with CSS from first paint; this only fills in the values.
if (receipt) void fill(receipt);
