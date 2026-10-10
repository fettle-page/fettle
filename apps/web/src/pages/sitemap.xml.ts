// /sitemap.xml, where Search Console and other tools look first: the same index the sitemap
// integration writes to /sitemap-index.xml (test/sitemap.test.ts keeps them byte for byte equal).
import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) =>
  new Response(
    `<?xml version="1.0" encoding="UTF-8"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${new URL("/sitemap-0.xml", site)}</loc></sitemap></sitemapindex>`,
    { headers: { "content-type": "application/xml" } },
  );
