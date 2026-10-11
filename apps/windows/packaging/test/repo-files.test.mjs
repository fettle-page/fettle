// Only files that belong in the public repo may be tracked: no hidden folders or notes beyond the
// short list below. This also catches a file forced past .gitignore (`git add -f`).
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { describe, it } from "node:test";
import { unexpectedPaths } from "../repo-files.mjs";

describe("tracked files", () => {
  it("refuses hidden paths and Markdown files that aren't on the list", () => {
    assert.deepEqual(unexpectedPaths(["notes.md", "docs/plan.md", ".hidden/x.json", "apps/web/.secret", "src/lib.rs"]), ["notes.md", "docs/plan.md", ".hidden/x.json", "apps/web/.secret"]);
  });

  it("allows the repo's own dotfiles, the workflows, READMEs and the site's .well-known", () => {
    const ok = [".gitattributes", ".gitignore", ".github/workflows/ci.yml", "README.md", "worker/README.md", "packages/brain/.gitignore", "apps/web/public/.well-known/security.txt", "spike/.cargo/config.toml"];
    assert.deepEqual(unexpectedPaths(ok), []);
  });

  it("holds for every file tracked in this repo", () => {
    const tracked = execFileSync("git", ["ls-files"], { cwd: join(import.meta.dirname, "../../../.."), encoding: "utf8" }).trim().split("\n");
    assert.deepEqual(unexpectedPaths(tracked), []);
  });
});
