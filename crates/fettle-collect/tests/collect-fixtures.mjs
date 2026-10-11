// Writes what the JS converter makes of each tracked spike dump, for the Rust collectors' parity
// test (crates/fettle-collect/tests/parity.rs). Run: node crates/fettle-collect/tests/collect-fixtures.mjs
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseReport } from "@fettle/report-schema";
import { convertDump } from "../../fettle-receipt/parity/from-spike.mjs";

const root = fileURLToPath(new URL("../../../", import.meta.url));
export const DUMPS = join(root, "spike/dumps");
/** Made-up scans with odd values the real dumps don't have, to keep the two mappings alike there too. */
export const TWIN_CASES = join(root, "crates/fettle-collect/tests/twin-cases");
export const OUT = join(root, "crates/fettle-collect/tests/fixtures");

/** File name → contents: the parsed report's facts, 2-space JSON with LF. */
export function generate() {
  const files = {};
  const dumps = [DUMPS, TWIN_CASES].flatMap((dir) => readdirSync(dir).filter((f) => f.endsWith(".json")).map((name) => [dir, name]));
  for (const [dir, name] of dumps.sort(([, a], [, b]) => a.localeCompare(b))) {
    const parsed = parseReport(convertDump(JSON.parse(readFileSync(join(dir, name), "utf8"))));
    if (!parsed.ok) throw new Error(`${name}: ${parsed.error}`);
    files[name.replace(/\.json$/, ".facts.json")] = `${JSON.stringify(parsed.report.facts, null, 2)}\n`;
  }
  return files;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  mkdirSync(OUT, { recursive: true });
  for (const [name, text] of Object.entries(generate())) writeFileSync(join(OUT, name), text);
  console.log(`wrote ${OUT}`);
}
