import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, it } from "node:test";
import { delayImports, loadFlags } from "../pe.mjs";
import { crtDirIn, exitCodes, knownDllsIn, manifestFiles, newestCrt, readExitCodes } from "../runtime.mjs";

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
    assert.deepEqual(crtDirIn(vs), { dir: join(vs, "VC", "Redist", "MSVC", "14.44.35112", "x64", "Microsoft.VC143.CRT"), version: "14.44.35112" });
  });

  it("returns null when the installation has no runtime to ship", () => {
    assert.equal(crtDirIn(scratch("vs-empty")), null);
  });
});

describe("newestCrt", () => {
  it("picks the newest runtime by its version, whichever Visual Studio it comes with", () => {
    const older = { dir: "C:\\VS\\2022\\BuildTools\\VC\\Redist\\MSVC\\14.38.33135\\x64\\Microsoft.VC143.CRT", version: "14.38.33135" };
    const newer = { dir: "C:\\VS\\18\\Community\\VC\\Redist\\MSVC\\14.51.36231\\x64\\Microsoft.VC145.CRT", version: "14.51.36231" };
    assert.equal(newestCrt([older, newer]), newer.dir);
    assert.equal(newestCrt([newer, older]), newer.dir);
    assert.equal(newestCrt([]), null);
  });
});

describe("readExitCodes", () => {
  it("matches each exit code to its run, even when one run couldn't start", () => {
    assert.deepEqual(readExitCodes("0:1\r\n2:-1073741521\r\n", 3), ["1", "not started", "-1073741521"]);
    assert.deepEqual(readExitCodes("1:timeout\n", 2), ["not started", "timeout"]);
  });
});

describe("exitCodes", () => {
  it("takes a long list of runs (too long to pass in the environment) and keeps Windows' error boxes off", () => {
    const dir = scratch("many-runs");
    const runs = Array.from({ length: 400 }, (_, i) => [join(dir, `missing-${i}`, "fettle.exe"), `--deep-scan "${join(dir, `missing-${i}`, "fettle-deep-0-0.json")}"`]);
    const codes = exitCodes(runs, dir);
    assert.equal(codes.length, 400);
    assert.ok(codes.every((c) => c === "not started"), codes.slice(0, 3).join(" "));
  });
});

describe("knownDllsIn", () => {
  it("reads Windows' list of DLLs that always load from System32", () => {
    const out = "\r\nHKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\KnownDLLs\r\n    *kernel32    REG_SZ    kernel32.dll\r\n    COMDLG32    REG_SZ    COMDLG32.dll\r\n    PSAPI    REG_SZ    PSAPI.DLL\r\n\r\n";
    assert.deepEqual([...knownDllsIn(out)].sort(), ["comdlg32.dll", "kernel32.dll", "psapi.dll"]);
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
    // Their own dependencies would be looked for beside fettle.exe first: loaded when first used,
    // after main has limited every load to System32, they come from System32 too.
    it(`(${profile}) loads secur32, netapi32 and opengl32 only when first used`, { skip: !existsSync(exe) && `no ${profile} build` }, () => {
      const delayed = delayImports(readFileSync(exe)).map((d) => d.toLowerCase());
      for (const d of ["secur32.dll", "netapi32.dll", "opengl32.dll"]) assert.ok(delayed.includes(d), `${d} isn't delay-loaded: ${delayed.join(", ")}`);
    });
  }
});
