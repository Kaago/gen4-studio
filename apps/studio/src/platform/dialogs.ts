import { ask as tauriAsk, open as tauriOpen, save as tauriSave } from "@tauri-apps/plugin-dialog";
import { inTauri } from "./transport";

/**
 * The few native dialogs the app uses. Inside Tauri they are the system dialogs; in a plain browser (the dev
 * bridge, see `transport.ts`) they are asked on the page itself by `WebDialogHost`, which a person or an agent
 * can answer like any other control. A path is typed there instead of picked.
 */

export type AskKind = "info" | "warning" | "error";

/** A question waiting for an answer on the page. */
export type WebRequest =
  | { kind: "ask"; message: string; level: AskKind; resolve: (yes: boolean) => void }
  | { kind: "path"; title: string; initial: string; resolve: (path: string | null) => void };

const LAST_PATH_KEY = "g4.web.lastPath";

function lastPath(): string {
  try {
    return localStorage.getItem(LAST_PATH_KEY) ?? "";
  } catch {
    return "";
  }
}

function rememberPath(path: string) {
  try {
    localStorage.setItem(LAST_PATH_KEY, path);
  } catch {
    /* the field is simply empty next time */
  }
}

// One request is shown at a time; later ones wait their turn.
let queue: WebRequest[] = [];
const listeners = new Set<() => void>();

function notify() {
  for (const l of listeners) l();
}

function enqueue(request: WebRequest) {
  queue = [...queue, request];
  notify();
}

/** For `WebDialogHost`: the request being shown, or null. */
export function currentRequest(): WebRequest | null {
  return queue[0] ?? null;
}

export function subscribeRequests(listener: () => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

/** For `WebDialogHost`: answers the request being shown and moves on to the next one. */
export function answerRequest(answer: boolean | string | null) {
  const [done, ...rest] = queue;
  if (!done) return;
  queue = rest;
  if (done.kind === "ask") done.resolve(answer === true);
  else {
    const path = typeof answer === "string" ? answer.trim() : "";
    if (path !== "") rememberPath(path);
    done.resolve(path === "" ? null : path);
  }
  notify();
}

/** Yes/no question; Escape and the system "cancel" answer no. */
export function ask(message: string, opts?: { kind?: AskKind }): Promise<boolean> {
  if (inTauri()) return tauriAsk(message, opts);
  return new Promise((resolve) => enqueue({ kind: "ask", message, level: opts?.kind ?? "info", resolve }));
}

/** A file type the system dialog offers, e.g. `{ name: "DS-ROM", extensions: ["nds"] }`. */
export type FileFilter = { name: string; extensions: string[] };

/** A folder (`directory`) or file to open; null when cancelled or left empty. `filters` narrow the system dialog (a typed path is taken as it is). */
export async function openPath(opts: { directory: boolean; title?: string; filters?: FileFilter[] }): Promise<string | null> {
  if (inTauri()) {
    const chosen = await tauriOpen({ directory: opts.directory, multiple: false, title: opts.title, ...(opts.filters ? { filters: opts.filters } : {}) });
    return typeof chosen === "string" ? chosen : null;
  }
  const title = opts.title ?? (opts.directory ? "Ordner wählen" : "Datei wählen");
  return new Promise((resolve) => enqueue({ kind: "path", title, initial: lastPath(), resolve }));
}

/** Several files to open (only inside Tauri, where the system dialog allows a multiple choice); null when cancelled. In a browser paths are typed into the page instead. */
export async function openFiles(opts: { title?: string }): Promise<string[] | null> {
  if (!inTauri()) return null;
  const chosen = await tauriOpen({ directory: false, multiple: true, title: opts.title });
  if (chosen === null) return null;
  return Array.isArray(chosen) ? chosen : [chosen];
}

/** A file to write; null when cancelled or left empty. A `.nds` default path gets the ROM filter. */
export async function savePath(opts: { defaultPath?: string; title?: string }): Promise<string | null> {
  if (inTauri()) {
    const rom = opts.defaultPath?.toLowerCase().endsWith(".nds") ?? false;
    const chosen = await tauriSave({
      defaultPath: opts.defaultPath,
      ...(rom ? { filters: [{ name: "ROM", extensions: ["nds"] }] } : {}),
    });
    return typeof chosen === "string" ? chosen : null;
  }
  const title = opts.title ?? "Speichern unter";
  return new Promise((resolve) => enqueue({ kind: "path", title, initial: opts.defaultPath ?? lastPath(), resolve }));
}

/** The project folder named by `?project=/a/b` (decoded), or null. */
export function projectFromUrl(search: string): string | null {
  const path = new URLSearchParams(search).get("project");
  return path ? path : null;
}
