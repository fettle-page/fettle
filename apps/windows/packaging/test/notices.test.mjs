import assert from "node:assert/strict";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import { SKIA_FEATURES, cargoPath, checkSkiaKey, collectCrates, loadAssets, newestSkiaKey, renderNotices, writeNotices } from "../notices.mjs";

const pkg = (name, version, license, extra = {}) => ({
  id: `${name}@${version}`,
  name,
  version,
  license,
  authors: [],
  source: "registry+https://github.com/rust-lang/crates.io-index",
  manifest_path: `/reg/${name}-${version}/Cargo.toml`,
  ...extra,
});
const normal = (to) => ({ pkg: to, dep_kinds: [{ kind: null }] });

const metadata = {
  packages: [
    pkg("fettle-windows", "1.0.0", "MIT", { source: null, manifest_path: "/ws/apps/windows/Cargo.toml" }),
    pkg("fettle-facts", "0.1.0", "MIT", { source: null, manifest_path: "/ws/crates/fettle-facts/Cargo.toml" }),
    pkg("alpha", "1.0.0", "MIT OR Apache-2.0"),
    pkg("beta", "2.0.0", "MIT"),
    pkg("devonly", "1.0.0", "MIT"),
    pkg("buildonly", "1.0.0", "MIT"),
    pkg("deep", "0.3.0", "MIT"),
  ],
  resolve: {
    nodes: [
      { id: "fettle-windows@1.0.0", deps: [normal("fettle-facts@0.1.0"), normal("alpha@1.0.0"), { pkg: "devonly@1.0.0", dep_kinds: [{ kind: "dev" }] }, { pkg: "buildonly@1.0.0", dep_kinds: [{ kind: "build" }] }] },
      { id: "fettle-facts@0.1.0", deps: [normal("beta@2.0.0")] },
      { id: "alpha@1.0.0", deps: [normal("deep@0.3.0"), normal("beta@2.0.0")] },
      { id: "beta@2.0.0", deps: [] },
      { id: "devonly@1.0.0", deps: [] },
      { id: "buildonly@1.0.0", deps: [] },
      { id: "deep@0.3.0", deps: [] },
    ],
  },
};

const MIT = "MIT License\n\nCopyright (c) <year> <copyright holders>\n\nPermission is hereby granted, free of charge.\n";
const APACHE = "Apache License\nVersion 2.0, January 2004\n";
const BSL = "Boost Software License - Version 1.0 - August 17th, 2003\n";

function fakeFs(files) {
  return {
    readDir: (dir) => [...new Set(Object.keys(files).filter((p) => p.startsWith(dir + "/")).map((p) => p.slice(dir.length + 1).split("/")[0]))],
    isDir: (path) => Object.keys(files).some((p) => p.startsWith(path + "/")),
    readFile: (path) => {
      if (!(path in files)) throw new Error(`no file ${path}`);
      return files[path];
    },
  };
}

const crate = (name, version, license, authors = []) => ({ name, version, license, authors, dir: `/reg/${name}-${version}` });
const assets = {
  bundled: [{ title: "Skia", text: "Copyright (c) 2011 Google Inc." }],
  fonts: [{ title: "Bricolage Grotesque", text: "SIL OPEN FONT LICENSE Version 1.1 (Bricolage)" }, { title: "IBM Plex Mono", text: "SIL OPEN FONT LICENSE Version 1.1 (Plex)" }],
  licences: { MIT, "Apache-2.0": APACHE, "BSL-1.0": BSL },
};

describe("collectCrates", () => {
  it("follows normal dependencies only, skips the workspace's own crates, and sorts by name", () => {
    assert.deepEqual(
      collectCrates(metadata).map((c) => `${c.name} ${c.version}`),
      ["alpha 1.0.0", "beta 2.0.0", "deep 0.3.0"],
    );
  });

  it("gives each crate its folder, licence and authors", () => {
    const [alpha] = collectCrates(metadata);
    assert.equal(alpha.dir.replaceAll("\\", "/"), "/reg/alpha-1.0.0");
    assert.equal(alpha.license, "MIT OR Apache-2.0");
    assert.deepEqual(alpha.authors, []);
  });
});

