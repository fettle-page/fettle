// Only files that belong in the public repo may be tracked: no hidden folders or notes beyond the
// short list below. This also catches a file forced past .gitignore (`git add -f`).
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { describe, it } from "node:test";
import { readFileSync } from "node:fs";
import { realLookingSids, unexpectedPaths } from "../repo-files.mjs";

// Test data must use obviously fake identifiers: a real account SID once slipped into a test.
describe("account SIDs in tracked files", () => {
  it("allows only obviously fake ones (each number one repeated digit)", () => {
    assert.deepEqual(realLookingSids("HKU\\S-1-5-21-111111111-2222222222-333333333-1001\\Run and S-1-5-21-1-2-3-1001"), []);
    // Built at run time, so this file doesn't hold a real-looking SID itself.
    const random = ["S-1-5-21", "3623811015", "3361044348", "30300820", "1013"].join("-");
    assert.equal(realLookingSids(`HKU\\${random}\\Run`).length, 1);
  });

  it("holds for every file tracked in this repo", () => {
    const root = join(import.meta.dirname, "../../../..");
    const tracked = execFileSync("git", ["ls-files"], { cwd: root, encoding: "utf8" }).trim().split("\n");
    const bad = tracked.filter((p) => !/\.(png|ico|woff2?|ttf|wasm)$/i.test(p) && realLookingSids(readFileSync(join(root, p), "latin1")).length);
    assert.deepEqual(bad, []);
  });
});

describe("tracked files", () => {
  it("refuses hidden paths and Markdown files that aren't on the list", () => {
    assert.deepEqual(unexpectedPaths(["notes.md", "docs/plan.md", ".hidden/x.json", "apps/web/.secret", "src/lib.rs"]), ["notes.md", "docs/plan.md", ".hidden/x.json", "apps/web/.secret"]);
  });

  it("allows the repo's own dotfiles, the workflows, READMEs, SECURITY.md and the site's .well-known", () => {
    const ok = [".gitattributes", ".gitignore", ".github/workflows/ci.yml", "README.md", "worker/README.md", "SECURITY.md", "packages/brain/.gitignore", "apps/web/public/.well-known/security.txt", "spike/.cargo/config.toml"];
    assert.deepEqual(unexpectedPaths(ok), []);
    assert.deepEqual(unexpectedPaths(["docs/SECURITY.md"]), ["docs/SECURITY.md"]);
  });

  it("SECURITY.md gives the same contact as the site's security.txt", () => {
    const root = join(import.meta.dirname, "../../../..");
    const contact = /^Contact: mailto:(\S+)$/m.exec(readFileSync(join(root, "apps/web/public/.well-known/security.txt"), "utf8"))[1];
    assert.ok(readFileSync(join(root, "SECURITY.md"), "utf8").includes(contact), contact);
  });

  it("holds for every file tracked in this repo", () => {
    const tracked = execFileSync("git", ["ls-files"], { cwd: join(import.meta.dirname, "../../../.."), encoding: "utf8" }).trim().split("\n");
    assert.deepEqual(unexpectedPaths(tracked), []);
  });
});
