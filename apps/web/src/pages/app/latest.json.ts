// The Windows app's update check reads this every hour, day or week (only with its switch on). Built with the site,
// so it says the published version (src/release.ts), or the app's own before any release.
import type { APIRoute } from "astro";
import { appVersion } from "../../app-version.ts";
import { latestVersion, release } from "../../release.ts";

const version = latestVersion(release, appVersion);

export const GET: APIRoute = ({ site }) => {
  const page = (path: string) => new URL(path, site).href;
  return new Response(JSON.stringify({ version, page: { en: page("/download"), it: page("/it/download") } }), {
    headers: { "content-type": "application/json" },
  });
};
