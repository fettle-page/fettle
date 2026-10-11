import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import { appVersion, checkTag, sums } from "../release.mjs";

const root = join(import.meta.dirname, "../../../..");
const workflow = (name) => readFileSync(join(root, ".github", "workflows", name), "utf8");
const scripts = Object.keys(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).scripts);
const INNO_SHA256 = "9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732";
// skia-binaries 0.153.3, the archive the app builds with (matches GitHub's digest for that asset).
const SKIA_SHA256 = "a19adbcb3c8e9d691ce4e9350fcc3e3ce066a0793263d5358185efbceae46f30";
const lockfile = readFileSync(join(root, "Cargo.lock"), "utf8");

describe("checkTag", () => {
  it("accepts the tag v<version>", () => {
    assert.doesNotThrow(() => checkTag("v1.0.0", "1.0.0"));
  });

  for (const tag of ["v1.0.1", "1.0.0", "v1.0.0-rc1", "", undefined]) {
    it(`refuses ${JSON.stringify(tag)} for version 1.0.0, naming both`, () => {
      assert.throws(() => checkTag(tag, "1.0.0"), (e) => e.message.includes("1.0.0") && e.message.includes("v1.0.0"));
    });
  }
});

describe("appVersion", () => {
  it("reads the app's version from apps/windows/Cargo.toml", () => {
    const cargo = readFileSync(join(root, "apps", "windows", "Cargo.toml"), "utf8");
    assert.equal(appVersion(), /^version\s*=\s*"([^"]+)"/m.exec(cargo)[1]);
  });
});

