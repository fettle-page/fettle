// Theme choice, shared by the site's controls and the share page. public/theme.js applies a stored
// choice before first paint; this module changes it afterwards. Auto means "follow the device".
export type Theme = "auto" | "light" | "dark" | "oled";
export type ChosenTheme = Exclude<Theme, "auto">;

export const THEMES: Theme[] = ["auto", "light", "dark", "oled"];
export const STORAGE_KEY = "fettle-theme";

/** Browser bar colours, matching each theme's mat (also hard-coded in public/theme.js). */
export const THEME_COLOR: Record<ChosenTheme, string> = { light: "#E8EDE7", dark: "#1F6A45", oled: "#000000" };

const isChosen = (v: unknown): v is ChosenTheme => v === "light" || v === "dark" || v === "oled";

export function nextTheme(t: Theme): Theme {
  return THEMES[(THEMES.indexOf(t) + 1) % THEMES.length]!;
}

/** What a theme looks like on this device: Auto shows Light or Dark, as the device prefers. */
function looks(t: Theme, prefersDark: boolean): ChosenTheme {
  return t === "auto" ? (prefersDark ? "dark" : "light") : t;
}

/** The next theme that actually changes what's on screen (so no click looks like it did nothing). */
export function nextVisibleTheme(t: Theme, prefersDark: boolean): Theme {
  const next = nextTheme(t);
  return looks(next, prefersDark) === looks(t, prefersDark) ? nextTheme(next) : next;
}

export function prefersDark(): boolean {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    return false;
  }
}

/** The stored choice; anything missing, unknown or unreadable (private windows) is Auto. */
export function readTheme(storage: Pick<Storage, "getItem"> | null): Theme {
  try {
    const v = storage?.getItem(STORAGE_KEY);
    return isChosen(v) ? v : "auto";
  } catch {
    return "auto";
  }
}

export function applyTheme(doc: Document, t: Theme, storage: Pick<Storage, "setItem" | "removeItem"> | null): void {
  const root = doc.documentElement;
  if (t === "auto") root.removeAttribute("data-theme");
  else root.dataset.theme = t;

  for (const meta of doc.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) {
    const followsLight = (meta.getAttribute("media") ?? "").includes("light");
    meta.content = t === "auto" ? THEME_COLOR[followsLight ? "light" : "dark"] : THEME_COLOR[t];
  }

  try {
    if (t === "auto") storage?.removeItem(STORAGE_KEY);
    else storage?.setItem(STORAGE_KEY, t);
  } catch {
    // Storage blocked: the theme still applies to this page.
  }
}

/** localStorage, or null where touching it throws. */
export function safeStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}
