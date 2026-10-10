import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

export const DIST = fileURLToPath(new URL("../dist/", import.meta.url));

/** Every built HTML page, as [path relative to dist with forward slashes, html]. */
export function pages(): [string, string][] {
  if (!existsSync(DIST)) throw new Error("dist/ is missing: run astro build first");
  const out: [string, string][] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const full = join(dir, name);
      if (statSync(full).isDirectory()) walk(full);
      else if (name.endsWith(".html")) out.push([relative(DIST, full).replaceAll("\\", "/"), readFileSync(full, "utf8")]);
    }
  };
  walk(DIST);
  return out.sort(([a], [b]) => a.localeCompare(b));
}

export const page = (file: string) => readFileSync(join(DIST, file), "utf8");
const ENTITIES: Record<string, string> = { "&amp;": "&", "&lt;": "<", "&gt;": ">", "&quot;": '"', "&#39;": "'", "&nbsp;": " " };

/** The visible text of some HTML, with tags dropped and common entities decoded. */
export const text = (html: string) =>
  html
    .replace(/<script[\s\S]*?<\/script>/g, "")
    .replace(/<[^>]+>/g, " ")
    .replace(/&(amp|lt|gt|quot|#39|nbsp);/g, (e) => ENTITIES[e]!)
    .replace(/\s+/g, " ");
