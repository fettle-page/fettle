// The release switch: everything the site needs to know about going public, in one place.
//
// Release day, in order:
//   1. Set `repo` (the public GitHub repo) and `contact` (a public email address). Safe any time:
//      they only add the source and contact links.
//   2. Push the version tag; the Release workflow drafts the GitHub release. Publish it.
//   3. Date the version's entry in src/content/changelog.ts and set `published` to that version.
//   4. Build and deploy the site.
// For a later version, the same 2–4: until `published` changes, the site and its /app/latest.json
// keep pointing at the last published version, whatever apps/windows/Cargo.toml says.
// Once the downloads are signed, set `signed: true`. codeSigning.signed holds SignPath's attribution, written
// for an application there (declined in October 2026, to retry later): if another service signs, use its wording.
//
// The download page runs checkRelease at build time, so a half-set switch fails the build.
import type { Release as ChangelogEntry } from "./content/changelog.ts";

export type Release = {
  /** The public repo, e.g. "https://github.com/<owner>/fettle". Null until it's public. */
  repo: string | null;
  /** A public email address for questions and reports. Null until there is one. */
  contact: string | null;
  /** The version whose GitHub release is public, e.g. "1.0.0": the site links its files. */
  published: string | null;
  /** The downloads are code-signed. */
  signed: boolean;
};

export const release: Release = { repo: "https://github.com/fettle-page/fettle", contact: "hello@fettle.page", published: "1.0.4", signed: false };

// Owner and repo names: letters, digits, - _ and dots, but not only dots, and no ".git" on the end.
const GITHUB_REPO = /^https:\/\/github\.com\/(?!\.+\/)[\w.-]+\/(?!\.+$)[\w.-]+$/;
const EMAIL = /^[^\s@<>"]+@[^\s@<>"]+\.[a-z]{2,}$/i;
const VERSION = /^\d+\.\d+\.\d+$/;

/** Throws a plain message when the switch is set in a way the site can't honour. */
export function checkRelease(r: Release, changelog: ChangelogEntry[]): void {
  if (r.repo !== null && (!GITHUB_REPO.test(r.repo) || r.repo.endsWith(".git"))) {
    throw new Error(`release.repo must be a GitHub repo page like https://github.com/<owner>/fettle, not ${JSON.stringify(r.repo)}`);
  }
  if (r.contact !== null && !EMAIL.test(r.contact)) throw new Error(`release.contact must be an email address, not ${JSON.stringify(r.contact)}`);
  if (r.published === null) return;
  if (!VERSION.test(r.published)) throw new Error(`release.published must be a version like 1.0.0, not ${JSON.stringify(r.published)}`);
  if (!r.repo) throw new Error("release.published is set but release.repo is empty: the download links need the repo");
  if (!r.contact) throw new Error("release.published is set but release.contact is empty: the privacy page promises a contact");
  if (!changelog.some((e) => e.version === r.published && e.date)) throw new Error(`release.published is ${r.published} but the changelog has no dated entry for it`);
}

/** The version the site offers and the app's update check reads: the published one, or the app's own before any release. */
export const latestVersion = (r: Release, appVersion: string): string => r.published ?? appVersion;

/** Where the site links for the source and for questions: null until each exists. */
export function links(r: Release): { source: string | null; contact: string | null; email: string | null } {
  return { source: r.repo, contact: r.contact ? `mailto:${r.contact}` : null, email: r.contact };
}

export type Downloads = { version: string; installer: string; portable: string; checksums: string; all: string };

/** The published version's files on GitHub Releases, or null before anything is published. */
export function downloads(r: Release): Downloads | null {
  if (!r.published || !r.repo) return null;
  const v = r.published;
  const base = `${r.repo}/releases/download/v${v}/`;
  return {
    version: v,
    installer: `${base}Fettle-Setup-${v}.exe`,
    portable: `${base}Fettle-${v}-portable.zip`,
    checksums: `${base}SHA256SUMS.txt`,
    all: `${r.repo}/releases`,
  };
}
