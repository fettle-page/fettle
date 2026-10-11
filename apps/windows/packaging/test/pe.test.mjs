import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, it } from "node:test";
import { apiSet, delayImports, importChain, imports, loadFlags, missing, redistributable, runtimeFor } from "../pe.mjs";

const root = resolve(import.meta.dirname, "../../../..");
const system32 = join(process.env.SystemRoot ?? "C:\\Windows", "System32");

/** A minimal 64-bit Windows program that imports `dlls`, delay-loads `delayed`, with these load flags. */
function fakePe({ dlls = [], delayed = [], flags = null } = {}) {
  const b = Buffer.alloc(0x2000);
  b.writeUInt16LE(0x5a4d, 0);
  b.writeUInt32LE(0x40, 0x3c);
  b.writeUInt32LE(0x4550, 0x40);
  b.writeUInt16LE(0x8664, 0x44);
  b.writeUInt16LE(1, 0x46);
  b.writeUInt16LE(240, 0x54);
  const opt = 0x58;
  b.writeUInt16LE(0x20b, opt);
  const dirs = opt + 112;
  const section = opt + 240;
  b.writeUInt32LE(0x1000, section + 8);
  b.writeUInt32LE(0x1000, section + 12);
  b.writeUInt32LE(0x1000, section + 16);
  b.writeUInt32LE(0x200, section + 20);
  const rva = (offset) => offset - 0x200 + 0x1000;
  let names = 0x800;
  const name = (n) => {
    const at = names;
    b.write(`${n}\0`, at, "latin1");
    names += n.length + 1;
    return rva(at);
  };
  if (dlls.length) {
    b.writeUInt32LE(rva(0x200), dirs + 8);
    dlls.forEach((d, i) => b.writeUInt32LE(name(d), 0x200 + i * 20 + 12));
  }
  if (delayed.length) {
    b.writeUInt32LE(rva(0x400), dirs + 13 * 8);
    delayed.forEach((d, i) => {
      b.writeUInt32LE(1, 0x400 + i * 32);
      b.writeUInt32LE(name(d), 0x400 + i * 32 + 4);
    });
  }
  if (flags !== null) {
    b.writeUInt32LE(rva(0x600), dirs + 10 * 8);
    b.writeUInt32LE(0x140, 0x600);
    b.writeUInt16LE(flags, 0x600 + 78);
  }
  return b;
}

