import { defineConfig } from "vitest/config";

// A fixed time zone east of UTC, so the local-time tests read the same on every machine and CI.
export default defineConfig({ test: { env: { TZ: "Europe/Rome" } } });
