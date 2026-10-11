// Secrets aren't in wrangler.jsonc, so `wrangler types` doesn't list them.
interface Env {
  /** Optional, read-only GitHub token for the download count (src/downloads.ts). */
  GITHUB_TOKEN?: string;
  /** Required for uploads: mixed into the daily upload budget's name (src/budget.ts). Set with `wrangler secret put`. */
  ADDRESS_PEPPER?: string;
}
