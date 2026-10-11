import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import { encodedRustFlags, remapFlags, timesIn } from "../paths.mjs";

const build = readFileSync(join(import.meta.dirname, "..", "build.mjs"), "utf8");
const home = "C:\\Users\\Jane Example";

describe("remapFlags", () => {
  it("writes the build machine's folders under neutral names, the most specific last (the last match wins)", () => {
    const flags = remapFlags({ home, cargoHome: `${home}\\.cargo`, rustupHome: `${home}\\.rustup`, root: `${home}\\code\\fettle` });
    assert.deepEqual(flags, [
      `--remap-path-prefix=${home}=/home`,
      `--remap-path-prefix=${home}\\.cargo=/cargo`,
      `--remap-path-prefix=${home}\\.rustup=/rustup`,
      `--remap-path-prefix=${home}\\code\\fettle=/fettle`,
    ]);
  });

  it("leaves out a folder it doesn't know", () => {
    assert.deepEqual(remapFlags({ home, root: "D:\\fettle" }), [`--remap-path-prefix=${home}=/home`, "--remap-path-prefix=D:\\fettle=/fettle"]);
  });
});

describe("encodedRustFlags", () => {
  it("keeps a folder name with a space in one piece, which RUSTFLAGS can't", () => {
    assert.equal(encodedRustFlags(["--remap-path-prefix=C:\\Developing Area=/fettle", "-Cfoo"]), "--remap-path-prefix=C:\\Developing Area=/fettle\x1f-Cfoo");
  });
});

describe("timesIn", () => {
  it("counts a path in a program, in any case, as plain and as wide text", () => {
    const exe = Buffer.concat([Buffer.from("xx C:\\Users\\Jane Example\\.cargo\\a.rs yy c:\\users\\jane example\\b"), Buffer.from("C:\\Users\\Jane Example\\c", "utf16le")]);
    assert.equal(timesIn(exe, home), 3);
    assert.equal(timesIn(Buffer.from("/home/registry/src/a.rs"), home), 0);
  });
});

describe("build.mjs", () => {
  it("builds from Cargo.lock exactly, with the build machine's folders renamed", () => {
    assert.match(build, /"build", "--release", "--locked", "-p", "fettle-windows"/);
    assert.match(build, /CARGO_ENCODED_RUSTFLAGS: encodedRustFlags\(remapFlags\(/);
  });

  it("stops if the exe still holds the build machine's user folder", () => {
    assert.match(build, /timesIn\(exeBytes, homedir\(\)\)/);
  });
});