describe("renderNotices", () => {
  it("includes every licence file a crate ships", () => {
    const fs = fakeFs({ "/reg/a-1.0.0/LICENSE-MIT": "a mit text", "/reg/a-1.0.0/LICENSE-APACHE": "a apache text", "/reg/a-1.0.0/src.rs": "code" });
    const out = renderNotices([crate("a", "1.0.0", "MIT OR Apache-2.0")], { ...fs, ...assets });
    assert.match(out, /a mit text/);
    assert.match(out, /a apache text/);
    assert.doesNotMatch(out, /code/);
  });

  it("reads every file in a LICENSES folder", () => {
    const fs = fakeFs({ "/reg/r-1.0.0/LICENSES/MIT.txt": "r mit", "/reg/r-1.0.0/LICENSES/Apache-2.0.txt": "r apache" });
    const out = renderNotices([crate("r", "1.0.0", "MIT OR Apache-2.0")], { ...fs, ...assets });
    assert.match(out, /r mit/);
    assert.match(out, /r apache/);
  });

  it("prints a text shared by several crates once, under all their names", () => {
    const fs = fakeFs({ "/reg/a-1.0.0/LICENSE": "same  text\n", "/reg/b-2.0.0/COPYING": "same text" });
    const out = renderNotices([crate("a", "1.0.0", "MIT"), crate("b", "2.0.0", "MIT")], { ...fs, ...assets });
    assert.equal(out.split("same").length - 1, 1);
    assert.match(out, /a 1\.0\.0/);
    assert.match(out, /b 2\.0\.0/);
  });

  it("falls back to the MIT text, naming the crate, when a crate ships no licence file", () => {
    const out = renderNotices([crate("lyon_path", "1.0.19", "MIT OR Apache-2.0", ["Jane Example <jane@example.com>"])], { ...fakeFs({}), ...assets });
    assert.match(out, /Copyright \(c\) the lyon_path authors \(Jane Example\)/);
    assert.match(out, /Permission is hereby granted/);
    assert.doesNotMatch(out, /<copyright holders>/);
  });

  it("falls back to Apache-2.0 or the Boost licence when that's the only choice", () => {
    const out = renderNotices([crate("spirv", "0.4.0", "Apache-2.0"), crate("clipboard-win", "5.4.1", "BSL-1.0")], { ...fakeFs({}), ...assets });
    assert.match(out, /Apache License/);
    assert.match(out, /Boost Software License/);
  });

  it("fails, naming the crate, when there's no file and no text for its licence", () => {
    assert.throws(() => renderNotices([crate("odd", "0.1.0", "LicenseRef-Odd")], { ...fakeFs({}), ...assets }), /odd 0\.1\.0 \(LicenseRef-Odd\)/);
  });

  it("names the bundled libraries, the fonts and the Independent JPEG Group, with Windows line ends", () => {
    const out = renderNotices([], { ...fakeFs({}), ...assets });
    assert.match(out, /Skia/);
    assert.match(out, /Bricolage Grotesque/);
    assert.match(out, /IBM Plex Mono/);
    assert.match(out, /based in part on the work of the Independent JPEG Group/);
    assert.match(out, /Microsoft Visual C\+\+ runtime/);
    assert.match(out, /vcruntime140\.dll/);
    assert.ok(out.includes("\r\n"));
    assert.ok(!/[^\r]\n/.test(out), "every line ends in CRLF");
  });

  it("wraps a long list of crate names at 78 characters, keeping each name with its version", () => {
    const many = Array.from({ length: 40 }, (_, i) => crate(`crate-number-${i}`, "1.0.0", "MIT"));
    const fs = fakeFs(Object.fromEntries(many.map((c) => [`${c.dir}/LICENSE`, "one shared text"])));
    const lines = renderNotices(many, { ...fs, ...assets }).split("\r\n").filter((l) => l.includes("crate-number"));
    assert.ok(lines.length > 5);
    for (const line of lines) assert.ok(line.length <= 78, line);
    for (let i = 0; i < 40; i++) assert.ok(lines.some((l) => l.includes(`crate-number-${i} 1.0.0`)), `crate-number-${i}`);
  });
});

