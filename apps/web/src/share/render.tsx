// Server-side rendering of the share page, used by the Worker (worker/src/share.ts) so the receipt
// arrives already drawn; src/share/main.tsx then hydrates it. Pure: no browser APIs during render.
import type { Locale } from "@fettle/brain";
import { StrictMode } from "react";
import { renderToString } from "react-dom/server";
import type { SharePayload } from "./payload.ts";
import { SharePage } from "./SharePage.tsx";

export type { SharePayload } from "./payload.ts";

export function renderSharePage(payload: SharePayload, locale: Locale): string {
  return renderToString(
    <StrictMode>
      <SharePage payload={payload} locale={locale} />
    </StrictMode>,
  );
}
