# Fettle share Worker

One Cloudflare Worker serves the share links:

- `POST /api/reports` takes `{ expires?, password?, report }`. It validates the report against `@fettle/report-schema` (unknown fields are dropped; every text field, number and list has its own limit from the Rust types, and anything outside them is refused with `invalid_report`), stores it in R2 and returns `{ id, url, deleteUrl, deleteToken, expiresAt, locked }`.
  - **Limits:** uploads are capped at 64 KiB (the largest real report is about 4 KB), 10 per minute per IP, and 100 a day (UTC) per IPv4 address or IPv6 /48, counted in a Durable Object (`UploadBudget`) named by a SHA-256 of the address and the day. Share pages (`/r/:id`), `GET /api/reports/:id` and `DELETE /api/reports/:id` share a cap of 60 a minute per IP, and unlock requests at 20 a minute per IP, checked before anything is read from R2.
  - **Expiry:** `7d`, `30d` (default) or `1y`.
- `GET /api/reports/:id` returns `{ report, expiresAt }`. A password-protected report answers `401 password_required` instead.
- `POST /api/reports/:id/unlock` takes `{ password, locale? }` and returns the finished receipt, built here with the Rust brain in that language (`en` unless `locale` is `it`): `{ model, expiresAt }`. Wrong guesses are capped per report at 10 a minute, per visitor (IPv4 address, or IPv6 /48, since one person can easily get a /48) at 20 a day (UTC), and per report at 500 a day across all visitors, counted globally in a Durable Object (`UnlockGuard`). Visitors are stored only as a hash salted with a random key that is replaced, with every count, when the day ends. Past a daily cap the answer is a 429 whose `retry-after` runs to midnight UTC, and the share page says to try again tomorrow. Requests are also capped at 20 a minute per IP. Passwords are 8–128 characters. A right password also returns a 24-hour `pass` and `passExpiresAt` (`UnlockGuard` keeps only a hash); sending `{ pass, locale? }` instead reopens the receipt without counting as a guess and renews the pass for another 24 hours (never past the report's expiry), returning it with its new `passExpiresAt`, so a burst of wrong guesses can't lock out people who already opened it.
- `DELETE /api/reports/:id` deletes a report. It needs `Authorization: Bearer <deleteToken>`.
- `/r/:id` serves the share page (`apps/web`, page `/share`) with Open Graph tags filled in. For a readable report the Worker builds the receipt model once, draws the page with React (`@fettle/web/share-render`) and inlines the model, so the receipt arrives with the HTML and the browser hydrates it. The language comes from `?lang=` or `Accept-Language`. `/r/:id/delete` serves the delete page.
- The share page's CSP allows its inline stylesheet and early theme script by SHA-256; the Worker reads the hashes from the built shell (cached per isolate).

## How it stores things

- **IDs** are one letter for the expiry (`w`, `m`, `y`) followed by 16 random characters.
- **R2 keys** are `<expiry>/<id>`.
- **Custom metadata** holds the expiry, a SHA-256 hash of the delete token and, for protected reports, a PBKDF2 hash of the password. Neither the token nor the password is stored.
- **Expiry** is checked by the Worker on every read; R2 lifecycle rules only clean up afterwards.
- **URLs** are built from the request's own origin, so the code has no domain in it.

## Local development

```sh
npm run dev:share                 # builds apps/web (the whole site), then runs wrangler dev on http://localhost:8787
node worker/scripts/seed.mjs      # uploads a test report and prints its links
node worker/scripts/seed.mjs --password hunter22
node worker/scripts/seed.mjs --file target/scan.json   # a report saved by fettle-collect
npm test -w @fettle/worker        # runs in workerd with local R2 and rate limiting
```

The worker's tests stay on vitest 4.1 while the other workspaces use vitest 5. Under vitest 5, `@cloudflare/vitest-plugin` 1.4.0 (the latest, even though it lists vitest 5 as supported) fails every test file with "File path must be a file: URL" (checked 2026-10-12 on Windows, which CI also uses). Move the worker to vitest 5 once a plugin release newer than 1.4.0 runs these tests.

R2 and the rate limiters are simulated locally; nothing touches a Cloudflare account.

Uploads need the `ADDRESS_PEPPER` secret (see Deploy); without it they answer 503 `unavailable`. For `wrangler dev`, put a line `ADDRESS_PEPPER=<any long random text>` in `worker/.dev.vars` (git-ignored). The tests set their own.

## Cloudflare setup (done 2026-10-10)

- R2 bucket `fettle-reports`, in the **EU jurisdiction** (shared receipts stay in EU data centres). Lifecycle rules delete `7d/` after 7 days, `30d/` after 30 days and `1y/` after 365 days. The Worker also refuses expired links on every read; the rules only clean up.
- Rate limiters have their own `namespace_id`s (unique across the account). They count per data centre, so they only bound cost; the global password cap is `UnlockGuard`.
- The Worker serves only on the custom domain `fettle.page` (attached on deploy). `workers_dev` and preview URLs are off.
- Zone `fettle.page`: full (strict) TLS, minimum TLS 1.2, HTTP/3, Early Hints, and browser cache left to the site's own headers. Everything that rewrites HTML or adds scripts or cookies is off: email obfuscation, Rocket Loader, automatic HTTPS rewrites, Bot Fight Mode, Web Analytics and Speed Brain. Turning any of them on would break the strict CSP or the site's no-cookies promise.
- `www.fettle.page` redirects (301) to `https://fettle.page` with an edge redirect rule.
- Mail is Fastmail (MX, SPF, DKIM and a monitor-only DMARC record, each commented in the DNS).

## Deploy

`npm run deploy -w @fettle/worker` (it rebuilds the site, then runs `wrangler deploy`).

Secrets, set once with `wrangler secret put <name>` (each put redeploys):
- `ADDRESS_PEPPER` (required): long random text, mixed into the name of each address's daily upload budget so the name can't be worked back to an IP. Without it, uploads are refused (503) and a loud `address_pepper_missing` line is logged. Changing it only resets that day's upload counts.
- `GITHUB_TOKEN` (optional): a read-only token for the download count. Deploy the site with each app release: `/app/latest.json` follows `published` in `apps/web/src/release.ts`.
