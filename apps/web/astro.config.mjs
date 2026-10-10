import react from "@astrojs/react";
import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

// Static site served by our own Worker (worker/wrangler.jsonc points its assets here).
// `site` only feeds canonical, hreflang and sitemap URLs; the Worker itself has no domain in it.
export default defineConfig({
  site: "https://fettle.page",
  output: "static",
  trailingSlash: "never",
  build: {
    format: "file",
    // One external stylesheet per page; scripts/inline-css.mjs then inlines it and adds its hash to the CSP.
    inlineStylesheets: "never",
  },
  i18n: {
    locales: ["en", "it"],
    defaultLocale: "en",
    routing: { prefixDefaultLocale: false },
  },
  integrations: [
    react(),
    sitemap({
      i18n: { defaultLocale: "en", locales: { en: "en", it: "it" } },
      filter: (page) => !/\/(share|404)$/.test(page),
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
    build: { assetsInlineLimit: 0 },
  },
});
