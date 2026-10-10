// Writes THIRD-PARTY-NOTICES.txt for the downloads: the licence of every Rust crate built into
// fettle.exe (from `cargo metadata`), the C and C++ libraries that come inside Skia's prebuilt
// renderer, the Rust standard library and the two fonts. Most open-source licences ask that their
// notice travels with the program, and the OFL asks it for the fonts.
//
// Identical texts are printed once, under every crate that uses them. A crate that ships no
// licence file gets the standard text of a licence it allows (MIT, Apache-2.0 or the Boost
// licence); anything else stops the build with the crate's name, so nothing goes out unlisted.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const assetsDir = join(here, "notices");

const LICENCE_FILE = /^(licen[cs]e|copying|notice|copyright)/i;
const FALLBACKS = ["MIT", "Apache-2.0", "BSL-1.0"];

// The C and C++ libraries inside Skia's prebuilt binaries for the features below, checked against
// the symbols in skia.lib (Windows draws text with DirectWrite, so no FreeType; no textlayout, so no
// HarfBuzz or ICU; no vulkan, so no Vulkan Memory Allocator), plus Rust's standard library.
export const SKIA_FEATURES = ["d3d", "ganesh", "gl", "jpegd", "jpege", "pdf"];
const BUNDLED = [
  ["Skia", "skia.txt"],
  ["libpng", "libpng.txt"],
  ["zlib", "zlib.txt"],
  ["libjpeg-turbo", "libjpeg-turbo.txt", "libjpeg-turbo-ijg.txt"],
  ["Expat", "expat.txt"],
  ["Wuffs (image decoding)", "wuffs.txt"],
  ["D3D12 Memory Allocator", "d3d12-memory-allocator.txt"],
  ["Rust standard library", "rust.txt"],
];
const TARGET = "x86_64-pc-windows-msvc";

/**
 * Throws unless a skia-bindings build key (out/skia/key.txt, "<hash>-<target>-<features>") has exactly
 * SKIA_FEATURES: a new feature can bring libraries BUNDLED doesn't name, so the list must be redone.
 */
export function checkSkiaKey(key) {
  const text = key.trim();
  const at = text.indexOf(`-${TARGET}-`);
  if (at < 0) throw new Error(`Can't read the Skia build key ${JSON.stringify(text)}`);
  const features = text.slice(at + TARGET.length + 2).split("-");
  const added = features.filter((f) => !SKIA_FEATURES.includes(f));
  const gone = SKIA_FEATURES.filter((f) => !features.includes(f));
  if (added.length || gone.length) {
    const what = [added.length ? `gained ${added.join(", ")}` : "", gone.length ? `lost ${gone.join(", ")}` : ""].filter(Boolean).join(" and ");
    throw new Error(`Skia's build ${what}: check which libraries its binaries carry now, then update BUNDLED and SKIA_FEATURES in notices.mjs`);
  }
}

/** The newest skia-bindings build key under target/<profile>, or null if Skia hasn't been built there. */
export function newestSkiaKey(profile) {
  const build = join(root, "target", profile, "build");
  if (!existsSync(build)) return null;
  const keys = readdirSync(build)
    .filter((d) => d.startsWith("skia-bindings-"))
    .map((d) => join(build, d, "out", "skia", "key.txt"))
    .filter((f) => existsSync(f))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return keys.length ? readFileSync(keys[0], "utf8") : null;
}
const FONTS = [
  ["Bricolage Grotesque", "OFL-bricolage-grotesque.txt"],
  ["IBM Plex Mono", "OFL-ibm-plex-mono.txt"],
];

export function cargoPath() {
  const candidates = [process.env.CARGO, join(homedir(), ".cargo", "bin", "cargo.exe")];
  return candidates.find((p) => p && existsSync(p)) ?? null;
}

/** Registry and git crates reachable from the app over normal dependencies, sorted by name and version. */
export function collectCrates(metadata, rootName = "fettle-windows") {
  const packages = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const start = metadata.packages.find((p) => p.name === rootName && p.source == null);
  if (!start) throw new Error(`${rootName} isn't in this workspace`);
  const seen = new Set();
  const stack = [start.id];
  while (stack.length) {
    const id = stack.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const d of nodes.get(id)?.deps ?? []) if (d.dep_kinds.some((k) => k.kind === null)) stack.push(d.pkg);
  }
  return [...seen]
    .map((id) => packages.get(id))
    .filter((p) => p.source != null)
    .map((p) => ({ name: p.name, version: p.version, license: p.license ?? null, authors: p.authors ?? [], dir: dirname(p.manifest_path) }))
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version, undefined, { numeric: true }));
}

const clean = (text) => text.replace(/^﻿/, "").replace(/\r\n?/g, "\n").replace(/[ \t]+$/gm, "").trim();
const key = (text) => text.replace(/\s+/g, " ").trim();

