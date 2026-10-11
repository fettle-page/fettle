// The /r/* pages: the static share shell, filled in per request with OG tags and the report.
import { LOCALES, type Locale } from "@fettle/brain";
import { brain } from "@fettle/brain/worker";
import { renderSharePage, type SharePayload } from "@fettle/web/share-render";
import { isValidId } from "./ids.ts";
import { escapeHtml, inlineJson, LOCKED_OG, ogText, type OgText } from "./og.ts";
import { clientKey, loadReport } from "./reports.ts";

/** Strict CSP; the shell's own inline stylesheet and theme script (inlined at build time for speed) are allowed by hash. */
const csp = (h: InlineHashes) =>
  `default-src 'self'; script-src ${["'self'", ...h.scripts].join(" ")}; style-src ${["'self'", ...h.styles].join(" ")}; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`;

/** The shell is a static asset: its stylesheet hashes only change with a deploy, so compute them once per isolate. */
type InlineHashes = { styles: string[]; scripts: string[] };
let shellCache: { text: string; hashes: InlineHashes } | undefined;

async function sha256(text: string): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return `'sha256-${btoa(String.fromCharCode(...digest))}'`;
}

/** Hashes of the shell's <style> blocks and attribute-less <script> blocks (JSON data blocks don't run). */
async function inlineHashes(text: string): Promise<InlineHashes> {
  if (shellCache?.text === text) return shellCache.hashes;
  const styles = await Promise.all([...text.matchAll(/<style>([\s\S]*?)<\/style>/g)].map(([, css]) => sha256(css!)));
  const scripts = await Promise.all([...text.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(([, js]) => sha256(js!)));
  const hashes = { styles, scripts };
  shellCache = { text, hashes };
  return hashes;
}

type Page = { status: number; payload: SharePayload; og?: OgText };

const speaks = (tag: string): Locale | undefined => {
  const lang = tag.trim().slice(0, 2).toLowerCase();
  return (LOCALES as readonly string[]).includes(lang) ? (lang as Locale) : undefined;
};

/** `?lang=` wins, then the first language in Accept-Language that Fettle speaks, then English. */
function pickLocale(asked: string | null, acceptLanguage: string | null): Locale {
  if (asked && speaks(asked) && asked.length === 2) return asked as Locale;
  const ranked = (acceptLanguage ?? "")
    .split(",")
    .map((part, i) => {
      const [tag = "", ...params] = part.split(";");
      const q = Number(params.find((p) => p.trim().startsWith("q="))?.trim().slice(2) ?? "1");
      return { tag, q: Number.isFinite(q) ? q : 0, i };
    })
    .sort((a, b) => b.q - a.q || a.i - b.i);
  for (const { tag } of ranked) {
    const locale = speaks(tag);
    if (locale) return locale;
  }
  return "en";
}

function pageHeaders(hashes: InlineHashes): Headers {
  return new Headers({
    "content-type": "text/html; charset=utf-8",
    "content-security-policy": csp(hashes),
    "x-robots-tag": "noindex",
    "referrer-policy": "no-referrer",
    "x-content-type-options": "nosniff",
    // As on the static site (apps/web/public/_headers).
    "permissions-policy": "camera=(), microphone=(), geolocation=()",
    "cache-control": "no-store",
  });
}

async function pageFor(env: Env, path: string, locale: Locale): Promise<Page> {
  const MISSING: Page = { status: 404, payload: { kind: "missing", locale } };
  const m = /^\/r\/([^/]+)(\/delete)?$/.exec(path);
  if (!m || !isValidId(m[1]!)) return MISSING;
  const id = m[1]!;
  if (m[2]) return { status: 200, payload: { kind: "delete", id, locale } };
  const loaded = await loadReport(env, id);
  if (!loaded) return MISSING;
  if (loaded.locked) return { status: 200, payload: { kind: "locked", id, expiresAt: loaded.expiresAt, locale }, og: LOCKED_OG[locale] };
  const { report } = loaded;
  // The Rust receipt, as the app builds it: verdicts as of the scan, not as of today. The model is
  // built once, here; the page draws it now and the browser reuses it, so the two never differ.
  let receipt;
  let model;
  try {
    ({ receipt, model } = brain.receiptFor(report, locale, "UTC"));
  } catch {
    // Stored before uploads checked the date, or data the engine or receipt can't read: "isn't here", never a 500.
    return MISSING;
  }
  return { status: 200, payload: { kind: "report", id, expiresAt: loaded.expiresAt, locale, model }, og: ogText(receipt, locale) };
}

function ogTags(og: OgText, origin: string, url: string): string {
  const prop = (p: string, v: string) => `<meta property="${p}" content="${escapeHtml(v)}">`;
  return [
    // The shell has no description of its own; search engines and some previews read this one.
    `<meta name="description" content="${escapeHtml(og.description)}">`,
    prop("og:title", og.title),
    prop("og:description", og.description),
    prop("og:url", url),
    prop("og:type", "website"),
    prop("og:site_name", "Fettle"),
    // A generic card (no report data in the image), served by the website's static assets.
    prop("og:image", `${origin}/og/share.png`),
    prop("og:image:width", "1200"),
    prop("og:image:height", "630"),
    '<meta name="twitter:card" content="summary_large_image">',
  ].join("");
}

export async function sharePage(request: Request, env: Env): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return new Response("Method not allowed", { status: 405, headers: { allow: "GET, HEAD", "content-type": "text/plain; charset=utf-8" } });
  }
  // Before anything is read from R2, so a flood stays cheap.
  const { success } = await env.VIEW_LIMITER.limit({ key: clientKey(request) });
  if (!success) {
    return new Response("Too many requests. Wait a minute and try again.\nTroppe richieste. Aspetta un minuto e riprova.\n", {
      status: 429,
      headers: { "retry-after": "60", "content-type": "text/plain; charset=utf-8" },
    });
  }
  const url = new URL(request.url);
  // A bare request: the visitor's conditional headers must not turn the shell into a bodiless 304.
  const shell = await env.ASSETS.fetch(new Request(new URL("/share", url)));
  if (shell.status !== 200 || !(shell.headers.get("content-type") ?? "").includes("text/html")) {
    console.log(JSON.stringify({ event: "shell_missing", status: shell.status }));
    return new Response("Share page unavailable", { status: 500, headers: { "content-type": "text/plain; charset=utf-8" } });
  }

  // Small, static and needed whole for the CSP hash, so read it once instead of streaming.
  const shellText = await shell.text();
  const hashes = await inlineHashes(shellText);

  const locale = pickLocale(url.searchParams.get("lang"), request.headers.get("accept-language"));
  const page = await pageFor(env, url.pathname, locale);
  // Draw the page now, so it arrives with the HTML (the largest paint no longer waits for scripts, and
  // it reads without JavaScript). Not the delete page: its token is in the #fragment, which only the
  // browser sees. If drawing fails, the browser draws it instead.
  let drawn = "";
  if (page.payload.kind !== "delete") {
    try {
      drawn = renderSharePage(page.payload, locale);
    } catch (e) {
      console.log(JSON.stringify({ event: "ssr_failed", message: String(e).slice(0, 200) }));
    }
  }
  let rewriter = new HTMLRewriter()
    .on("html", {
      element(el) {
        el.setAttribute("lang", locale);
      },
    })
    .on("script#fettle-share", {
      element(el) {
        el.setInnerContent(inlineJson(page.payload), { html: true });
      },
    });
  if (drawn) {
    rewriter = rewriter.on("div#root", {
      element(el) {
        el.setInnerContent(drawn, { html: true });
      },
    });
  }
  if (page.og) {
    const og = page.og;
    rewriter = rewriter
      .on("title", {
        element(el) {
          el.setInnerContent(og.title);
        },
      })
      .on("head", {
        element(el) {
          el.append(ogTags(og, url.origin, url.origin + url.pathname), { html: true });
        },
      });
  }
  const body = rewriter.transform(new Response(shellText, { headers: { "content-type": "text/html; charset=utf-8" } })).body;
  return new Response(request.method === "HEAD" ? null : body, { status: page.status, headers: pageHeaders(hashes) });
}
