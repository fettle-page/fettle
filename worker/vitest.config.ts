import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

const REACT = ["react", "react-dom", "react-dom/server", "react/jsx-runtime"];

export default defineConfig({
  plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.jsonc" } })],
  // The share page is rendered with React inside the Worker. Pre-bundle React so the test runner
  // loads one copy (its CommonJS files would otherwise load twice and break hooks).
  resolve: { dedupe: ["react", "react-dom"] },
  test: { deps: { optimizer: { ssr: { enabled: true, include: REACT } } } },
});
