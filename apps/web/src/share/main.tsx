import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
// Styles and fonts are inlined into src/pages/share.astro at build time (allowed by hash in the CSP).
import { pickLocale, readPayload } from "./payload.ts";
import { SharePage } from "./SharePage.tsx";

const payload = readPayload(document);
// The server decides the language for pages it draws, so hydration matches; otherwise pick here.
const locale = payload.locale ?? pickLocale(location.search, navigator.languages);
document.documentElement.lang = locale;

const root = document.getElementById("root")!;
const page = (
  <StrictMode>
    <SharePage payload={payload} locale={locale} />
  </StrictMode>
);
// A receipt the Worker already drew is taken over in place; other states are drawn here.
if (root.hasChildNodes()) hydrateRoot(root, page);
else createRoot(root).render(page);
