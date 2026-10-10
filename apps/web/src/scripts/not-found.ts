// One 404 page serves every missing address. It carries both languages; English shows without
// JavaScript, and an Italian address (/it or /it/…) switches to the Italian.

export function pickNotFound(doc: Document, path: string): void {
  if (path !== "/it" && !path.startsWith("/it/")) return;
  for (const section of doc.querySelectorAll<HTMLElement>("[data-nf]")) section.hidden = section.dataset.nf !== "it";
  doc.documentElement.lang = "it";
  const title = doc.querySelector<HTMLElement>('[data-nf="it"]')?.dataset.title;
  if (title) doc.title = title;
}

if (typeof document !== "undefined") pickNotFound(document, location.pathname);
