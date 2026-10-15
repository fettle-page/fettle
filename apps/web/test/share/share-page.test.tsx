// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { brain } from "@fettle/brain/node";
import { stamp } from "@fettle/receipt-ui";
import { SharePage } from "../../src/share/SharePage.tsx";
import { battery, report, startup } from "../../../../packages/receipt-ui/test/helpers/report.ts";

afterEach(cleanup);
beforeEach(() => {
  window.location.hash = "";
  localStorage.clear();
});

const ID = "m3kx9f2q7pa8hd4wz";
const expiresAt = "2026-11-06T10:00:00.000Z";
const tired = report({ battery: battery(45000, 26100), startup: startup(18) });
/** What the server sends: the finished receipt. */
const tiredModel = (locale: "en" | "it" = "en") => brain.receiptFor(tired, locale, "UTC").model;
const reply = (status: number, body?: unknown) =>
  Promise.resolve(new Response(body === undefined ? null : JSON.stringify(body), { status, headers: { "content-type": "application/json" } }));

describe("SharePage · report", () => {
  it("shows the receipt, its expiry and a way home", () => {
    const { container } = render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel(), expiresAt }} locale="en" />);
    expect(container.querySelector(".fettle-receipt")).not.toBeNull();
    // No line-by-line print: hidden lines would delay the largest paint (share.css feeds it in instead).
    expect(container.querySelector(".fettle-receipt[data-printing]")).toBeNull();
    expect(screen.getByRole("heading", { level: 1, name: "Shared receipt" })).toBeTruthy();
    expect(screen.getByText("This link expires on 6 November 2026.")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Check your own PC with Fettle" }).getAttribute("href")).toBe("/");
  });

  it("has a page heading for screen readers, without moving focus on load", () => {
    render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel(), expiresAt }} locale="en" />);
    expect(screen.getByRole("heading", { level: 1, name: "Shared receipt" })).toBeTruthy();
    expect(document.activeElement).toBe(document.body);
  });

  it("speaks Italian", () => {
    render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel("it"), expiresAt }} locale="it" />);
    expect(screen.getByText("Questo link scade il 6 novembre 2026.")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: "Scontrino condiviso" })).toBeTruthy();
  });

  it("sends Italian visitors to the Italian home page", () => {
    const { container } = render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel("it"), expiresAt }} locale="it" />);
    expect(screen.getByRole("link", { name: "Controlla il tuo PC con Fettle" }).getAttribute("href")).toBe("/it");
    expect(container.querySelector("a.home")!.getAttribute("href")).toBe("/it");
  });

  it("gives an Italian date the article its day needs", () => {
    for (const [day, text] of [
      ["2026-11-08", "Questo link scade l'8 novembre 2026."],
      ["2026-11-11", "Questo link scade l'11 novembre 2026."],
      ["2026-11-01", "Questo link scade il 1° novembre 2026."],
    ] as const) {
      render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel("it"), expiresAt: `${day}T10:00:00.000Z` }} locale="it" />);
      expect(screen.getByText(text)).toBeTruthy();
      cleanup();
    }
  });
});

describe("SharePage · unreadable receipt", () => {
  it("shows the missing page instead of a blank screen", () => {
    const silence = vi.spyOn(console, "error").mockImplementation(() => {});
    const broken = { ...tiredModel(), sections: null } as never;
    render(<SharePage payload={{ kind: "report", id: ID, model: broken, expiresAt }} locale="en" />);
    expect(screen.getByRole("heading", { name: "This receipt isn't here." })).toBeTruthy();
    silence.mockRestore();
  });
});