describe("sums", () => {
  it("writes sha256sum lines, sorted by file name, with LF line ends", () => {
    const dir = join(root, "target", "tmp", "sums-test");
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "b.zip"), "bee");
    writeFileSync(join(dir, "a.exe"), "ay");
    const hex = (s) => createHash("sha256").update(s).digest("hex");
    assert.equal(sums([join(dir, "b.zip"), join(dir, "a.exe")]), `${hex("ay")}  a.exe\n${hex("bee")}  b.zip\n`);
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("workflows", () => {
  for (const name of ["ci.yml", "release.yml"]) {
    it(`${name} pins every action to a full commit, with its version beside it`, () => {
      const uses = [...workflow(name).matchAll(/uses:\s*(\S+)(.*)$/gm)];
      assert.ok(uses.length > 0);
      for (const [, ref, rest] of uses) {
        assert.match(ref, /^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/, ref);
        assert.match(rest, /# v\d+\.\d+\.\d+/, ref);
      }
    });

    it(`${name} doesn't leave the GitHub token in .git/config, where install and build scripts could read it`, () => {
      const checkouts = [...workflow(name).matchAll(/uses: actions\/checkout@\S+.*\n(\s+)with:\n((?:\1\s+.*\n)+)/g)];
      assert.equal(checkouts.length, (workflow(name).match(/uses: actions\/checkout@/g) ?? []).length, "every checkout has a with: block");
      for (const [, , block] of checkouts) assert.match(block, /persist-credentials: false/);
    });

    it(`${name} only runs npm scripts that exist`, () => {
      for (const [, script] of workflow(name).matchAll(/npm run ([\w:-]+)/g)) assert.ok(scripts.includes(script), script);
    });

    // skia-bindings' build script downloads Skia's prebuilt libraries and doesn't check them.
    it(`${name} checks Skia's prebuilt libraries against a pinned hash before cargo runs`, () => {
      const w = workflow(name);
      const skia = /^name = "skia-bindings"\nversion = "([^"]+)"/m.exec(lockfile)[1];
      assert.ok(w.includes(`/releases/download/${skia}/`), `downloads the archive for skia-bindings ${skia}, as Cargo.lock has it`);
      const check = w.indexOf(SKIA_SHA256);
      assert.ok(check >= 0, "checks the pinned hash");
      assert.ok(w.indexOf("SKIA_BINARIES_URL=file://") > check, "points the build at the checked file");
      assert.ok(w.indexOf("run: npm ci") > check && w.indexOf("run: cargo ") > check, "before anything builds");
    });

    it(`${name} builds with exactly the dependencies in Cargo.lock`, () => {
      const runs = [...workflow(name).matchAll(/run: (cargo .*)$/gm)].map((m) => m[1]);
      assert.ok(runs.length > 0);
      for (const run of runs) assert.match(run, /--locked/, run);
    });
  }

  // rust-toolchain.toml can't name both a version and the MSVC host (rustup refuses it, and a PC whose
  // default host is GNU can't build the app), so it says "stable" for local builds and the workflows
  // pin the exact version.
  it("ci.yml and release.yml build with the same exact Rust version, with the WebAssembly target", () => {
    const pins = ["ci.yml", "release.yml"].map((name) => {
      const w = workflow(name);
      const pin = /^\s+RUSTUP_TOOLCHAIN: "(\d+\.\d+\.\d+)-x86_64-pc-windows-msvc"$/m.exec(w);
      assert.ok(pin, `${name} sets RUSTUP_TOOLCHAIN to an exact version`);
      assert.match(w, new RegExp(`rustup toolchain install ${pin[1].replaceAll(".", "\\.")}-x86_64-pc-windows-msvc --profile minimal --target wasm32-unknown-unknown`), name);
      return pin[1];
    });
    assert.equal(pins[0], pins[1]);
  });

  it("ci.yml runs every test on pushes and pull requests", () => {
    const ci = workflow("ci.yml");
    assert.match(ci, /^\s+push:/m);
    assert.match(ci, /^\s+pull_request:/m);
    assert.match(ci, /npm test/);
    assert.match(ci, /cargo test --workspace/);
    assert.match(ci, /rustup toolchain install .* --target wasm32-unknown-unknown/);
  });

  it("release.yml runs only for version tags, checks the tag first, and drafts the release", () => {
    const r = workflow("release.yml");
    assert.match(r, /tags:\s*\n\s*- "v\*"/);
    assert.doesNotMatch(r, /branches:/);
    assert.doesNotMatch(r, /pull_request/);
    assert.match(r, /contents: write/);
    assert.match(r, /id-token: write/);
    assert.match(r, /attestations: write/);
    const at = (s) => {
      const i = r.indexOf(s);
      assert.ok(i >= 0, `${s} missing`);
      return i;
    };
    assert.ok(at("npm run release:check") < at("npm test"));
    assert.ok(at("npm test") < at("npm run package:windows"));
    assert.ok(at(INNO_SHA256) < at("Start-Process"), "the Inno Setup download is checked before it runs");
    assert.ok(at("npm run package:windows") < at("attest-build-provenance"));
    assert.match(r, /gh release create[^\n]*--draft/);
    assert.match(r, /SHA256SUMS\.txt/);
  });

  // Install and build scripts (npm, cargo, Inno Setup) run in a job that can only read the repo; the
  // job that can write (attestations, the draft release) runs none of them.
  it("release.yml builds with read-only rights and publishes in a separate job", () => {
    const r = workflow("release.yml");
    assert.match(r, /^permissions: \{\}$/m, "no rights by default");
    const job = (name) => {
      const m = new RegExp(`^  ${name}:\\n([\\s\\S]*?)(?=^  \\w[\\w-]*:\\n|(?![\\s\\S]))`, "m").exec(r);
      assert.ok(m, `${name} job missing`);
      return m[1];
    };
    const build = job("build");
    const publish = job("publish");
    assert.match(build, /permissions:\n\s+contents: read\n/);
    assert.doesNotMatch(build, /: write/);
    for (const step of ["npm ci", "npm test", "cargo test", "npm run package:windows", "upload-artifact"]) assert.ok(build.includes(step), `build: ${step}`);
    assert.match(publish, /needs: build/);
    for (const right of ["contents: write", "id-token: write", "attestations: write"]) assert.ok(publish.includes(right), `publish: ${right}`);
    for (const step of ["download-artifact", "sha256sum -c SHA256SUMS.txt", "attest-build-provenance", "gh release create"]) assert.ok(publish.includes(step), `publish: ${step}`);
    for (const forbidden of ["npm ", "cargo ", "actions/checkout", "Inno Setup"]) assert.ok(!publish.includes(forbidden), `publish runs ${forbidden}`);
  });
});
