// @vitest-environment jsdom
import { act, StrictMode } from "react";
import { hydrateRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderSharePage } from "../../src/share/render.tsx";
import { SharePage } from "../../src/share/SharePage.tsx";
import type { SharePayload } from "../../src/share/payload.ts";
import { battery, model, startup } from "../../../../packages/receipt-ui/test/helpers/report.ts";

// Tell React this is a test environment, so act() works without warnings.
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => vi.restoreAllMocks());

describe("server-drawn share page", () => {
  for (const locale of ["en", "it"] as const) {
    it(`hydrates without a mismatch (${locale})`, async () => {
      localStorage.setItem("fettle-theme", "oled"); // the server can't know this; the button must not mismatch
      const payload: SharePayload = {
        kind: "report",
        id: "m3kx9f2q7pa8hd4wz",
        expiresAt: "2026-11-06T23:30:00.000Z",
        locale,
        model: model({ battery: battery(45000, 26100), startup: startup(18) }, locale),
      };
      const container = document.createElement("div");
      container.innerHTML = renderSharePage(payload, locale);
      document.body.append(container);
      expect(container.querySelector(".fettle-receipt")).not.toBeNull();
      expect(container.innerHTML, "no style attributes in server HTML").not.toMatch(/\sstyle="/);
      // Before hydration the theme button can't work or know the stored theme: it stays hidden.
      expect(container.querySelector<HTMLButtonElement>(".theme-toggle")!.hidden).toBe(true);

      const errors = vi.spyOn(console, "error").mockImplementation(() => {});
      const recoverable: unknown[] = [];
      await act(async () => {
        hydrateRoot(
          container,
          <StrictMode>
            <SharePage payload={payload} locale={locale} />
          </StrictMode>,
          { onRecoverableError: (e) => recoverable.push(e) },
        );
      });
      expect(recoverable).toEqual([]);
      expect(errors).not.toHaveBeenCalled();
      // After mount, the theme button appears and shows the stored choice.
      const toggle = container.querySelector<HTMLButtonElement>(".theme-toggle")!;
      expect(toggle.hidden).toBe(false);
      expect(toggle.textContent).toContain("OLED");
      container.remove();
      localStorage.clear();
    });
  }
});

describe("server-drawn short pages", () => {
  for (const payload of [{ kind: "missing", locale: "en" }, { kind: "locked", id: "m3kx9f2q7pa8hd4wz", expiresAt: "2026-11-06T10:00:00.000Z", locale: "it" }] as SharePayload[]) {
    it(`hydrate without a mismatch (${payload.kind})`, async () => {
      const locale = payload.locale!;
      const container = document.createElement("div");
      container.innerHTML = renderSharePage(payload, locale);
      document.body.append(container);
      const errors = vi.spyOn(console, "error").mockImplementation(() => {});
      const recoverable: unknown[] = [];
      await act(async () => {
        hydrateRoot(container, <StrictMode><SharePage payload={payload} locale={locale} /></StrictMode>, { onRecoverableError: (e) => recoverable.push(e) });
      });
      expect(recoverable).toEqual([]);
      expect(errors).not.toHaveBeenCalled();
      container.remove();
    });
  }
});
