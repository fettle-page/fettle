# Contributing to Fettle

Thanks for helping. Bug reports, ideas and pull requests are all welcome. For a security problem, please don't open an issue: [SECURITY.md](SECURITY.md) says how to report it privately.

Fettle only reads what Windows already knows and explains it in plain words. It doesn't clean, "boost" or change anything, and changes that would make it do so won't be merged.

## Build and test

You need Windows, Node 24 and npm, and a stable Rust toolchain (MSVC) with the WebAssembly target. The README's [Develop](README.md#develop) section lists every command. The two that matter before any pull request:

```sh
rustup target add wasm32-unknown-unknown
npm install
npm test                  # every package: unit tests, type checks, the website build
cargo test --workspace    # the Rust crates
```

CI also runs `cargo clippy --workspace --all-targets -- -D warnings`, so any Rust warning fails it. After changing a verdict rule or its wording, run `npm run parity:goldens` to refresh the expected receipts, and check the diff.

## How changes are made

- **Test first.** Write the test, watch it fail, then make the change that passes it. A bug fix comes with the test that would have caught it.
- **Plain words.** Everything people read (verdicts, buttons, the website) says what it means without jargon, the way you'd explain it to a friend.
- **English and Italian.** Every string people see exists in both languages: the receipt and verdict copy in `crates/*/copy/en.json` and `it.json`, the app's in `apps/windows/copy/`, the website's in `apps/web/src/i18n/`. If you don't write Italian, add the English and say so in the pull request; the Italian is written before it's merged.
- **Privacy.** Reports never hold serial numbers, the PC's name, usernames, account SIDs, network addresses or product keys. Test data uses only made-up identifiers, never values copied from a real PC; `apps/windows/packaging/test/repo-files.test.mjs` checks every tracked file for real-looking SIDs, serials, processor IDs and email addresses.
- **Commits.** A short, plain message that says what changed, e.g. "Warn about a hot hard drive above 55 °C". A change people will notice gets its line in `apps/web/src/content/changelog.ts`, in English and Italian, in the same commit. `main`'s history is never rewritten.

## Pull requests

Keep each one to a single change, fill in the template, and make sure `npm test` and `cargo test --workspace` pass. By contributing, you agree your work is released under the [MIT licence](LICENSE).
