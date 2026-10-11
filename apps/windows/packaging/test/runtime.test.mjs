import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, it } from "node:test";
import { loadFlags } from "../pe.mjs";
import { crtDirIn, manifestFiles } from "../runtime.mjs";

const root = resolve(import.meta.dirname, "../../../..");

function scratch(name) {
  const dir = join(root, "target", "tmp", "fettle-packaging", name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

describe("crtDirIn", () => {
  it("picks the newest Visual C++ runtime folder of a Visual Studio installation", () => {
    const vs = scratch("vs");
    for (const v of ["14.40.33807", "14.44.35112", "14.9.1"]) mkdirSync(join(vs, "VC", "Redist", "MSVC", v, "x64", "Microsoft.VC143.CRT"), { recursive: true });
    mkdirSync(join(vs, "VC", "Redist", "MSVC", "v143"), { recursive: true });
    mkdirSync(join(vs, "VC", "Redist", "MSVC", "14.50.1", "x86", "Microsoft.VC145.CRT"), { recursive: true });
    assert.equal(crtDirIn(vs), join(vs, "VC", "Redist", "MSVC", "14.44.35112", "x64", "Microsoft.VC143.CRT"));
  });

  it("returns null when the installation has no runtime to ship", () => {
    assert.equal(crtDirIn(scratch("vs-empty")), null);
  });
});

describe("manifestFiles", () => {
  it("lists the DLLs a program's manifest says load from its own folder", () => {
    const exe = Buffer.concat([
      Buffer.from("MZ...binary..."),
      Buffer.from('<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">\n  <file name="vcruntime140.dll"/>\n  <file name="MSVCP140.dll" />\n</assembly>'),
      Buffer.from("...more binary"),
    ]);
    assert.deepEqual(manifestFiles(exe), ["vcruntime140.dll", "msvcp140.dll"]);
    assert.deepEqual(manifestFiles(Buffer.from("MZ no manifest here")), []);
  });
});

// Every build, not only the packaged one: the exe's own DLLs come only from System32, so a DLL
// left beside fettle.exe (in Downloads, say) never loads into it, even as administrator.
describe("this repo's fettle.exe", () => {
  for (const profile of ["debug", "release"]) {
    const exe = join(root, "target", profile, "fettle.exe");
    it(`(${profile}) loads its own DLLs only from System32`, { skip: !existsSync(exe) && `no ${profile} build` }, () => {
      const stale = statSync(exe).mtimeMs < statSync(join(root, "apps", "windows", "build.rs")).mtimeMs;
      assert.equal(loadFlags(readFileSync(exe)), 0x800, stale ? `target/${profile}/fettle.exe is older than build.rs: rebuild it (cargo build${profile === "release" ? " --release" : ""} -p fettle-windows)` : undefined);
    });
  }
});