function scratch(name) {
  const dir = join(root, "target", "tmp", "fettle-packaging", name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

describe("imports", () => {
  it("lists the DLLs a program imports, delay-loaded ones too", () => {
    assert.deepEqual(imports(fakePe({ dlls: ["KERNEL32.dll", "VCRUNTIME140.dll"], delayed: ["dwmapi.dll"] })), ["KERNEL32.dll", "VCRUNTIME140.dll", "dwmapi.dll"]);
    assert.deepEqual(imports(fakePe()), []);
  });

  it("reads a real Windows program", { skip: !existsSync(join(system32, "notepad.exe")) && "no notepad.exe" }, () => {
    const list = imports(readFileSync(join(system32, "notepad.exe")));
    assert.ok(list.length > 3, list.join(" "));
    assert.ok(list.every((d) => /\.(dll|drv)$/i.test(d)), list.join(" "));
  });

  it("refuses a file that isn't a Windows program", () => {
    assert.throws(() => imports(Buffer.from("not a program at all, just text")), /not a Windows program/);
  });
});

describe("delayImports", () => {
  it("lists only the DLLs a program loads when first used", () => {
    assert.deepEqual(delayImports(fakePe({ dlls: ["KERNEL32.dll"], delayed: ["secur32.dll", "opengl32.dll"] })), ["secur32.dll", "opengl32.dll"]);
    assert.deepEqual(delayImports(fakePe({ dlls: ["KERNEL32.dll"] })), []);
  });
});

describe("loadFlags", () => {
  it("reads where a program's own DLLs may load from", () => {
    assert.equal(loadFlags(fakePe({ flags: 0x800 })), 0x800);
    assert.equal(loadFlags(fakePe({ flags: 0 })), 0);
    assert.equal(loadFlags(fakePe()), null, "no load configuration at all");
  });
});

describe("which DLLs come from where", () => {
  it("tells the Visual C++ runtime from Windows' own DLLs and API sets", () => {
    for (const d of ["VCRUNTIME140.dll", "vcruntime140_1.dll", "MSVCP140.dll", "msvcp140_atomic_wait.dll", "concrt140.dll"]) assert.ok(redistributable(d), d);
    for (const d of ["KERNEL32.dll", "ucrtbase.dll", "msvcrt.dll", "d3d12.dll", "D3DCOMPILER_47.dll"]) assert.ok(!redistributable(d), d);
    assert.ok(apiSet("api-ms-win-crt-runtime-l1-1-0.dll"));
    assert.ok(apiSet("ext-ms-win-ntuser-window-l1-1-0.dll"));
    assert.ok(!apiSet("advapi32.dll"));
  });
});

describe("missing", () => {
  it("counts the Visual C++ runtime as missing unless it ships, even when this PC has it", () => {
    const dir = scratch("missing");
    writeFileSync(join(dir, "fettle.exe"), fakePe({ dlls: ["KERNEL32.dll", "VCRUNTIME140.dll", "api-ms-win-crt-heap-l1-1-0.dll"] }));
    assert.deepEqual(missing([join(dir, "fettle.exe")], system32), ["fettle.exe → VCRUNTIME140.dll"]);
    writeFileSync(join(dir, "vcruntime140.dll"), fakePe({ dlls: ["KERNEL32.dll"] }));
    assert.deepEqual(missing([join(dir, "fettle.exe"), join(dir, "vcruntime140.dll")], system32), []);
  });

  it("counts a DLL that's neither shipped nor Windows' own as missing", () => {
    const dir = scratch("unknown");
    writeFileSync(join(dir, "fettle.exe"), fakePe({ dlls: ["KERNEL32.dll", "skia-extra.dll"] }));
    assert.deepEqual(missing([join(dir, "fettle.exe")], system32), ["fettle.exe → skia-extra.dll"]);
  });

  it("checks what the shipped DLLs need too", () => {
    const dir = scratch("chain");
    writeFileSync(join(dir, "fettle.exe"), fakePe({ dlls: ["MSVCP140.dll"] }));
    writeFileSync(join(dir, "msvcp140.dll"), fakePe({ dlls: ["VCRUNTIME140.dll", "VCRUNTIME140_1.dll"] }));
    writeFileSync(join(dir, "vcruntime140.dll"), fakePe({ dlls: ["KERNEL32.dll"] }));
    assert.deepEqual(missing(["fettle.exe", "msvcp140.dll", "vcruntime140.dll"].map((f) => join(dir, f)), system32), ["msvcp140.dll → VCRUNTIME140_1.dll"]);
  });
});

describe("importChain", () => {
  it("follows Windows' DLLs down from the program, but not into known DLLs or the runtime", () => {
    const sys = scratch("chain-system32");
    writeFileSync(join(sys, "secur32.dll"), fakePe({ dlls: ["sspicli.dll", "KERNEL32.dll", "api-ms-win-core-heap-l1-1-0.dll"] }));
    writeFileSync(join(sys, "sspicli.dll"), fakePe({ dlls: ["cryptbase.dll"] }));
    writeFileSync(join(sys, "cryptbase.dll"), fakePe());
    writeFileSync(join(sys, "kernel32.dll"), fakePe({ dlls: ["kernelbase.dll"] }));
    writeFileSync(join(sys, "opengl32.dll"), fakePe({ delayed: ["glu32.dll"] }));
    writeFileSync(join(sys, "glu32.dll"), fakePe());
    const exe = join(scratch("chain-exe"), "fettle.exe");
    writeFileSync(exe, fakePe({ dlls: ["SECUR32.dll", "KERNEL32.dll", "VCRUNTIME140.dll"], delayed: ["opengl32.dll"] }));
    assert.deepEqual(importChain(exe, sys, new Set(["kernel32.dll"])), ["cryptbase.dll", "glu32.dll", "kernel32.dll", "opengl32.dll", "secur32.dll", "sspicli.dll", "vcruntime140.dll"]);
  });
});

describe("runtimeFor", () => {
  it("picks the runtime DLLs a program needs, and the ones those need, from the runtime folder", () => {
    const crt = scratch("crt");
    writeFileSync(join(crt, "msvcp140.dll"), fakePe({ dlls: ["VCRUNTIME140.dll", "VCRUNTIME140_1.dll", "KERNEL32.dll"] }));
    writeFileSync(join(crt, "vcruntime140.dll"), fakePe({ dlls: ["KERNEL32.dll"] }));
    writeFileSync(join(crt, "vcruntime140_1.dll"), fakePe({ dlls: ["VCRUNTIME140.dll"] }));
    writeFileSync(join(crt, "concrt140.dll"), fakePe());
    const exe = join(scratch("crt-exe"), "fettle.exe");
    writeFileSync(exe, fakePe({ dlls: ["KERNEL32.dll", "MSVCP140.dll", "VCRUNTIME140.dll"] }));
    assert.deepEqual(runtimeFor(exe, crt).map((p) => p.slice(crt.length + 1)).sort(), ["msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll"]);
  });

  it("stops when the runtime folder lacks one", () => {
    const crt = scratch("crt-short");
    writeFileSync(join(crt, "vcruntime140.dll"), fakePe());
    const exe = join(scratch("crt-short-exe"), "fettle.exe");
    writeFileSync(exe, fakePe({ dlls: ["MSVCP140.dll"] }));
    assert.throws(() => runtimeFor(exe, crt), /fettle\.exe needs msvcp140\.dll/);
  });
});
