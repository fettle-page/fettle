import type { Locale, ReceiptModel } from "@fettle/brain";
import { localePath } from "../i18n/paths.ts";
import { ReceiptView, stamp, Wordmark } from "@fettle/receipt-ui";
import { Component, useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import { dateParams, longDate, shareText, type ShareKey } from "./copy.ts";
import { isDate, isModel, type SharePayload } from "./payload.ts";
import { applyTheme, nextVisibleTheme, prefersDark, readTheme, safeStorage, STORAGE_KEY, type Theme } from "../scripts/theme.ts";

type Fetcher = (input: string, init: RequestInit) => Promise<Response>;
type Props = { payload: SharePayload; locale: Locale; fetcher?: Fetcher };

const browserFetch: Fetcher = (input, init) => fetch(input, init);

export function SharePage({ payload, locale, fetcher = browserFetch }: Props) {
  const t = (key: ShareKey, params?: Record<string, string>) => shareText(key, locale, params);
  const home = localePath(locale, "/");
  return (
    <div className="mat">
      <header className="bar">
        <a className="home" href={home}>
          <Wordmark className="brand" />
        </a>
        {/* A label for the eye: every page has its own h1 for screen readers (the report's says the same). */}
        <span className="tape quiet" aria-hidden="true">
          {t("label")}
        </span>
        <ThemeButton locale={locale} />
      </header>
      <main className="stage">
        <Fallback fallback={<Missing locale={locale} />}>
          {payload.kind === "report" && (
            <Shared model={payload.model} expiresAt={payload.expiresAt} locale={locale} />
          )}
          {payload.kind === "locked" && <Locked id={payload.id} locale={locale} fetcher={fetcher} />}
          {payload.kind === "delete" && <Delete id={payload.id} locale={locale} fetcher={fetcher} />}
          {payload.kind === "missing" && <Missing locale={locale} />}
        </Fallback>
      </main>
      <footer className="foot">
        <p>{t("footer")}</p>
        <a className="tape light" href={home}>{t("cta")}</a>
      </footer>
    </div>
  );
}

/** A receipt that can't be drawn shows "isn't here", never a blank page. */
class Fallback extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

/** Cycles Auto → Light → Dark → OLED, shared with the website through localStorage. */
function ThemeButton({ locale }: { locale: Locale }) {
  // "Auto" on the server and on the first client render (so hydration matches); the stored choice
  // is read right after mount. public/theme.js has already applied it to the page itself.
  const [theme, setTheme] = useState<Theme>("auto");
  // Hidden until interactive: before hydration it could neither work nor know the stored theme.
  const [mounted, setMounted] = useState(false);
  const name = shareText(`theme.${theme}`, locale);
  useEffect(() => {
    setTheme(readTheme(safeStorage()));
    setMounted(true);
  }, []);
  // Follow a choice made in another tab, or on a page restored from the back/forward cache.
  useEffect(() => {
    const resync = () => {
      const stored = readTheme(safeStorage());
      applyTheme(document, stored, null);
      setTheme(stored);
    };
    const onStorage = (e: StorageEvent) => (e.key === STORAGE_KEY || e.key === null) && resync();
    const onShow = (e: PageTransitionEvent) => e.persisted && resync();
    window.addEventListener("storage", onStorage);
    window.addEventListener("pageshow", onShow);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("pageshow", onShow);
    };
  }, []);
  function cycle() {
    const next = nextVisibleTheme(theme, prefersDark());
    applyTheme(document, next, safeStorage());
    setTheme(next);
  }
  return (
    <button className="tape theme-toggle" type="button" onClick={cycle} hidden={!mounted} aria-label={`${shareText("theme.label", locale)}: ${name}`}>
      <svg className="theme-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
        <circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M8 1.75a6.25 6.25 0 0 1 0 12.5z" fill="currentColor" />
      </svg>
      <span>{name}</span>
    </button>
  );
}

type SharedProps = { model: ReceiptModel; expiresAt: string; locale: Locale; opened?: boolean };

/**
 * The server's finished receipt, drawn as is (hosts build the model once; the browser never does).
 * `opened`: it just replaced the password form, so focus and the tab title move to it.
 */
