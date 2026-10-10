// Builds the Rust receipt (crates/fettle-wasm) for wasm32 and puts it in dist/fettle.wasm, where
// the Node and Worker loaders read it. Run by `npm run build` here and before the web and Worker builds.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const repoRoot = here("../../../");

/** A missing wasm32 target is the likely failure on a new machine: say what to run instead of cargo's dump. */
export function explainBuildFailure(stderr) {
  const missingTarget = /wasm32-unknown-unknown.*may not be installed|can't find crate for `(core|std)`/.test(stderr);
  return missingTarget ? "Building the website needs Rust's wasm32 target: run  rustup target add wasm32-unknown-unknown" : stderr;
}

function main() {
  // rustup's bin folder isn't always on PATH for Node.
  const env = { ...process.env, PATH: `${join(homedir(), ".cargo", "bin")}${delimiter}${process.env.PATH}` };
  const args = ["build", "-q", "-p", "fettle-wasm", "--target", "wasm32-unknown-unknown", "--profile", "wasm-release"];
  const run = spawnSync("cargo", args, { cwd: repoRoot, env, encoding: "utf8" });
  if (run.error) {
    console.error(`Building the website needs Rust (cargo wasn't found): ${run.error.message}`);
    process.exit(1);
  }
  if (run.status !== 0) {
    console.error(explainBuildFailure(run.stderr));
    process.exit(1);
  }
  mkdirSync(here("../dist"), { recursive: true });
  copyFileSync(join(repoRoot, "target/wasm32-unknown-unknown/wasm-release/fettle_wasm.wasm"), here("../dist/fettle.wasm"));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
