// The guides: plain answers to common Windows questions, each in its own file under guides/.
// To add one: write guides/<slug>.ts (copy windows-10-updates.ts for the shape), import it here and
// add it to `guides` in the order the index should list it. The pages and the sitemap follow.
import type { Locale } from "../i18n/paths.ts";
import windows10Updates from "./guides/windows-10-updates.ts";
import diskHealth from "./guides/disk-health.ts";
import batteryHealth from "./guides/battery-health.ts";
import windows11Ready from "./guides/windows-11-ready.ts";

/** A part of a guide: a heading, paragraphs, then (optionally) numbered steps. */
export type GuideSection = { h: string; p: string[]; steps?: string[] };

export type GuideCopy = {
  /** The browser tab title, "… — Fettle". */
  title: string;
  /** The meta description, for search results. */
  description: string;
  /** One line under the guide's name on the guides page. */
  summary: string;
  kicker: string;
  h1: string;
  intro: string;
  /** The last section says how Fettle shows the same thing. */
  sections: GuideSection[];
};

export type Guide = { slug: string } & Record<Locale, GuideCopy>;

export const guides: Guide[] = [windows10Updates, diskHealth, batteryHealth, windows11Ready];

/** The guides page itself, and the links at the end of each guide. */
export const guidesPage: Record<Locale, { title: string; description: string; kicker: string; h1: string; intro: string; download: string; back: string }> = {
  en: {
    title: "Guides — Fettle",
    description: "Plain answers to common questions about a Windows PC, with the steps to check it yourself using Windows' own tools.",
    kicker: "Guides",
    h1: "Check it yourself.",
    intro: "Short guides to common questions about a Windows PC: what it means, how to check with Windows' own tools, and what to do next.",
    download: "Download Fettle",
    back: "All guides",
  },
  it: {
    title: "Guide — Fettle",
    description: "Risposte semplici alle domande più comuni su un PC Windows, con i passaggi per controllare da solo con gli strumenti di Windows.",
    kicker: "Guide",
    h1: "Controlla da solo.",
    intro: "Brevi guide alle domande più comuni su un PC Windows: cosa significa, come controllare con gli strumenti di Windows e cosa fare dopo.",
    download: "Scarica Fettle",
    back: "Tutte le guide",
  },
};
