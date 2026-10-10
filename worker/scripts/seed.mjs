// Uploads a report to a running share Worker and prints its links: the tired-laptop test report,
// or any saved report (`cargo run -p fettle-collect -- --out target/scan.json` makes one of this PC).
// Usage: node scripts/seed.mjs [origin] [--password <p>] [--file <report.json>]
//        (origin defaults to http://localhost:8787)
import { readFile } from "node:fs/promises";

const args = process.argv.slice(2);
const option = (name) => {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
};
const password = option("--password");
const file = option("--file");
const values = new Set([password, file]);
const origin = args.find((a) => !a.startsWith("--") && !values.has(a)) ?? "http://localhost:8787";

const source = file ?? new URL("../test/fixtures/tired.json", import.meta.url);
const report = JSON.parse(await readFile(source, "utf8"));
const res = await fetch(`${origin}/api/reports`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ expires: "30d", report, ...(password && { password }) }),
});
const body = await res.json();
if (res.status !== 201) {
  console.error(res.status, body);
  process.exit(1);
}
console.log(`url:       ${body.url}`);
console.log(`deleteUrl: ${body.deleteUrl}`);
console.log(`locked:    ${body.locked}`);
