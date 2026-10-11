// Builds Fettle's two downloads from one release build, into target/package/:
//   Fettle-Setup-<version>.exe      the installer (Inno Setup 6)
//   Fettle-<version>-portable.zip   fettle.exe plus its "Fettle data" folder: a copy that keeps
//                                   everything beside itself and nothing on the PC it runs on.
//   SHA256SUMS.txt                  the SHA-256 of both, to check a download against
// Both downloads carry LICENSE.txt (Fettle's MIT licence) and THIRD-PARTY-NOTICES.txt (see notices.mjs),
// and the Visual C++ runtime DLLs fettle.exe needs (see runtime.mjs).
// Run: npm run package:windows. Needs Inno Setup 6 (winget install JRSoftware.InnoSetup); set ISCC
// to its ISCC.exe if it isn't in one of the usual places.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync, copyFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { writeNotices } from "./notices.mjs";
import { loadFlags, missing, runtimeFor } from "./pe.mjs";
import { checkStarts, findCrtDir, manifestFiles } from "./runtime.mjs";
import { sums } from "./release.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const version = /^version\s*=\s*"([^"]+)"/m.exec(readFileSync(join(here, "../Cargo.toml"), "utf8"))[1];
const out = join(root, "target", "package");
const exeDir = join(root, "target", "release");

function find(candidates, what) {
  const hit = candidates.find((p) => p && existsSync(p));
  if (!hit) throw new Error(`${what} not found (looked in: ${candidates.filter(Boolean).join(", ")})`);
  return hit;
}

const cargo = find([process.env.CARGO, join(homedir(), ".cargo", "bin", "cargo.exe")], "cargo");
const iscc = find(
  [process.env.ISCC, join(process.env.LOCALAPPDATA ?? "", "Programs", "Inno Setup 6", "ISCC.exe"), "C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe", "C:\\Program Files\\Inno Setup 6\\ISCC.exe"],
  "Inno Setup's ISCC.exe",
);

console.log(`Fettle ${version}: release build`);
execFileSync(cargo, ["build", "--release", "-p", "fettle-windows"], { cwd: root, stdio: "inherit", env: { ...process.env, FETTLE_APP_LOCAL_RUNTIME: "1" } });
const exe = join(exeDir, "fettle.exe");

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

console.log("Visual C++ runtime");
const exeBytes = readFileSync(exe);
if (loadFlags(exeBytes) !== 0x800) throw new Error("fettle.exe may load DLLs from its own folder: build.rs sets /DEPENDENTLOADFLAG:0x800");
const runtime = runtimeFor(exe, findCrtDir());
const runtimeNames = runtime.map((p) => basename(p).toLowerCase());
const declared = manifestFiles(exeBytes);
const undeclared = runtimeNames.filter((n) => !declared.includes(n));
if (undeclared.length) throw new Error(`fettle.exe needs ${undeclared.join(", ")}, which its manifest doesn't name: add them to APP_LOCAL_RUNTIME in build.rs`);
const runtimeDir = join(out, "runtime");
mkdirSync(runtimeDir);
for (const p of runtime) {
  copyFileSync(p, join(runtimeDir, basename(p).toLowerCase()));
  // Beside the release build too, so target/release/fettle.exe runs where it was built.
  copyFileSync(p, join(exeDir, basename(p).toLowerCase()));
}

console.log("licences");
const notices = join(out, "notices");
writeNotices(join(notices, "THIRD-PARTY-NOTICES.txt"), { skiaProfile: "release" });
copyFileSync(join(root, "LICENSE"), join(notices, "LICENSE.txt"));

console.log("installer");
execFileSync(iscc, ["/Q", `/DAppVersion=${version}`, `/DExeDir=${exeDir}`, `/DRuntimeDir=${runtimeDir}`, `/DNoticesDir=${notices}`, `/DOutDir=${out}`, join(here, "fettle.iss")], { stdio: "inherit" });

console.log("portable zip");
const stage = join(out, "portable");
mkdirSync(join(stage, "Fettle data"), { recursive: true });
copyFileSync(exe, join(stage, "fettle.exe"));
for (const n of runtimeNames) copyFileSync(join(runtimeDir, n), join(stage, n));
const gaps = missing([join(stage, "fettle.exe"), ...runtimeNames.map((n) => join(stage, n))], join(process.env.SystemRoot ?? "C:\\Windows", "System32"));
if (gaps.length) throw new Error(`The downloads would miss DLLs a clean Windows doesn't have: ${gaps.join(", ")}`);
checkStarts(stage, runtimeNames, join(root, "target", "tmp", "runtime-check"));
for (const f of ["LICENSE.txt", "THIRD-PARTY-NOTICES.txt"]) copyFileSync(join(notices, f), join(stage, f));
writeFileSync(
  join(stage, "Fettle data", "README.txt"),
  [
    "Fettle keeps its settings, the remembered check and your shared links in this folder,",
    "so it leaves nothing on the PC it runs on. Keep it next to fettle.exe.",
    "To remove Fettle, delete this folder together with fettle.exe.",
    "The only thing Fettle sends on its own is the update check (the newest version number,",
    "nothing about your PC). Turn it off in Settings. Privacy policy: https://fettle.page/privacy",
    "",
    "Fettle tiene qui le impostazioni, l'ultimo controllo ricordato e i link condivisi,",
    "così non lascia nulla sul PC su cui gira. Tienila accanto a fettle.exe.",
    "Per rimuovere Fettle, elimina questa cartella insieme a fettle.exe.",
    "L'unica cosa che Fettle invia da solo è il controllo degli aggiornamenti (il numero",
    "dell'ultima versione, nulla sul tuo PC). Si disattiva nelle Impostazioni.",
    "Informativa sulla privacy: https://fettle.page/it/privacy",
    "",
  ].join("\r\n"),
);
const zip = join(out, `Fettle-${version}-portable.zip`);
// Windows' own tar (built in since Windows 10) writes zips.
const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
execFileSync(tar, ["-a", "-c", "-f", zip, "-C", stage, "fettle.exe", ...runtimeNames, "LICENSE.txt", "THIRD-PARTY-NOTICES.txt", "Fettle data"], { stdio: "inherit" });
rmSync(stage, { recursive: true, force: true });
rmSync(notices, { recursive: true, force: true });
rmSync(runtimeDir, { recursive: true, force: true });

const installer = join(out, `Fettle-Setup-${version}.exe`);
writeFileSync(join(out, "SHA256SUMS.txt"), sums([installer, zip]));

console.log(`done: ${out}`);