describe("SharePage · missing", () => {
  it("says the receipt isn't here", () => {
    render(<SharePage payload={{ kind: "missing" }} locale="en" />);
    expect(screen.getByRole("heading", { name: "This receipt isn't here." })).toBeTruthy();
    expect(screen.getByText("It may have expired, or its owner deleted it.")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Check your own PC with Fettle" })).toBeTruthy();
  });
});

describe("SharePage · delete", () => {
  it("explains an incomplete link", () => {
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" />);
    expect(screen.getByText("This delete link is incomplete. Copy it again from Fettle.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Delete this receipt" })).toBeNull();
  });

  it("deletes with the token from the link", async () => {
    window.location.hash = "#tok123";
    let resolve!: (r: Response) => void;
    const fetcher = vi.fn(() => new Promise<Response>((r) => { resolve = r; }));
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" fetcher={fetcher} />);
    expect(screen.getByRole("heading", { name: "Delete this receipt?" })).toBeTruthy();
    const button = screen.getByRole("button", { name: "Delete this receipt" });
    fireEvent.click(button);
    expect(fetcher).toHaveBeenCalledWith(`/api/reports/${ID}`, { method: "DELETE", headers: { Authorization: "Bearer tok123" } });
    expect((button as HTMLButtonElement).disabled).toBe(true);
    resolve(new Response(null, { status: 204 }));
    expect(await screen.findByRole("heading", { name: "Deleted. The link no longer works." })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Delete this receipt?" })).toBeNull();
    // The button that had focus is gone: focus moves to the outcome, so a screen reader reads it.
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Deleted. The link no longer works." }));
  });

  it("titles an already-deleted receipt as gone", async () => {
    window.location.hash = "#tok123";
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" fetcher={() => reply(404)} />);
    fireEvent.click(screen.getByRole("button", { name: "Delete this receipt" }));
    expect(await screen.findByRole("heading", { name: "This receipt is already gone." })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Delete this receipt" })).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "This receipt is already gone." }));
  });

  it("doesn't blame the connection for a server problem", async () => {
    window.location.hash = "#tok123";
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" fetcher={() => reply(500)} />);
    fireEvent.click(screen.getByRole("button", { name: "Delete this receipt" }));
    expect(await screen.findByText("Something went wrong. Try again in a moment.")).toBeTruthy();
  });

  it.each([
    [403, "This delete link doesn't match this receipt."],
    [404, "This receipt is already gone."],
  ])("explains a %i", async (status, text) => {
    window.location.hash = "#tok123";
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" fetcher={() => reply(status)} />);
    fireEvent.click(screen.getByRole("button", { name: "Delete this receipt" }));
    expect(await screen.findByText(text)).toBeTruthy();
  });

  it("says when Fettle can't be reached", async () => {
    window.location.hash = "#tok123";
    render(<SharePage payload={{ kind: "delete", id: ID }} locale="en" fetcher={() => Promise.reject(new TypeError("offline"))} />);
    // The live region is there, empty, before anything happens: screen readers announce a change to it,
    // not a region that appears already filled.
    const status = screen.getByRole("status");
    expect(status.textContent).toBe("");
    fireEvent.click(screen.getByRole("button", { name: "Delete this receipt" }));
    await waitFor(() => expect(status.textContent).toBe("Couldn't reach Fettle. Try again."));
    expect(screen.getByRole("status")).toBe(status);
  });
});

describe("SharePage · locked", () => {
  const locked = { kind: "locked" as const, id: ID, expiresAt };
  const submit = (password: string) => {
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: password } });
    fireEvent.click(screen.getByRole("button", { name: "Open receipt" }));
  };

  it("asks for the password and opens the receipt", async () => {
    let resolve!: (r: Response) => void;
    const fetcher = vi.fn(() => new Promise<Response>((r) => { resolve = r; }));
    const { container } = render(<SharePage payload={locked} locale="en" fetcher={fetcher} />);
    expect(screen.getByRole("heading", { name: "This receipt is password protected." })).toBeTruthy();
    expect(screen.getByLabelText("Password").getAttribute("type")).toBe("password");
    submit("hunter22");
    expect(fetcher).toHaveBeenCalledWith(`/api/reports/${ID}/unlock`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"password":"hunter22","locale":"en"}',
    });
    expect((screen.getByRole("button", { name: "Open receipt" }) as HTMLButtonElement).disabled).toBe(true);
    resolve(new Response(JSON.stringify({ model: tiredModel(), expiresAt }), { status: 200 }));
    await waitFor(() => expect(container.querySelector(".fettle-receipt")).not.toBeNull());
    // Drawn from the server's model, as sent.
    expect(container.querySelector(".fettle-receipt")!.textContent).toContain(tiredModel().summary);
    // The form that had focus is gone: focus moves to the page heading, and the tab title follows.
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: "Shared receipt" }));
    expect(document.title).toBe(`Fettle receipt: ${tiredModel().summary}`);
  });

  it("titles the tab in Italian once opened", async () => {
    render(<SharePage payload={locked} locale="it" fetcher={() => reply(200, { model: tiredModel("it"), expiresAt })} />);
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: "Apri lo scontrino" }));
    await waitFor(() => expect(document.title).toBe(`Scontrino Fettle: ${tiredModel("it").summary}`));
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: "Scontrino condiviso" }));
  });

  const passKey = `fettle-pass:${ID}`;
  const tomorrow = () => new Date(Date.now() + 86_400_000).toISOString();

  it("remembers the pass the server gives for a right password", async () => {
    const fetcher = () => reply(200, { model: tiredModel(), expiresAt, pass: "p4ss", passExpiresAt: tomorrow() });
    const { container } = render(<SharePage payload={locked} locale="en" fetcher={fetcher} />);
    submit("hunter22");
    await waitFor(() => expect(container.querySelector(".fettle-receipt")).not.toBeNull());
    expect(JSON.parse(localStorage.getItem(passKey)!).pass).toBe("p4ss");
  });

  it("reopens with a saved pass without asking for the password", async () => {
    localStorage.setItem(passKey, JSON.stringify({ pass: "p4ss", expiresAt: tomorrow() }));
    const fetcher = vi.fn(() => reply(200, { model: tiredModel(), expiresAt }));
    const { container } = render(<SharePage payload={locked} locale="en" fetcher={fetcher} />);
    await waitFor(() => expect(container.querySelector(".fettle-receipt")).not.toBeNull());
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher).toHaveBeenCalledWith(`/api/reports/${ID}/unlock`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"pass":"p4ss","locale":"en"}',
    });
  });

  it("forgets a pass the server refuses and quietly asks for the password", async () => {
    localStorage.setItem(passKey, JSON.stringify({ pass: "old", expiresAt: tomorrow() }));
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(403, { error: "invalid_pass" })} />);
    await waitFor(() => expect((screen.getByRole("button", { name: "Open receipt" }) as HTMLButtonElement).disabled).toBe(false));
    expect(localStorage.getItem(passKey)).toBeNull();
    expect(screen.queryByText("That password isn't right.")).toBeNull();
  });

  it("keeps a receipt opened by password when an old saved pass is refused later", async () => {
    localStorage.setItem(passKey, JSON.stringify({ pass: "old", expiresAt: tomorrow() }));
    let refuseOld!: () => void;
    const fetcher = vi.fn((_url: string, init: RequestInit) =>
      String(init.body).includes('"pass":"old"')
        ? new Promise<Response>((r) => { refuseOld = () => r(new Response(JSON.stringify({ error: "invalid_pass" }), { status: 403 })); })
        : reply(200, { model: tiredModel(), expiresAt, pass: "new", passExpiresAt: tomorrow() }),
    );
    const { container } = render(<SharePage payload={locked} locale="en" fetcher={fetcher} />);
    await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(1));
    // The old pass is still being checked when the visitor types the password anyway.
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "hunter22" } });
    fireEvent.submit(screen.getByLabelText("Password").closest("form")!);
    await waitFor(() => expect(container.querySelector(".fettle-receipt")).not.toBeNull());
    refuseOld();
    await new Promise((r) => setTimeout(r, 20));
    expect(container.querySelector(".fettle-receipt")).not.toBeNull();
    expect(JSON.parse(localStorage.getItem(passKey)!).pass).toBe("new");
  });

  it("ignores an expired saved pass", () => {
    localStorage.setItem(passKey, JSON.stringify({ pass: "old", expiresAt: "2020-01-01T00:00:00.000Z" }));
    const fetcher = vi.fn(() => reply(200, {}));
    render(<SharePage payload={locked} locale="en" fetcher={fetcher} />);
    expect(fetcher).not.toHaveBeenCalled();
    expect(localStorage.getItem(passKey)).toBeNull();
    expect(screen.getByLabelText("Password")).toBeTruthy();
  });

  it("says when the password is wrong and keeps focus on the field", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(403, { error: "wrong_password" })} />);
    submit("nope");
    expect(await screen.findByText("That password isn't right.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText("Password"));
  });

  it("doesn't blame the connection for a server problem", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(500)} />);
    submit("hunter22");
    expect(await screen.findByText("Something went wrong. Try again in a moment.")).toBeTruthy();
  });

  it("asks to try again later while the server can't check passwords (503), in both languages", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(503, { error: "unavailable" })} />);
    submit("hunter22");
    expect(await screen.findByText("Something went wrong. Try again in a moment.")).toBeTruthy();
    cleanup();
    render(<SharePage payload={{ ...locked, locale: "it" }} locale="it" fetcher={() => reply(503, { error: "unavailable" })} />);
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: "Apri lo scontrino" }));
    expect(await screen.findByText("Qualcosa non ha funzionato. Riprova tra poco.")).toBeTruthy();
  });

  it("asks people to wait after too many tries", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(429, { error: "rate_limited" })} />);
    submit("nope");
    expect(await screen.findByText("Too many tries. Wait a minute and try again.")).toBeTruthy();
  });

  // The day's tries reset at midnight UTC, which isn't "tomorrow" everywhere: say how long, from retry-after.
  const dayUsedUp = (seconds: number) => () =>
    Promise.resolve(new Response(JSON.stringify({ error: "rate_limited" }), { status: 429, headers: { "content-type": "application/json", "retry-after": String(seconds) } }));
  it.each([
    [36_000, "en", "Too many tries for today. Try again in about 10 hours."],
    [5_400, "en", "Too many tries for today. Try again in about 2 hours."],
    [4_000, "en", "Too many tries for today. Try again in about an hour."],
    [1_800, "en", "Too many tries for today. Try again in less than an hour."],
    [7_200, "it", "Troppi tentativi per oggi. Riprova tra circa 2 ore."],
    [3_600, "it", "Troppi tentativi per oggi. Riprova tra circa un'ora."],
    [900, "it", "Troppi tentativi per oggi. Riprova tra meno di un'ora."],
  ] as const)("says how long until the day's tries come back (%i s, %s)", async (seconds, locale, text) => {
    render(<SharePage payload={locked} locale={locale} fetcher={dayUsedUp(seconds)} />);
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "nope" } });
    fireEvent.submit(screen.getByLabelText("Password").closest("form")!);
    expect(await screen.findByText(text)).toBeTruthy();
  });

  it("says when Fettle can't be reached", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => Promise.reject(new TypeError("offline"))} />);
    const status = screen.getByRole("status");
    expect(status.textContent).toBe("");
    expect(screen.getByLabelText("Password").getAttribute("aria-describedby")).toBeNull();
    submit("hunter22");
    await waitFor(() => expect(status.textContent).toBe("Couldn't reach Fettle. Try again."));
    expect(screen.getByRole("status")).toBe(status);
    expect(screen.getByLabelText("Password").getAttribute("aria-describedby")).toBe(status.id);
    expect(await screen.findByText("Couldn't reach Fettle. Try again.")).toBeTruthy();
  });

  it("doesn't trust an unlock reply whose expiry isn't a date", async () => {
    render(<SharePage payload={locked} locale="en" fetcher={() => reply(200, { model: tiredModel(), expiresAt: "x" })} />);
    submit("hunter22");
    expect(await screen.findByRole("heading", { name: "This receipt isn't here." })).toBeTruthy();
  });

  it("a reply without a usable receipt shows the missing card", async () => {
    for (const body of [{ report: tired, expiresAt }, { model: { nope: 1 }, expiresAt }]) {
      cleanup();
      render(<SharePage payload={locked} locale="en" fetcher={() => reply(200, body)} />);
      submit("hunter22");
      expect(await screen.findByRole("heading", { name: "This receipt isn't here." }), JSON.stringify(Object.keys(body))).toBeTruthy();
    }
  });

  it("asks for the receipt in the page's language", () => {
    const fetcher = vi.fn(() => new Promise<Response>(() => {}));
    render(<SharePage payload={locked} locale="it" fetcher={fetcher} />);
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: "Apri lo scontrino" }));
    expect(JSON.parse((fetcher.mock.calls[0] as unknown as [string, RequestInit])[1].body as string).locale).toBe("it");
  });
});

