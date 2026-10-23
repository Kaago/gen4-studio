/** Theme and density switches. Framework-free: the scene and CodeMirror subscribe through `onThemeChange`. */

export type Theme = "system" | "hell" | "dunkel";
export type Density = "komfort" | "kompakt";

const THEME_KEY = "g4.theme";
const DENSITY_KEY = "g4.density";
const DARK_QUERY = "(prefers-color-scheme: dark)";

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* the choice is simply not remembered */
  }
}

export function readTheme(): Theme {
  const v = read(THEME_KEY);
  return v === "hell" || v === "dunkel" ? v : "system";
}

export function readDensity(): Density {
  return read(DENSITY_KEY) === "kompakt" ? "kompakt" : "komfort";
}

/** The fixed theme currently on `<html>`, or null while the system decides. */
function fixedTheme(): "hell" | "dunkel" | null {
  const v = document.documentElement.dataset.theme;
  return v === "hell" || v === "dunkel" ? v : null;
}

function systemPrefersDark(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia(DARK_QUERY).matches;
}

/** The theme that is actually shown: the fixed one, else whatever the system prefers. */
export function effectiveTheme(): "hell" | "dunkel" {
  return fixedTheme() ?? (systemPrefersDark() ? "dunkel" : "hell");
}

type ThemeListener = (t: "hell" | "dunkel") => void;

const listeners = new Set<ThemeListener>();
let mediaList: MediaQueryList | null = null;

function emit(): void {
  const t = effectiveTheme();
  for (const cb of [...listeners]) cb(t);
}

/** An OS switch only matters while no fixed theme overrides it. */
function onSystemChange(): void {
  if (fixedTheme() === null) emit();
}

export function applyTheme(t: Theme): void {
  const before = effectiveTheme();
  const root = document.documentElement;
  if (t === "system") delete root.dataset.theme;
  else root.dataset.theme = t;
  write(THEME_KEY, t);
  if (effectiveTheme() !== before) emit();
}

export function applyDensity(d: Density): void {
  document.documentElement.dataset.dichte = d;
  write(DENSITY_KEY, d);
}

/** Calls `cb` once per effective switch; returns the unsubscribe function. */
export function onThemeChange(cb: ThemeListener): () => void {
  listeners.add(cb);
  if (mediaList === null && typeof window.matchMedia === "function") {
    mediaList = window.matchMedia(DARK_QUERY);
    mediaList.addEventListener("change", onSystemChange);
  }
  return () => {
    listeners.delete(cb);
    if (listeners.size === 0 && mediaList !== null) {
      mediaList.removeEventListener("change", onSystemChange);
      mediaList = null;
    }
  };
}
