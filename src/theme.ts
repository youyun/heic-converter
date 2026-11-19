export type ThemePref = "light" | "dark" | "system";

const STORAGE_KEY = "heic-converter.theme";

let pref: ThemePref = "system";
let media: MediaQueryList | null = null;

export function detectInitialTheme(): ThemePref {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark" || saved === "system") return saved;
  } catch {
    /* ignore */
  }
  return "system";
}

export function getThemePref(): ThemePref {
  return pref;
}

function resolvedTheme(): "light" | "dark" {
  if (pref === "light" || pref === "dark") return pref;
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return "light";
}

function applyDom() {
  const resolved = resolvedTheme();
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
}

function onSystemChange() {
  if (pref === "system") applyDom();
}

export function setThemePref(next: ThemePref) {
  pref = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
  applyDom();
}

export function initTheme(initial: ThemePref = detectInitialTheme()) {
  pref = initial;
  applyDom();
  if (typeof window !== "undefined" && window.matchMedia) {
    media?.removeEventListener("change", onSystemChange);
    media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", onSystemChange);
  }
}
