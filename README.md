# Fettle

A free, open-source check-up for Windows PCs. It reads what Windows already knows about the disks,
battery, memory, Windows itself, startup apps and graphics, and prints it as a receipt you can
actually read: a plain-words verdict on every line, with the numbers underneath.

Fettle only reads. It doesn't clean, "boost" or change anything, and the app sends nothing on its
own: the only automatic request is the update check, which can be turned off.

**Status:** 1.0.0, the first release. The downloads aren't code-signed yet, so Windows may warn before it runs them; [the code signing page](https://fettle.page/code-signing) explains why and what comes next.

## Download

Fettle runs on Windows 10 and 11. Get the installer or the portable zip from [fettle.page/download](https://fettle.page/download) or the [releases](https://github.com/fettle-page/fettle/releases). Each release lists the SHA-256 of its files in `SHA256SUMS.txt`, and is built by GitHub Actions from the tagged source (see [the code signing page](https://fettle.page/code-signing)).

## What's in this repo

| Path | What it is |
|---|---|
| `crates/fettle-facts` | The report format, in Rust (the app's collectors produce it). JSON Schema and zod are generated from it. |
| `crates/fettle-collect` | The Windows scan: reads every source in parallel, maps it to the report and hides private details. |
| `crates/fettle-verdicts` | The verdict engine: facts in, plain-words verdicts out (English and Italian). |
| `crates/fettle-receipt` | The receipt everyone shows: verdicts, wording and numbers, already in one language. Its copy lives in `copy/*.json`. |
| `crates/fettle-wasm` | The receipt as WebAssembly, for the website build and the share Worker. |
| `apps/windows` | The Windows app (Rust and Slint), and in `packaging/` the installer, the portable zip and the licence notices. |
| `packages/brain` | Loads the WebAssembly receipt in Node and in the Worker; the receipt's TypeScript types, generated from Rust. |
| `packages/report-schema` | The report types and validation for TypeScript (generated from the Rust types). |
| `packages/receipt-ui` | Draws the receipt (React) on the website and share pages. |
| `packages/share-client` | The app's side of share links: upload and delete, tested against the Worker. |
| `apps/web` | The website (Astro, static), including the share page. |
| `worker` | A Cloudflare Worker: serves the website and the share-link API (R2 storage). |
| `spike` | A throwaway Rust CLI used to test Windows data sources. |
| `assets/brand` | Logo and app icon. |
| `.github/workflows` | CI runs every test on each push; a version tag builds the downloads and drafts a GitHub release. |

## Develop

Needs Node 24 and npm, and a stable Rust toolchain (MSVC on Windows) with the WebAssembly target:
the website and the share Worker build their receipts with the same Rust code as the app.

```sh
rustup target add wasm32-unknown-unknown
npm install
npm test                      # every package: unit tests, type checks, site build checks
npm run parity:goldens        # after changing a verdict rule or the copy: refresh the expected receipts
npm run dev:share             # the website + share Worker on http://localhost:8787
node worker/scripts/seed.mjs  # upload a sample report to the local Worker and print its links
cargo test --workspace        # the Rust crates
cargo run -p fettle-collect   # scan this PC and print the report (Windows)
npm run kit                   # build target/fettle-test-kit.zip to run on other PCs
npm run package:windows       # the installer, portable zip and SHA256SUMS.txt in target/package (needs Inno Setup 6)
```

Everything runs locally. The Worker uses local simulations of R2, rate limiting and Durable Objects.

## Releasing

1. Set the version in `apps/windows/Cargo.toml` (the app and its downloads read it there), run cargo once so `Cargo.lock` follows, and write the version's notes in `apps/web/src/content/changelog.ts`.
2. Push the tag `v<version>`. The Release workflow checks the tag matches, runs every test, builds the downloads with a build attestation, and drafts a GitHub release.
3. Publish the draft. Then date the notes, set `published` in `apps/web/src/release.ts` to the version, and deploy the site: its download links and `/app/latest.json` (which tells the app a new version is out) follow `published`, never an unpublished version.

## Privacy

- Reports leave out serial numbers, the PC's name, usernames and network addresses. Product keys are never read.
- A share link uploads only the receipt you see, when you ask. Links expire after 7 days, 30 days or 1 year, can have a password (stored only as a hash), and can be deleted with a private link.
- The website uses no cookies, no analytics and no third-party requests. Fonts are self-hosted.

## Contact and security

- Questions, or an antivirus that flags Fettle: [hello@fettle.page](mailto:hello@fettle.page), or open an issue.
- A security problem (in the app, the website or share links): please write to [security@fettle.page](mailto:security@fettle.page) instead of opening a public issue.
- A share link being misused: [abuse@fettle.page](mailto:abuse@fettle.page).

## Licence

MIT (see `LICENSE`). The fonts are under the SIL Open Font License; their licence files are in `apps/web/public/fonts`. The Windows downloads carry `THIRD-PARTY-NOTICES.txt`, the licences of everything built into the app, written at packaging time by `apps/windows/packaging/notices.mjs`.
