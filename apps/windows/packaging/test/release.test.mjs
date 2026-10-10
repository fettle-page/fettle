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
  }

  it("ci.yml runs every test on pushes and pull requests", () => {
    const ci = workflow("ci.yml");
    assert.match(ci, /^\s+push:/m);
    assert.match(ci, /^\s+pull_request:/m);
    assert.match(ci, /npm test/);
    assert.match(ci, /cargo test --workspace/);
    assert.match(ci, /rustup target add wasm32-unknown-unknown/);
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
});
