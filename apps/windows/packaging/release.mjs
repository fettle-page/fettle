// Small helpers for releases, used by build.mjs and the release workflow.
//   node apps/windows/packaging/release.mjs check   stops unless the pushed tag (GITHUB_REF_NAME)
//                                                    is v<version> from apps/windows/Cargo.toml
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/** The app's version, the only one there is: apps/windows/Cargo.toml. */
export function appVersion() {
  const cargo = readFileSync(join(here, "..", "Cargo.toml"), "utf8");
  const version = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
  if (!version) throw new Error("apps/windows/Cargo.toml has no version");
  return version;
}

/** Throws unless the tag is exactly v<version>, so a release can't go out under the wrong number. */
export function checkTag(tag, version) {
  if (tag !== `v${version}`) {
    throw new Error(`The tag ${JSON.stringify(tag ?? "")} doesn't match the app's version ${version} (apps/windows/Cargo.toml). Tag the release v${version}, or change the version first.`);
  }
}

/** SHA256SUMS.txt in sha256sum's format, so `sha256sum -c` and PowerShell users can both check a download. */
export function sums(files) {
  return files
    .map((f) => [basename(f), createHash("sha256").update(readFileSync(f)).digest("hex")])
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, hex]) => `${hex}  ${name}\n`)
    .join("");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1] && process.argv[2] === "check") {
  try {
    checkTag(process.env.GITHUB_REF_NAME, appVersion());
    console.log(`Tag ${process.env.GITHUB_REF_NAME} matches the app's version.`);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