describe("SharePage · theme", () => {
  it("cycles the theme and remembers it for the website too", () => {
    document.documentElement.removeAttribute("data-theme");
    // A dark device: Auto looks Dark, so the full cycle Light → Dark → OLED → Auto applies.
    window.matchMedia = ((q: string) => ({ matches: q.includes("dark"), media: q, addEventListener() {}, removeEventListener() {} })) as never;
    render(<SharePage payload={{ kind: "missing" }} locale="en" />);
    const button = screen.getByRole("button", { name: "Theme: Auto" });
    fireEvent.click(button);
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(localStorage.getItem("fettle-theme")).toBe("light");
    fireEvent.click(screen.getByRole("button", { name: "Theme: Light" }));
    fireEvent.click(screen.getByRole("button", { name: "Theme: Dark" }));
    expect(document.documentElement.dataset.theme).toBe("oled");
    fireEvent.click(screen.getByRole("button", { name: "Theme: OLED" }));
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    expect(localStorage.getItem("fettle-theme")).toBeNull();
  });
});

describe("SharePage · expiry in local time", () => {
  it("formats the expiry in the visitor's own time zone once mounted", async () => {
    // 23:30 UTC is already the next day east of Greenwich (this PC runs on Italian time).
    const late = "2026-11-06T23:30:00.000Z";
    const { container } = render(<SharePage payload={{ kind: "report", id: ID, model: tiredModel(), expiresAt: late }} locale="en" />);
    const local = new Intl.DateTimeFormat("en-GB", { dateStyle: "long" }).format(new Date(late));
    expect(local, "this test needs a time zone east of UTC").not.toBe("6 November 2026");
    await waitFor(() => expect(container.querySelector(".note")!.textContent).toBe(`This link expires on ${local}.`));
  });
});

describe("SharePage · scan time in local time", () => {
  it("shows the scan time in the visitor's own time zone once mounted", async () => {
    const r = tired;
    const model = tiredModel();
    const local = stamp(r.collected_at, "en");
    expect(local, "this test needs a time zone other than UTC").not.toBe(model.meta.when);
    const { container } = render(<SharePage payload={{ kind: "report", id: ID, model, expiresAt: "2026-11-06T12:00:00.000Z" }} locale="en" />);
    await waitFor(() => expect(container.querySelector(".when")!.textContent).toBe(local));
  });
});
