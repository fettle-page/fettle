// The installer script and the build can't run here without Inno Setup, so these read them as text.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";

const dir = join(import.meta.dirname, "..");
const iss = readFileSync(join(dir, "fettle.iss"), "utf8");
const build = readFileSync(join(dir, "build.mjs"), "utf8");

describe("fettle.iss", () => {
  it("keeps the installer's identity, so upgrades stay upgrades", () => {
    assert.match(iss, /^AppId=\{\{9404A274-E009-4582-B280-0BE4327BB7A1\}$/m);
  });

  it("installs Fettle's licence and the third-party notices beside the exe", () => {
    assert.match(iss, /^#ifndef NoticesDir\s*\n\s*#error/m);
    assert.match(iss, /^Source: "\{#NoticesDir\}\\LICENSE\.txt"; DestDir: "\{app\}"/m);
    assert.match(iss, /^Source: "\{#NoticesDir\}\\THIRD-PARTY-NOTICES\.txt"; DestDir: "\{app\}"/m);
  });
});

// SignPath's terms: software that contacts other systems on its own describes it in a privacy
// policy, shows that during installation, and offers an installation option to turn it off.
describe("fettle.iss: privacy during installation", () => {
  const note = (lang) => readFileSync(join(dir, "privacy", `privacy-${lang}.txt`));

  it("shows the privacy note before installing, in English and Italian", () => {
    assert.match(iss, /^Name: "en"; MessagesFile: "compiler:Default\.isl"; InfoBeforeFile: "privacy\\privacy-en\.txt"$/m);
    assert.match(iss, /^Name: "it"; MessagesFile: "compiler:Languages\\Italian\.isl"; InfoBeforeFile: "privacy\\privacy-it\.txt"$/m);
  });

  it("writes the notes so Inno Setup reads them right: UTF-8 with a BOM, Windows line ends", () => {
    for (const lang of ["en", "it"]) {
      const bytes = note(lang);
      assert.deepEqual([...bytes.subarray(0, 3)], [0xef, 0xbb, 0xbf], lang);
      const text = bytes.toString("utf8");
      assert.ok(!/[^\r]\n/.test(text), `${lang}: every line ends in CRLF`);
    }
  });

  it("says what leaves the PC: only the update check, and a receipt you choose to share", () => {
    const en = note("en").toString("utf8");
    const it_ = note("it").toString("utf8");
    assert.match(en, /update check/);
    assert.match(en, /nothing about your PC/);
    assert.match(en, /share/);
    assert.match(en, /https:\/\/fettle\.page\/privacy/);
    assert.match(it_, /controllo degli aggiornamenti/);
    assert.match(it_, /nulla sul tuo PC/);
    assert.match(it_, /condividi/);
    assert.match(it_, /https:\/\/fettle\.page\/it\/privacy/);
  });

  it("offers the update check as a box, ticked by default, in both languages", () => {
    const task = /^Name: "updatecheck"; Description: "\{cm:UpdateCheck\}"; GroupDescription: "\{cm:UpdatesGroup\}"(.*)$/m.exec(iss);
    assert.ok(task, "no updatecheck task");
    assert.doesNotMatch(task[1], /unchecked/);
    for (const key of ["UpdateCheck", "UpdatesGroup"]) {
      assert.match(iss, new RegExp(`^en\\.${key}=.+$`, "m"));
      assert.match(iss, new RegExp(`^it\\.${key}=.+$`, "m"));
    }
  });

  it("turns the app's update check off when the box is cleared, but never undoes a user's choice", () => {
    assert.match(iss, /not WizardIsTaskSelected\('updatecheck'\)/);
    assert.match(iss, /\{localappdata\}\\Fettle\\settings\.json/);
    // Only when the user hasn't chosen yet: no settings, or settings without "updates".
    assert.match(iss, /Pos\('"updates"', Text\) = 0/);
    assert.match(iss, /'\{"updates":false\}'/);
    // A choice made in Settings ("updates":true) is never turned back off by an upgrade.
    assert.doesNotMatch(iss, /'"updates":true', '"updates":false'/);
  });

  it("an all-users install leaves per-user settings alone (they'd land in the installing account's profile)", () => {
    const step = /procedure CurStepChanged[\s\S]*?\nend;/.exec(iss)[0];
    assert.match(step, /not IsAdminInstallMode/);
  });

  it("for all users, a cleared box turns the update check off for every account on the PC", () => {
    // HKLM\Software\Fettle\UpdateCheck = 0, written only in an all-users install with the box cleared,
    // and removed with Fettle.
    const reg = /^Root: HKLM; Subkey: "Software\\Fettle"; ValueType: dword; ValueName: "UpdateCheck"; ValueData: 0; (.*)$/m.exec(iss);
    assert.ok(reg, "no machine-wide UpdateCheck value");
    assert.match(reg[1], /Tasks: not updatecheck/);
    assert.match(reg[1], /Check: IsAdminInstallMode/);
    assert.match(reg[1], /uninsdeletevalue/);
    // Installing again for all users with the box ticked takes the machine-wide "off" away.
    assert.match(iss, /RegDeleteValue\(HKLM, 'Software\\Fettle', 'UpdateCheck'\)/);
  });
});

describe("fettle.iss: uninstalling for all users", () => {
  it("finds every account's Fettle folder from Windows' list of user profiles", () => {
    assert.match(iss, /RegGetSubkeyNames\(HKLM, 'SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList'/);
    assert.match(iss, /'ProfileImagePath'/);
    assert.match(iss, /'AppData\\Local\\Fettle'/);
  });

  it("asks once for all of them, in both languages, and says which folders it couldn't remove", () => {
    for (const lang of ["en", "it"]) {
      assert.match(iss, new RegExp(`^${lang}\\.RemoveDataAll=.*%1`, "m"));
      assert.match(iss, new RegExp(`^${lang}\\.RemoveDataLeft=.+`, "m"));
    }
    assert.match(iss, /if IsAdminInstallMode then/);
    assert.match(iss, /CustomMessage\('RemoveDataLeft'\)/);
  });
});

describe("build.mjs", () => {
  it("tells portable users how to remove Fettle and how to turn off the update check", () => {
    assert.match(build, /delete this folder together with fettle\.exe/);
    assert.match(build, /elimina questa cartella insieme a fettle\.exe/);
    assert.match(build, /Settings/);
  });

  it("writes the notices and Fettle's licence, and hands their folder to Inno Setup", () => {
    assert.match(build, /writeNotices\(.*, \{ skiaProfile: "release" \}\);/);
    assert.match(build, /LICENSE\.txt/);
    assert.match(build, /\/DNoticesDir=/);
  });

  it("writes SHA256SUMS.txt for the installer and the zip", () => {
    assert.match(build, /writeFileSync\(join\(out, "SHA256SUMS\.txt"\), sums\(\[installer, zip\]\)\)/);
  });

  it("puts both files in the portable zip, beside fettle.exe", () => {
    const tarCall = /execFileSync\(tar, \[([^\]]*)\]/.exec(build)?.[1] ?? "";
    for (const f of ["fettle.exe", "LICENSE.txt", "THIRD-PARTY-NOTICES.txt", "Fettle data"]) assert.ok(tarCall.includes(`"${f}"`), `${f} not in the zip`);
  });
});

// Skia's prebuilt renderer needs the Visual C++ runtime, which a clean Windows doesn't have.
describe("the Visual C++ runtime in both downloads", () => {
  it("builds the release with the manifest that loads the runtime from beside fettle.exe", () => {
    assert.match(build, /FETTLE_APP_LOCAL_RUNTIME: "1"/);
  });

  it("ships the DLLs fettle.exe needs, found from its import table, in the zip and the installer", () => {
    assert.match(build, /runtimeFor\(/);
    const tarCall = /execFileSync\(tar, \[([^\]]*)\]/.exec(build)?.[1] ?? "";
    assert.match(tarCall, /\.\.\.runtimeNames/);
    assert.match(build, /\/DRuntimeDir=/);
    assert.match(iss, /^#ifndef RuntimeDir\s*\n\s*#error/m);
    assert.match(iss, /^Source: "\{#RuntimeDir\}\\\*\.dll"; DestDir: "\{app\}"; Flags: ignoreversion$/m);
  });

  it("stops unless the packaged exe starts on what it ships, and only that", () => {
    assert.match(build, /missing\(/);
    assert.match(build, /checkStarts\(/);
    assert.match(build, /loadFlags\(/);
  });
});
