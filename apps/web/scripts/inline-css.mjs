// Runs after `astro build`. Each page has exactly one stylesheet and one early theme script; moving
// both into the page removes the render-blocking requests (the main cost in Lighthouse's slow-phone
// model). The CSP keeps 'self' and allows these exact inline blocks by SHA-256, never 'unsafe-inline'.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const dist = fileURLToPath(new URL("../dist/", import.meta.url));
const LINK = /<link rel="stylesheet" href="(\/_astro\/[^"]+\.css)">/g;

function htmlFiles(dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) return htmlFiles(full);
    return name.endsWith(".html") ? [full] : [];
  });
}

const sha = (text) => `'sha256-${createHash("sha256").update(text, "utf8").digest("base64")}'`;
const THEME_TAG = '<script src="/theme.js"></script>';
const themeJs = readFileSync(join(dist, "theme.js"), "utf8").trim();
if (themeJs.includes("</script")) throw new Error('theme.js contains "</script"');

const hashes = new Set();
for (const file of htmlFiles(dist)) {
  let html = readFileSync(file, "utf8");
  const links = [...html.matchAll(LINK)];
  if (links.length !== 1) throw new Error(`${file}: expected one stylesheet, found ${links.length}`);
  html = html.replace(LINK, (_, href) => {
    const css = readFileSync(join(dist, href), "utf8").trim();
    if (css.includes("</style")) throw new Error(`${href} contains "</style"`);
    hashes.add(sha(css));
    return `<style>${css}</style>`;
  });
  if (html.split(THEME_TAG).length !== 2) throw new Error(`${file}: expected one ${THEME_TAG}`);
  html = html.replace(THEME_TAG, `<script>${themeJs}</script>`);
  writeFileSync(file, html);
}

const headersFile = join(dist, "_headers");
const headers = readFileSync(headersFile, "utf8");
for (const directive of ["style-src 'self';", "script-src 'self';"]) {
  if (!headers.includes(directive)) throw new Error(`_headers: expected "${directive}"`);
}
writeFileSync(
  headersFile,
  headers
    .replace("style-src 'self';", `style-src 'self' ${[...hashes].sort().join(" ")};`)
    .replace("script-src 'self';", `script-src 'self' ${sha(themeJs)};`),
);
console.log(`inlined: ${hashes.size} stylesheets and the theme script; hashes added to _headers`);
