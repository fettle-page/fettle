// The Visual C++ runtime that ships beside fettle.exe. Skia's prebuilt renderer is built against the
// DLL runtime, and a clean Windows doesn't have it, so both downloads carry the DLLs fettle.exe needs
// (Microsoft allows this "app-local" copy). The packaged exe names them in its manifest, so Windows
// loads exactly those from its own folder and every other DLL from System32 only.
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, linkSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { importChain } from "./pe.mjs";

/** The newest x64 Microsoft.VC14x.CRT folder of a Visual Studio installation, with its version, or null. */
export function crtDirIn(installation) {
  const msvc = join(installation, "VC", "Redist", "MSVC");
  if (!existsSync(msvc)) return null;
  const versions = readdirSync(msvc)
    .filter((v) => /^\d+(\.\d+)+$/.test(v))
    .sort((a, b) => b.localeCompare(a, "en", { numeric: true }));
  for (const v of versions) {
    const x64 = join(msvc, v, "x64");
    const crt = existsSync(x64) && readdirSync(x64).find((d) => /^Microsoft\.VC\d+\.CRT$/.test(d));
    if (crt) return { dir: join(x64, crt), version: v };
  }
  return null;
}

/** The build machine's runtime folder: FETTLE_CRT_DIR, or the newest one Visual Studio's own locator finds. */
export function findCrtDir() {
  if (process.env.FETTLE_CRT_DIR) return process.env.FETTLE_CRT_DIR;
  const vswhere = join(process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)", "Microsoft Visual Studio", "Installer", "vswhere.exe");
  if (!existsSync(vswhere)) throw new Error("Visual Studio's vswhere.exe not found: set FETTLE_CRT_DIR to a Microsoft.VC14x.CRT folder");
  const installs = execFileSync(vswhere, ["-all", "-products", "*", "-property", "installationPath"], { encoding: "utf8" }).split(/\r?\n/).filter(Boolean);
  const found = newestCrt(installs.map(crtDirIn).filter(Boolean));
  if (!found) throw new Error(`No Visual C++ runtime folder in ${installs.join(", ") || "any Visual Studio installation"}: set FETTLE_CRT_DIR`);
  return found;
}

/** The folder of the newest runtime among several installations' ({dir, version}), or null. */
export function newestCrt(crts) {
  const sorted = [...crts].sort((a, b) => b.version.localeCompare(a.version, "en", { numeric: true }));
  return sorted[0]?.dir ?? null;
}

/** Windows' known DLLs from `reg query` output, lower case: they always load from System32. */
export function knownDllsIn(regOutput) {
  return new Set([...regOutput.matchAll(/\sREG_SZ\s+(\S+\.dll)\s*$/gim)].map((m) => m[1].toLowerCase()));
}

/** This PC's known DLLs. */
export function knownDlls() {
  return knownDllsIn(execFileSync("reg.exe", ["query", "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\KnownDLLs"], { encoding: "utf8" }));
}

/** The DLLs a program's manifest declares as its own files, lower case. */
export function manifestFiles(bytes) {
  const text = bytes.toString("latin1");
  return [...text.matchAll(/<file\s+name="([^"]+)"\s*\/?>/g)].map((m) => m[1].toLowerCase());
}

/**
 * Exit codes of programs started with Windows' "missing DLL" error boxes turned off (a broken copy
 * would otherwise stop on a message nobody is there to close). `runs` is [exe, args] pairs.
 */
export function exitCodes(runs, scratch) {
  // The runs go in a file: a long list would push the environment past what Windows allows.
  mkdirSync(scratch, { recursive: true });
  const list = join(scratch, "runs.json");
  writeFileSync(list, JSON.stringify(runs));
  const script = [
    "$ErrorActionPreference = 'Stop'",
    "Add-Type -Namespace W -Name E -MemberDefinition '[DllImport(\"kernel32.dll\")] public static extern uint SetErrorMode(uint mode);'",
    "[void][W.E]::SetErrorMode(0x8003)",
    "$i = 0",
    "foreach ($run in (Get-Content -Raw -LiteralPath $env:FETTLE_RUNS | ConvertFrom-Json)) {",
    "  try {",
    "    $psi = New-Object System.Diagnostics.ProcessStartInfo $run[0]",
    "    $psi.Arguments = $run[1]",
    "    $psi.UseShellExecute = $false",
    "    $p = [System.Diagnostics.Process]::Start($psi)",
    "    if ($p.WaitForExit(30000)) { \"${i}:$($p.ExitCode)\" } else { $p.Kill(); \"${i}:timeout\" }",
    "  } catch {}",
    "  $i++",
    "}",
  ].join("\n");
  const out = execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8", env: { ...process.env, FETTLE_RUNS: list } });
  rmSync(list, { force: true });
  return readExitCodes(out, runs.length);
}

