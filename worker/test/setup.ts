import { vi } from "vitest";

// Tests never reach the internet: GitHub (asked for the download count on the home and download pages)
// answers "unavailable" unless a test says otherwise (test/downloads.test.ts).
const real = globalThis.fetch;
vi.spyOn(globalThis, "fetch").mockImplementation((input: RequestInfo | URL, init?: RequestInit) => {
  const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
  if (url.startsWith("https://api.github.com/")) return Promise.resolve(new Response("{}", { status: 503 }));
  return real(input, init);
});
