// What a Windows program needs to start: the DLLs named in its import tables, read straight from the
// file (no Visual Studio tools needed). build.mjs uses it to ship the Visual C++ runtime beside
// fettle.exe, and the tests use it to check that nothing a download needs is missing.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

/** The parts of a PE file (exe or dll) these helpers read: its data directories and a way to follow addresses. */
function parse(bytes) {
  const u16 = (o) => bytes.readUInt16LE(o);
  const u32 = (o) => bytes.readUInt32LE(o);
  if (bytes.length < 0x40 || u16(0) !== 0x5a4d) throw new Error("not a Windows program (no MZ header)");
  const pe = u32(0x3c);
  if (pe + 4 > bytes.length || u32(pe) !== 0x4550) throw new Error("not a Windows program (no PE header)");
  const sectionCount = u16(pe + 6);
  const optional = pe + 24;
  const optionalSize = u16(pe + 20);
  const directories = optional + (u16(optional) === 0x20b ? 112 : 96);
  const sections = [];
  for (let i = 0; i < sectionCount; i++) {
    const s = optional + optionalSize + i * 40;
    sections.push({ va: u32(s + 12), size: Math.max(u32(s + 8), u32(s + 16)), raw: u32(s + 20) });
  }
  const offset = (rva) => {
    const s = sections.find((s) => rva >= s.va && rva < s.va + s.size);
    if (!s) throw new Error(`address ${rva} is outside every section`);
    return rva - s.va + s.raw;
  };
  return { u16, u32, offset, directory: (i) => u32(directories + i * 8) };
}

/** The DLLs a PE file (exe or dll) imports, as written in the file, normal and delay-loaded. */
export function imports(bytes) {
  const { u32, offset, directory } = parse(bytes);
  const name = (rva) => {
    const start = offset(rva);
    return bytes.toString("latin1", start, bytes.indexOf(0, start));
  };
  const names = [];
  if (directory(1)) {
    for (let d = offset(directory(1)); u32(d + 12) !== 0; d += 20) names.push(name(u32(d + 12)));
  }
  if (directory(13)) {
    for (let d = offset(directory(13)); u32(d + 4) !== 0; d += 32) names.push(name(u32(d + 4)));
  }
  return names;
}

/** The DLLs a PE file loads only when first used (its delay-load imports). */
export function delayImports(bytes) {
  const { u32, offset, directory } = parse(bytes);
  const names = [];
  if (directory(13)) {
    for (let d = offset(directory(13)); u32(d + 4) !== 0; d += 32) {
      const start = offset(u32(d + 4));
      names.push(bytes.toString("latin1", start, bytes.indexOf(0, start)));
    }
  }
  return names;
}

/**
 * Where a 64-bit program's own DLLs may load from (its load configuration's DependentLoadFlags, set
 * with the linker's /DEPENDENTLOADFLAG), or null when it has no load configuration.
 */
export function loadFlags(bytes) {
  const { u16, u32, offset, directory } = parse(bytes);
  if (!directory(10)) return null;
  const config = offset(directory(10));
  return u32(config) >= 80 ? u16(config + 78) : null;
}

/** Whether a DLL comes with the Visual C++ runtime rather than with Windows. */
export function redistributable(dll) {
  return /^(vcruntime|msvcp|concrt|vccorlib|vcomp|mfc)\d/i.test(dll);
}

/** API sets (api-ms-win-*, ext-ms-*) are names Windows itself resolves, never files to ship. */
export function apiSet(dll) {
  return /^(api|ext)-ms-/i.test(dll);
}

/**
 * The Visual C++ runtime DLLs `exe` needs, and the ones those need in turn, as paths in `crtDir`
 * (the build machine's Microsoft.VC14x.CRT folder). Throws if one isn't there.
 */
export function runtimeFor(exe, crtDir) {
  const found = new Map();
  const queue = imports(readFileSync(exe)).filter(redistributable);
  while (queue.length) {
    const dll = queue.shift().toLowerCase();
    if (found.has(dll)) continue;
    const file = readdirSync(crtDir).find((f) => f.toLowerCase() === dll);
    if (!file) throw new Error(`${basename(exe)} needs ${dll}, which isn't in ${crtDir}`);
    found.set(dll, join(crtDir, file));
    queue.push(...imports(readFileSync(join(crtDir, file))).filter(redistributable));
  }
  return [...found.values()];
}

/**
 * Every DLL a program's imports lead to through `system32`, lower case: its own imports, theirs, and
 * so on. Known DLLs (`known`, which Windows always maps from System32 with their own dependencies)
 * and the runtime shipped beside the program are listed but not followed.
 */
export function importChain(exe, system32, known = new Set()) {
  const seen = new Set();
  const queue = imports(readFileSync(exe)).map((d) => d.toLowerCase());
  while (queue.length) {
    const dll = queue.shift();
    if (seen.has(dll) || apiSet(dll)) continue;
    seen.add(dll);
    const file = join(system32, dll);
    if (known.has(dll) || redistributable(dll) || !existsSync(file)) continue;
    queue.push(...imports(readFileSync(file)).map((d) => d.toLowerCase()));
  }
  return [...seen].sort();
}

/**
 * What a folder of programs would miss on a clean Windows: every DLL one of `files` imports that is
 * neither among them, nor an API set, nor one of Windows' own in `system32` (the Visual C++ runtime
 * counts as missing even when this PC happens to have it there).
 */
export function missing(files, system32) {
  const shipped = new Set(files.map((f) => basename(f).toLowerCase()));
  const gaps = new Set();
  for (const f of files) {
    for (const dll of imports(readFileSync(f))) {
      const name = dll.toLowerCase();
      if (shipped.has(name) || apiSet(name)) continue;
      if (!redistributable(name) && existsSync(join(system32, name))) continue;
      gaps.add(`${basename(f)} → ${dll}`);
    }
  }
  return [...gaps].sort();
}
