// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { pickLocale, readPayload } from "../../src/share/payload.ts";
import { model, report } from "../../../../packages/receipt-ui/test/helpers/report.ts";

function docWith(text: string | null): Document {
  const doc = document.implementation.createHTMLDocument("t");
  if (text !== null) {
    const s = doc.createElement("script");
    s.id = "fettle-share";
    s.type = "application/json";
    s.textContent = text;
    doc.body.append(s);
  }
  return doc;
}

const ID = "m3kx9f2q7pa8hd4wz";

describe("readPayload", () => {
  it("returns a valid report payload", () => {
    const payload = { kind: "report", id: ID, model: model({}), expiresAt: "2026-11-06T10:00:00.000Z" };
    expect(readPayload(docWith(JSON.stringify(payload)))).toEqual(payload);
  });

  it("a raw report is no longer enough: the server always sends the finished receipt", () => {
    const payload = { kind: "report", id: ID, report: report({}), expiresAt: "2026-11-06T10:00:00.000Z" };
    expect(readPayload(docWith(JSON.stringify(payload)))).toEqual({ kind: "missing" });
  });

  it("falls back to missing for anything it can't trust", () => {
    for (const text of [null, "", "{", '{"kind":"nope"}', '{"kind":"report","report":{"schema_version":1}}', '{"kind":"locked"}']) {
      expect(readPayload(docWith(text)), String(text)).toEqual({ kind: "missing" });
    }
  });

  it("refuses an expiry that isn't a date", () => {
    expect(readPayload(docWith(`{"kind":"locked","id":"${ID}","expiresAt":"x"}`))).toEqual({ kind: "missing" });
    const payload = { kind: "report", id: ID, model: model({}), expiresAt: "soon" };
    expect(readPayload(docWith(JSON.stringify(payload)))).toEqual({ kind: "missing" });
  });

  it("passes locked and delete payloads through", () => {
    const locked = { kind: "locked", id: ID, expiresAt: "2026-11-06T10:00:00.000Z" };
    expect(readPayload(docWith(JSON.stringify(locked)))).toEqual(locked);
    expect(readPayload(docWith(`{"kind":"delete","id":"${ID}"}`))).toEqual({ kind: "delete", id: ID });
  });
});

describe("readPayload · server-drawn reports", () => {
  it("passes a server-built model and the server's language through, without needing the report", () => {
    const m = model({});
    const payload = { kind: "report", id: ID, expiresAt: "2026-11-06T10:00:00.000Z", locale: "it", model: m };
    expect(readPayload(docWith(JSON.stringify(payload)))).toEqual(payload);
  });

  it("drops an unknown language and refuses a report with neither model nor valid report", () => {
    const m = model({});
    expect(readPayload(docWith(JSON.stringify({ kind: "report", id: ID, expiresAt: "2026-11-06T10:00:00.000Z", locale: "de", model: m }))))
      .toEqual({ kind: "report", id: ID, expiresAt: "2026-11-06T10:00:00.000Z", model: m });
    expect(readPayload(docWith(JSON.stringify({ kind: "report", id: ID, expiresAt: "2026-11-06T10:00:00.000Z", model: { nope: 1 } })))).toEqual({ kind: "missing" });
  });
});

describe("pickLocale", () => {
  it("prefers ?lang=, then the browser languages, then English", () => {
    expect(pickLocale("?lang=it", ["en-GB"])).toBe("it");
    expect(pickLocale("", ["it-IT", "en"])).toBe("it");
    expect(pickLocale("", ["de-DE"])).toBe("en");
    expect(pickLocale("", ["de-DE", "it"])).toBe("it");
  });
});
