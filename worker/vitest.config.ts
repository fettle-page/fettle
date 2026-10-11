import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

const REACT = ["react", "react-dom", "react-dom/server", "react/jsx-runtime"];

export default defineConfig({
  // ADDRESS_PEPPER is a Worker secret in production; the tests use a fixed stand-in.
  plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.jsonc" }, miniflare: { bindings: { ADDRESS_PEPPER: "test-pepper-not-a-real-secret" } } })],
  // The share page is rendered with React inside the Worker. Pre-bundle React so the test runner
  // loads one copy (its CommonJS files would otherwise load twice and break hooks).
  resolve: { dedupe: ["react", "react-dom"] },
  // Each file's first test also loads the Worker, the brain and React: over 5 s on a busy CI machine.
  test: { testTimeout: 30_000, setupFiles: ["./test/setup.ts"], deps: { optimizer: { ssr: { enabled: true, include: REACT } } } },
});
