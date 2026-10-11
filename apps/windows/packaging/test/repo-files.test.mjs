// Only files that belong in the public repo may be tracked: no hidden folders or notes beyond the
// short list below. This also catches a file forced past .gitignore (`git add -f`).
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { describe, it } from "node:test";
import { readFileSync } from "node:fs";
import { realLookingAutoLaunchCodes, realLookingEmails, realLookingProcessorIds, realLookingSerials, realLookingSids, unexpectedPaths } from "../repo-files.mjs";

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

/** Every tracked text file, as [path, text]. */
function trackedTexts() {
  const root = join(import.meta.dirname, "../../../..");
  const tracked = execFileSync("git", ["ls-files"], { cwd: root, encoding: "utf8" }).trim().split("\n");
  return tracked.filter((p) => !/\.(png|ico|woff2?|ttf|wasm)$/i.test(p)).map((p) => [p, readFileSync(join(root, p), "latin1")]);
}

describe("Entra ID account SIDs in tracked files", () => {
  it("allows only obviously fake ones", () => {
    assert.deepEqual(realLookingSids("HKU\\S-1-12-1-1111111111-2222222222-3333333333-4444444444\\Run"), []);
    const random = ["S-1-12-1", "3623811015", "1336104434", "3030082011", "2013431744"].join("-");
    assert.equal(realLookingSids(`HKU\\${random}\\Run`).length, 1);
    // Windows writes SIDs in either case.
    assert.equal(realLookingSids(random.toLowerCase()).length, 1);
  });
});

// Test data uses example addresses; third-party licence texts quote their authors' real ones.
describe("email addresses in tracked files", () => {
  it("allows example and project addresses only", () => {
    assert.deepEqual(realLookingEmails("mario.rossi@example.com, info@studio.example, a@b.example.co.uk, hello@fettle.page, 1+x@users.noreply.github.com, dev@contoso.com, pkg@1.0.0"), []);
    // Not addresses: an image at twice the size, a package version.
    assert.deepEqual(realLookingEmails("logo@2x.png and pkg@1.0.0-beta.rc"), []);
    // Built at run time, so this file doesn't hold a real-looking address itself.
    const real = ["mario.rossi", "gmail.com"].join("@");
    assert.deepEqual(realLookingEmails(`write to ${real}`), [real]);
  });

  it("holds for every file tracked in this repo outside the licence texts", () => {
    const bad = trackedTexts().filter(([p]) => !p.startsWith("apps/windows/packaging/notices/")).filter(([, text]) => realLookingEmails(text).length);
    assert.deepEqual(bad.map(([p]) => p), []);
  });
});

// Serials in test data must look made up: a marker such as FAKE or TEST, a short value, or one
// repeated character.
describe("serial numbers in tracked files", () => {
  it("allows only obviously fake ones", () => {
    assert.deepEqual(realLookingSerials('"SerialNumber": "<private>", "SerialNumber": "TES12345", "FruId": "F1AKES2E345678R", "VolumeSerialNumber": "ABCD1234", "SerialNumber": "S123", "UUID": "00000000"'), []);
    const real = JSON.stringify({ ["Serial" + "Number"]: ["S6B0NL0", "W123987K"].join("") });
    assert.equal(realLookingSerials(real).length, 1);
    // A marker counts only at the start: a real serial can hold "1234" anywhere.
    assert.equal(realLookingSerials(JSON.stringify({ ["Serial" + "Number"]: ["WD-WCC4E", "1234567"].join("") })).length, 1);
    // Rust and JavaScript fields, not just JSON keys.
    assert.equal(realLookingSerials(`serial: "${["S6B0NL0", "W123987K"].join("")}".into()`).length, 1);
    assert.deepEqual(realLookingSerials('serial: "FAKE0001".into(), serial_number: Some("TEST-1".into())'), []);
  });

  it("holds for every file tracked in this repo", () => {
    assert.deepEqual(trackedTexts().filter(([, text]) => realLookingSerials(text).length).map(([p]) => p), []);
  });
});

// A dump from the test kit once carried a real CPU ProcessorId.
describe("CPU ProcessorIds in tracked files", () => {
  it("allows only placeholders and one repeated character", () => {
    assert.deepEqual(realLookingProcessorIds('"ProcessorId": "<processor-id>", "ProcessorId":"<private>", "ProcessorId": "0000000000000000"'), []);
    // Built at run time, so this file doesn't hold a real-looking ProcessorId itself.
    const random = JSON.stringify({ ["Processor" + "Id"]: ["178BFBFF", "00A20F12"].join("") });
    assert.equal(realLookingProcessorIds(random).length, 1);
  });

  it("holds for every file tracked in this repo", () => {
    assert.deepEqual(trackedTexts().filter(([, text]) => realLookingProcessorIds(text).length).map(([p]) => p), []);
  });
});

// Edge and Chrome name their startup entries after a hash of the browser profile's path, which
// holds the account name.
describe("browser startup codes in tracked files", () => {
  it("allows only obviously fake ones", () => {
    assert.deepEqual(realLookingAutoLaunchCodes("MicrosoftEdgeAutoLaunch_0123456789ABCDEF0123456789ABCDEF and MicrosoftEdgeAutoLaunch_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"), []);
    const random = ["5D0C93F1", "2B7E44A0", "9C1F08E3", "D47A6B25"].join("");
    assert.equal(realLookingAutoLaunchCodes(`MicrosoftEdgeAutoLaunch_${random}`).length, 1);
    assert.equal(realLookingAutoLaunchCodes(`GoogleChromeAutoLaunch_${random}`).length, 1);
  });

  it("holds for every file tracked in this repo", () => {
    assert.deepEqual(trackedTexts().filter(([, text]) => realLookingAutoLaunchCodes(text).length).map(([p]) => p), []);
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
