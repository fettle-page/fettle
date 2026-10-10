// The installer script and the build can't run here without Inno Setup, so these read them as text.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";

const dir = join(import.meta.dirname, "..");
const iss = readFileSync(join(dir, "fettle.iss"), "utf8");
const build = readFileSync(join(dir, "build.mjs"), "utf8");

describe("fettle.iss", () => {
  it("keeps the installer's identity, so upgrades stay upgrades", () => {
    assert.match(iss, /^AppId=\{\{9404A274-E009-4582-B280-0BE4327BB7A1\}$/m);
  });

  it("installs Fettle's licence and the third-party notices beside the exe", () => {
    assert.match(iss, /^#ifndef NoticesDir\s*\n\s*#error/m);
    assert.match(iss, /^Source: "\{#NoticesDir\}\\LICENSE\.txt"; DestDir: "\{app\}"/m);
    assert.match(iss, /^Source: "\{#NoticesDir\}\\THIRD-PARTY-NOTICES\.txt"; DestDir: "\{app\}"/m);
  });
});

describe("build.mjs", () => {
  it("writes the notices and Fettle's licence, and hands their folder to Inno Setup", () => {
    assert.match(build, /writeNotices\(.*, \{ skiaProfile: "release" \}\);/);
    assert.match(build, /LICENSE\.txt/);
    assert.match(build, /\/DNoticesDir=/);
  });

  it("writes SHA256SUMS.txt for the installer and the zip", () => {
    assert.match(build, /writeFileSync\(join\(out, "SHA256SUMS\.txt"\), sums\(\[installer, zip\]\)\)/);
  });

  it("puts both files in the portable zip, beside fettle.exe", () => {
    const tarCall = /execFileSync\(tar, \[([^\]]*)\]/.exec(build)?.[1] ?? "";
    for (const f of ["fettle.exe", "LICENSE.txt", "THIRD-PARTY-NOTICES.txt", "Fettle data"]) assert.ok(tarCall.includes(`"${f}"`), `${f} not in the zip`);
  });
});