/** The standard text of a licence the crate's SPDX expression allows, for crates that ship no file. */
function fallback(crate, licences) {
  const label = `${crate.name} ${crate.version} (${crate.license ?? "no licence field"})`;
  if (!crate.license) throw new Error(`No licence text for ${label}`);
  const parts = crate.license.replace(/[()]/g, "").split(/\s+AND\s+/);
  return parts.map((part) => {
    const allowed = part.split(/\s+OR\s+|\//).map((s) => s.trim());
    const pick = FALLBACKS.find((l) => allowed.includes(l));
    if (!pick) throw new Error(`No licence text for ${label}`);
    if (pick !== "MIT") return licences[pick];
    const people = crate.authors.map((a) => a.replace(/\s*<[^>]*>/, "").trim()).filter(Boolean);
    const holder = `the ${crate.name} authors${people.length ? ` (${people.join(", ")})` : ""}`;
    return licences.MIT.replace("<year> <copyright holders>", holder);
  });
}

/** "Used by: a 1.0, b 2.0, …" split into lines of at most 78 characters, never inside a "name version". */
function usedBy(names, width = 78) {
  const lines = [];
  let current = "Used by:";
  names.forEach((name, i) => {
    const item = i < names.length - 1 ? `${name},` : name;
    if (current.length + 1 + item.length > width) {
      lines.push(current);
      current = item;
    } else current = `${current} ${item}`;
  });
  return [...lines, current];
}

/** The whole notices file, with Windows line ends. */
export function renderNotices(crates, { readDir, isDir, readFile, bundled, fonts, licences }) {
  const path = (...parts) => join(...parts).replaceAll("\\", "/");
  // A LICENSES folder (the REUSE layout) counts file by file.
  const licenceFiles = (dir) =>
    readDir(dir)
      .filter((f) => LICENCE_FILE.test(f))
      .sort()
      .flatMap((f) => (isDir(path(dir, f)) ? readDir(path(dir, f)).sort().map((g) => path(dir, f, g)) : [path(dir, f)]));
  const groups = new Map();
  for (const crate of crates) {
    const files = licenceFiles(crate.dir);
    const texts = files.length ? files.map(readFile) : fallback(crate, licences);
    for (const raw of texts) {
      const text = clean(raw);
      const k = key(text);
      if (!groups.has(k)) groups.set(k, { text, crates: [] });
      const names = groups.get(k).crates;
      const label = `${crate.name} ${crate.version}`;
      if (!names.includes(label)) names.push(label);
    }
  }

  const rule = "=".repeat(78);
  const out = [
    "Fettle: third-party licences",
    rule,
    "",
    "Fettle is free software under the MIT licence (see LICENSE.txt). It is built with the",
    "open-source software below, whose licences ask that their notices travel with the program.",
    "",
    "Fettle è software libero con licenza MIT (vedi LICENSE.txt). Qui sotto ci sono le licenze",
    "del software open source con cui è costruito.",
    "",
    "This software is based in part on the work of the Independent JPEG Group.",
    "",
    "",
    "1. Inside the graphics renderer (Skia), and the Rust standard library",
    rule,
  ];
  for (const b of bundled) out.push("", `--- ${b.title} ---`, "", clean(b.text));
  out.push("", "", "2. Fonts", rule);
  for (const f of fonts) out.push("", `--- ${f.title} ---`, "", clean(f.text));
  out.push("", "", "3. Rust crates", rule);
  for (const g of groups.values()) out.push("", ...usedBy(g.crates), "-".repeat(78), "", g.text);
  return out.join("\n").replace(/\n/g, "\r\n") + "\r\n";
}

/** The texts kept in this repo: Skia's libraries, the font licences and the fallback licences. */
export function loadAssets() {
  const read = (p) => readFileSync(p, "utf8");
  return {
    bundled: BUNDLED.map(([title, ...files]) => ({ title, text: files.map((f) => clean(read(join(assetsDir, "bundled", f)))).join("\n\n") })),
    fonts: FONTS.map(([title, file]) => ({ title, text: read(join(root, "apps", "windows", "ui", "fonts", file)) })),
    licences: Object.fromEntries(FALLBACKS.map((l) => [l, read(join(assetsDir, "licences", `${l}.txt`))])),
  };
}

/**
 * Reads this repo's dependency graph and writes the notices file. Returns the crates listed. With
 * skiaProfile ("release" when packaging), Skia's build there must exist and match SKIA_FEATURES.
 */
export function writeNotices(outFile, { skiaProfile } = {}) {
  if (skiaProfile) {
    const key = newestSkiaKey(skiaProfile);
    if (!key) throw new Error(`No Skia build in target/${skiaProfile}: build the app first`);
    checkSkiaKey(key);
  }
  const cargo = cargoPath();
  if (!cargo) throw new Error("cargo not found (set CARGO)");
  const json = execFileSync(cargo, ["metadata", "--format-version", "1", "--locked", "--filter-platform", "x86_64-pc-windows-msvc"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  const crates = collectCrates(JSON.parse(json));
  const text = renderNotices(crates, { readDir: (d) => readdirSync(d), isDir: (p) => statSync(p).isDirectory(), readFile: (p) => readFileSync(p, "utf8"), ...loadAssets() });
  mkdirSync(dirname(outFile), { recursive: true });
  writeFileSync(outFile, text);
  return crates;
}