function Shared({ model, expiresAt, locale, opened = false }: SharedProps) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (!opened) return;
    heading.current?.focus();
    document.title = `${shareText("page_title", locale)}: ${model.summary}`;
  }, [opened, model, locale]);
  // The server can only format the scan time and the expiry in UTC; once mounted, show the
  // visitor's own (the first client render must match the server's HTML).
  const [localTime, setLocalTime] = useState(false);
  useEffect(() => setLocalTime(true), []);
  const shown = useMemo(
    () => (localTime ? { ...model, meta: { ...model.meta, when: stamp(model.meta.collectedAt) } } : model),
    [model, localTime],
  );
  // No line-by-line print here: lines starting hidden would delay the largest paint. share.css feeds
  // the whole receipt in with a transform instead.
  return (
    <>
      <h1 className="sr-only" tabIndex={-1} ref={heading}>
        {shareText("label", locale)}
      </h1>
      <ReceiptView model={shown} />
      <p className="note">{shareText("expires", locale, dateParams(longDate(expiresAt, locale, localTime ? null : "UTC")))}</p>
    </>
  );
}

/** `focus`: the card replaced what had focus (a button that's gone), so focus moves to its title. */
function Card({ title, focus = false, children }: { title: string; focus?: boolean; children?: ReactNode }) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (focus) heading.current?.focus();
  }, [focus]);
  return (
    <section className="card" aria-labelledby="card-title">
      <h1 id="card-title" ref={heading} tabIndex={focus ? -1 : undefined}>
        {title}
      </h1>
      {children}
    </section>
  );
}

function Missing({ locale }: { locale: Locale }) {
  return (
    <Card title={shareText("missing.title", locale)}>
      <p>{shareText("missing.body", locale)}</p>
    </Card>
  );
}

type DeleteState = "idle" | "busy" | "done" | "forbidden" | "gone" | "offline" | "failed";
const DELETE_MESSAGE: Partial<Record<DeleteState, ShareKey>> = {
  forbidden: "delete.forbidden",
  offline: "offline",
  failed: "failed",
};

function Delete({ id, locale, fetcher }: { id: string; locale: Locale; fetcher: Fetcher }) {
  // The token travels in the fragment, so it never reaches the server's logs.
  const token = window.location.hash.slice(1);
  const [state, setState] = useState<DeleteState>("idle");
  const t = (key: ShareKey) => shareText(key, locale);

  if (!token) {
    return (
      <Card title={t("delete.title")}>
        <p>{t("delete.no_token")}</p>
      </Card>
    );
  }

  async function remove() {
    setState("busy");
    try {
      const res = await fetcher(`/api/reports/${id}`, { method: "DELETE", headers: { Authorization: `Bearer ${token}` } });
      setState(res.status === 204 ? "done" : res.status === 403 ? "forbidden" : res.status === 404 ? "gone" : "failed");
    } catch {
      setState("offline");
    }
  }

  // Once it's gone, the outcome is the whole page.
  if (state === "done") return <Card title={t("delete.done")} focus />;
  if (state === "gone") return <Card title={t("delete.gone")} focus />;

  const message = DELETE_MESSAGE[state];
  return (
    <Card title={t("delete.title")}>
      <p>{t("delete.body")}</p>
      {/* Always there, so a screen reader announces the message when it appears. */}
      <p role="status">{message ? t(message) : null}</p>
      <button className="tape" type="button" disabled={state === "busy"} onClick={remove}>
        {t("delete.button")}
      </button>
    </Card>
  );
}

// After a right password the server hands out a 24-hour pass, so the same browser can reopen the
// receipt without typing it again. Storage can be unavailable (private windows); then it just asks.
const passKey = (id: string) => `fettle-pass:${id}`;

function savedPass(id: string): string | null {
  try {
    const raw = localStorage.getItem(passKey(id));
    if (!raw) return null;
    const { pass, expiresAt } = JSON.parse(raw) as { pass?: unknown; expiresAt?: unknown };
    if (typeof pass === "string" && typeof expiresAt === "string" && Date.parse(expiresAt) > Date.now()) return pass;
    localStorage.removeItem(passKey(id));
  } catch {
    // No storage, or a damaged entry: ask for the password.
  }
  return null;
}

function savePass(id: string, pass: unknown, expiresAt: unknown) {
  if (typeof pass !== "string" || !isDate(expiresAt)) return;
  try {
    localStorage.setItem(passKey(id), JSON.stringify({ pass, expiresAt }));
  } catch {
    // Fine: it will ask again next time.
  }
}

function forgetPass(id: string) {
  try {
    localStorage.removeItem(passKey(id));
  } catch {
    // Nothing to forget.
  }
}

