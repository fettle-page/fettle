// /open compiles the Rust receipt in the visitor's browser (the file they pick is never uploaded).
// Browsers block WebAssembly.compile under a script-src without 'wasm-unsafe-eval', so these two
// pages, and only these, get it added to the static site's policy (apps/web/public/_headers).
// _headers can't do this itself: a second rule's CSP would be joined on as a second policy, and the
// first, stricter one would still apply.

export const OPEN_PAGES = new Set(["/open", "/it/open"]);

export async function openPage(request: Request, env: Env): Promise<Response> {
  const page = await env.ASSETS.fetch(request);
  const policy = page.headers.get("content-security-policy");
  if (!policy) return page;
  const headers = new Headers(page.headers);
  headers.set("content-security-policy", policy.replace("script-src 'self'", "script-src 'self' 'wasm-unsafe-eval'"));
  return new Response(page.body, { status: page.status, statusText: page.statusText, headers });
}