/** "<run>:<exit code>" lines, as one code per run ("not started" for a run with no line). */
export function readExitCodes(out, count) {
  const codes = Array(count).fill("not started");
  for (const line of out.split(/\r?\n/)) {
    const m = /^(\d+):(.+)$/.exec(line.trim());
    if (m && Number(m[1]) < count) codes[Number(m[1])] = m[2];
  }
  return codes;
}

/**
 * Starts the packaged fettle.exe in `dir` with a Deep check file it refuses, so it stops (exit code 1)
 * once Windows has loaded everything, before any window or scan:
 *  - as packaged: it starts, so every DLL it needs is there;
 *  - beside a broken copy of the runtime: it doesn't start, so the copy beside it is the one it uses;
 *  - beside a planted copy of every DLL its imports lead to (other than Windows' known DLLs, which
 *    never load from elsewhere): it starts, and a real helper scan runs and writes its report too,
 *    so no DLL left beside fettle.exe (in Downloads, say) loads into it, before or during a scan.
 */
export function checkStarts(dir, runtime, scratch, system32 = join(process.env.SystemRoot ?? "C:\\Windows", "System32")) {
  const refused = "--deep-scan fettle-deep-0-0.json";
  const copy = (name, plant = []) => {
    const to = join(scratch, name);
    rmSync(to, { recursive: true, force: true });
    mkdirSync(to, { recursive: true });
    try {
      linkSync(join(dir, "fettle.exe"), join(to, "fettle.exe"));
    } catch {
      copyFileSync(join(dir, "fettle.exe"), join(to, "fettle.exe"));
    }
    for (const f of runtime) copyFileSync(join(dir, f), join(to, f));
    for (const f of plant) writeFileSync(join(to, f), "not a DLL");
    return join(to, "fettle.exe");
  };
  const known = knownDlls();
  const plantable = importChain(join(dir, "fettle.exe"), system32, known).filter((d) => !known.has(d) && !runtime.includes(d));
  const broken = copy("broken-runtime");
  writeFileSync(join(scratch, "broken-runtime", runtime[0]), "not a DLL");
  const planted = copy("planted", plantable);
  const report = join(scratch, "planted", "fettle-deep-0-0.json");
  const [asIs, withBroken, withPlanted, scanned] = exitCodes(
    [
      [join(dir, "fettle.exe"), refused],
      [broken, refused],
      [planted, refused],
      [planted, `--deep-scan "${report}"`],
    ],
    scratch,
  );
  try {
    if (asIs !== "1") throw new Error(`The packaged fettle.exe didn't start (exit code ${asIs}): a DLL it needs is missing`);
    if (withBroken === "1") throw new Error(`fettle.exe started with a broken ${basename(runtime[0])} beside it, so it doesn't use the runtime it ships with`);
    if (withPlanted !== "1" || scanned !== "0" || !existsSync(report)) {
      // Which ones: each planted alone.
      const runs = plantable.flatMap((d) => {
        const exe = copy(`one-${d}`, [d]);
        return [[exe, refused], [exe, `--deep-scan "${join(scratch, `one-${d}`, "fettle-deep-0-0.json")}"`]];
      });
      const codes = exitCodes(runs, scratch);
      const loaded = plantable.filter((_, i) => codes[2 * i] !== "1" || codes[2 * i + 1] !== "0");
      throw new Error(`fettle.exe loaded DLLs left beside it (exit codes ${withPlanted}, ${scanned}): ${loaded.join(", ") || "none alone, only together"}`);
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}