describe("loadAssets", () => {
  it("reads the bundled libraries, both font licences and the three fallback texts from the repo", () => {
    const a = loadAssets();
    const titles = a.bundled.map((b) => b.title).join(" | ");
    for (const lib of ["Skia", "libpng", "zlib", "libjpeg-turbo", "Expat", "Wuffs", "D3D12 Memory Allocator", "Rust standard library"]) assert.match(titles, new RegExp(lib));
    // Not in the prebuilt Skia for our features (no vulkan, no textlayout): checked against skia.lib's symbols.
    for (const lib of ["Vulkan Memory Allocator", "ICU", "HarfBuzz", "FreeType"]) assert.doesNotMatch(titles, new RegExp(lib));
    assert.deepEqual(a.fonts.map((f) => f.title), ["Bricolage Grotesque", "IBM Plex Mono"]);
    for (const f of a.fonts) assert.match(f.text, /SIL OPEN FONT LICENSE/i);
    assert.match(a.licences.MIT, /<copyright holders>/);
    assert.match(a.licences["Apache-2.0"], /Apache License/);
    assert.match(a.licences["BSL-1.0"], /Boost Software License/);
  });
});

describe("the Skia build the bundled list was written for", () => {
  const key = (features) => `b7f043e0b1e2a850e702-x86_64-pc-windows-msvc-${features.join("-")}`;

  it("is d3d, ganesh, gl, jpeg decode and encode, and pdf", () => {
    assert.deepEqual(SKIA_FEATURES, ["d3d", "ganesh", "gl", "jpegd", "jpege", "pdf"]);
    assert.doesNotThrow(() => checkSkiaKey(key(SKIA_FEATURES) + "\n"));
  });

  it("stops the build when Skia gains a feature, which can bring libraries the notices don't name", () => {
    assert.throws(() => checkSkiaKey(key([...SKIA_FEATURES, "textlayout"])), /textlayout/);
    assert.throws(() => checkSkiaKey(key(["d3d", "ganesh", "gl", "jpegd", "jpege", "pdf", "vulkan"])), /vulkan/);
  });

  it("stops the build when a feature goes, so the list doesn't keep naming what isn't there", () => {
    assert.throws(() => checkSkiaKey(key(["d3d", "ganesh", "gl", "pdf"])), /jpegd/);
  });

  it("stops the build on a key it can't read", () => {
    assert.throws(() => checkSkiaKey("nonsense"), /key/i);
  });

  it("matches this repo's own Skia build, when there is one", { skip: !newestSkiaKey("debug") && !newestSkiaKey("release") && "no Skia build yet" }, () => {
    for (const profile of ["debug", "release"]) {
      const k = newestSkiaKey(profile);
      if (k) assert.doesNotThrow(() => checkSkiaKey(k), profile);
    }
  });
});

describe("writeNotices on this repo", { skip: !cargoPath() && "cargo not found" }, () => {
  it("writes one file that names every crate in the app, Slint and Skia, under 2 MB", () => {
    const dir = join(import.meta.dirname, "../../../../target/tmp/notices-test");
    rmSync(dir, { recursive: true, force: true });
    const file = join(dir, "THIRD-PARTY-NOTICES.txt");
    const crates = writeNotices(file);
    assert.ok(existsSync(file));
    const text = readFileSync(file, "utf8");
    assert.ok(crates.length > 100, `only ${crates.length} crates`);
    for (const c of crates) assert.ok(text.includes(`${c.name} ${c.version}`), `${c.name} ${c.version} missing`);
    assert.match(text, /\bslint \d/);
    assert.match(text, /\bskia-safe \d/);
    assert.ok(text.length < 2_000_000, `${text.length} bytes`);
    rmSync(dir, { recursive: true, force: true });
  });
});
