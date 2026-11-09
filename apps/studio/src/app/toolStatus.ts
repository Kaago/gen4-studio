import { t } from "../i18n";

/** The language server as the script panel last saw it: "aus" while the project is not adopted or the panel is gone. */
export type LspToolState = "aus" | "startet" | "bereit" | "fehlt" | "fehler";
/** apicula as the map pictures found it: "unbekannt" until a picture was made or failed for a missing tool. */
export type ApiculaState = "unbekannt" | "bereit" | "fehlt";

export interface LspTool {
  state: LspToolState;
  /** The backend's words for "fehlt" and "fehler". */
  note?: string;
  /** Starts the server again (the panel's own restart). */
  restart?: () => void;
}

export interface ToolStatus {
  /** null while the script panel was never mounted: the server was not started. */
  lsp: LspTool | null;
  apicula: ApiculaState;
}

/**
 * What Technik › Werkzeuge shows. There is no command that asks the backend for its tools, so the parts that use them
 * write here: the script panel the language server's state, the map pictures apicula's. No paths (none are known).
 */
export interface ToolStatusStore {
  subscribe(onChange: () => void): () => void;
  /** The same object until something changed (as `useSyncExternalStore` wants it). */
  get(): ToolStatus;
  set(partial: Partial<ToolStatus>): void;
}

const sameLsp = (a: LspTool | null, b: LspTool | null) => a === b || (a !== null && b !== null && a.state === b.state && a.note === b.note && a.restart === b.restart);

/** A store whose methods work taken off the object. */
export function createToolStatus(): ToolStatusStore {
  let status: ToolStatus = { lsp: null, apicula: "unbekannt" };
  const listeners = new Set<() => void>();
  return {
    subscribe(onChange) {
      listeners.add(onChange);
      return () => void listeners.delete(onChange);
    },
    get: () => status,
    set(partial) {
      const next = { ...status, ...partial };
      if (sameLsp(next.lsp, status.lsp) && next.apicula === status.apicula) return;
      status = next;
      for (const listener of [...listeners]) listener();
    },
  };
}

/** The app's one store. */
export const toolStatus = createToolStatus();

/** How many of the two tools are known to work. */
export function readyCount(status: ToolStatus): number {
  return (status.lsp?.state === "bereit" ? 1 : 0) + (status.apicula === "bereit" ? 1 : 0);
}

/** "2 von 2 bereit". */
export function toolsText(status: ToolStatus): string {
  return t("app:toolStatus.ready", { ready: readyCount(status), total: 2 });
}
