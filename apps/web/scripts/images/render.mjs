// Renders the preview cards (public/og/*.png) and the touch icon (public/apple-touch-icon.png).
// Run by hand when the brand changes; the outputs are committed. Needs playwright-cli with Chromium:
//   playwright-cli open --browser=chromium
//   node apps/web/scripts/images/render.mjs
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../../", import.meta.url)).replaceAll("\\", "/").replace(/\/$/, "");
mkdirSync(join(root, "apps/web/public/og"), { recursive: true });
const code = readFileSync(new URL("./render.js", import.meta.url), "utf8").replaceAll("__ROOT__", root);
const file = join(tmpdir(), "fettle-render-images.js");
writeFileSync(file, code);
const out = execFileSync("playwright-cli", ["--raw", "run-code", `--filename=${file}`], { encoding: "utf8", shell: true });
console.log(out.trim());
