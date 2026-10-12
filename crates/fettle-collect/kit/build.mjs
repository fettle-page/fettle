// Builds the test kit for other PCs: fettle-collect (the app's own scan, which writes the report),
// with the C runtime linked in so it runs on a fresh Windows without the VC++ redistributable.
// Output: target/usb-kit/ and target/fettle-test-kit.zip. Run: npm run kit
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const root = fileURLToPath(new URL("../../../", import.meta.url));
const buildDir = join(root, "target", "usb-build");
const kitDir = join(root, "target", "usb-kit");
const zip = join(root, "target", "fettle-test-kit.zip");
const userCargo = join(homedir(), ".cargo", "bin", "cargo.exe");
const cargo = existsSync(userCargo) ? userCargo : "cargo";
const env = { ...process.env, RUSTFLAGS: "-C target-feature=+crt-static", CARGO_TARGET_DIR: buildDir };

execFileSync(cargo, ["build", "--release", "-p", "fettle-collect"], { cwd: root, env, stdio: "inherit" });

rmSync(kitDir, { recursive: true, force: true });
mkdirSync(kitDir, { recursive: true });
const exes = ["fettle-collect.exe"];
const docs = ["run-fettle-test.bat", "README.txt"];
for (const exe of exes) copyFileSync(join(buildDir, "release", exe), join(kitDir, exe));
for (const file of docs) copyFileSync(join(here, file), join(kitDir, file));

rmSync(zip, { force: true });
// Windows' own bsdtar writes zip archives with -a (Git's GNU tar, often first on PATH, can't).
const tar = join(process.env.SystemRoot ?? "C:/Windows", "System32", "tar.exe");
// Name the files: zipping "." stores "./" entries, which WinRAR can't extract and Explorer hides.
const files = [...exes, ...docs];
execFileSync(tar, ["-a", "-c", "-f", zip, "-C", kitDir, ...files], { stdio: "inherit" });
const entries = execFileSync(tar, ["-t", "-f", zip], { encoding: "utf8" }).split(/\r?\n/).filter(Boolean);
if (entries.sort().join("|") !== [...files].sort().join("|")) throw new Error(`unexpected zip entries: ${entries.join(", ")}`);
console.log(`kit: ${kitDir}\nzip: ${zip}`);
