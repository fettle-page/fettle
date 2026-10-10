// The FAQ answers are closed <details>; a link like /faq#temps opens the one it points at.

/** The answer's id from a location hash, or null when it isn't a plain id. */
export function answerId(hash: string): string | null {
  const m = /^#([\w-]+)$/.exec(hash);
  return m ? m[1]! : null;
}

function openAnswer() {
  const id = answerId(location.hash);
  const el = id ? document.getElementById(id) : null;
  if (el instanceof HTMLDetailsElement) el.open = true;
}

if (typeof document !== "undefined") {
  openAnswer();
  addEventListener("hashchange", openAnswer);
}