type LockedState =
  | { status: "idle" | "busy" | "wrong" | "limited" | "offline" | "failed" }
  | { status: "limited_today"; seconds: number }
  | { status: "open"; model: ReceiptModel; expiresAt: string }
  | { status: "missing" };

/**
 * How long until the day's tries come back. They reset at midnight UTC, which isn't "tomorrow" in every
 * time zone, so the message gives the wait itself, from the server's retry-after.
 */
const LOCKED_MESSAGE: Partial<Record<LockedState["status"], ShareKey>> = {
  wrong: "locked.wrong",
  limited: "locked.limited",
  offline: "offline",
  failed: "failed",
};

function dayUsedUp(seconds: number, locale: Locale): string {
  if (seconds < 3600) return shareText("locked.limited_today_soon", locale);
  const hours = Math.round(seconds / 3600);
  return hours === 1 ? shareText("locked.limited_today_hour", locale) : shareText("locked.limited_today_hours", locale, { n: String(hours) });
}

function Locked({ id, locale, fetcher }: { id: string; locale: Locale; fetcher: Fetcher }) {
  // "idle" first, as the server drew it (it can't see the browser's saved pass), so hydration matches.
  const [state, setState] = useState<LockedState>({ status: "idle" });
  const [password, setPassword] = useState("");
  const field = useRef<HTMLInputElement>(null);
  const tried = useRef(false);
  /** A typed password was sent: from then on, the saved pass's answer no longer counts. */
  const typed = useRef(false);
  const t = (key: ShareKey) => shareText(key, locale);

  const unlock = (body: { password: string } | { pass: string }) =>
    fetcher(`/api/reports/${id}/unlock`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...body, locale }) });

  /** Shows the receipt from a 200 reply, after checking its shape like any server-sent receipt. */
  async function openWith(res: Response) {
    const body: { model?: unknown; expiresAt?: unknown; pass?: unknown; passExpiresAt?: unknown } = await res.json();
    if (!isModel(body.model) || !isDate(body.expiresAt)) return setState({ status: "missing" });
    savePass(id, body.pass, body.passExpiresAt);
    setState({ status: "open", model: body.model, expiresAt: body.expiresAt });
  }

  useEffect(() => {
    if (tried.current) return;
    tried.current = true;
    const pass = savedPass(id);
    if (!pass) return;
    setState({ status: "busy" });
    unlock({ pass })
      .then(async (res) => {
        if (typed.current) return;
        if (res.status === 200) return openWith(res);
        if (res.status === 404) return setState({ status: "missing" });
        if (res.status === 403) forgetPass(id);
        setState({ status: "idle" });
      })
      .catch(() => typed.current || setState({ status: "idle" }));
  });

  if (state.status === "open") return <Shared model={state.model} expiresAt={state.expiresAt} locale={locale} opened />;
  if (state.status === "missing") return <Missing locale={locale} />;

  async function open(e: FormEvent) {
    e.preventDefault();
    typed.current = true;
    setState({ status: "busy" });
    try {
      const res = await unlock({ password });
      if (res.status === 200) {
        await openWith(res);
      } else if (res.status === 403) {
        setState({ status: "wrong" });
        field.current?.focus();
      } else if (res.status === 429) {
        // A wait past the minute means the report's tries for the day are used up.
        const seconds = Number(res.headers.get("retry-after"));
        setState(seconds > 60 ? { status: "limited_today", seconds } : { status: "limited" });
      } else if (res.status === 404) {
        setState({ status: "missing" });
      } else {
        setState({ status: "failed" });
      }
    } catch {
      setState({ status: "offline" });
    }
  }

  const key = LOCKED_MESSAGE[state.status];
  const message = state.status === "limited_today" ? dayUsedUp(state.seconds, locale) : key ? t(key) : null;
  return (
    <Card title={t("locked.title")}>
      <form className="unlock" onSubmit={open}>
        <label htmlFor="share-password">{t("locked.label")}</label>
        <input
          ref={field}
          id="share-password"
          type="password"
          autoComplete="off"
          required
          maxLength={128}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          aria-invalid={state.status === "wrong" || undefined}
          aria-describedby={message ? "unlock-message" : undefined}
        />
        <button className="tape" type="submit" disabled={state.status === "busy"}>
          {t("locked.button")}
        </button>
        {/* Always there, so a screen reader announces the message when it appears. */}
        <p id="unlock-message" role="status">
          {message}
        </p>
      </form>
    </Card>
  );
}
