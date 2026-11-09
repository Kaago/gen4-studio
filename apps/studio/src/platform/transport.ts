import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { t } from "../i18n";

/** Path the Vite dev server proxies to the dev bridge (see vite.config.ts). */
export const BRIDGE_PREFIX = "/__bridge";

/** Same shape as the backend's `CmdError`; Tauri rejects with this object itself. */
interface BridgeError {
  code: string;
  message: string;
  asset: string | null;
}

const bridgeError = (message: string): BridgeError => ({ code: "error", message, asset: null });

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function isCmdErrorBody(v: unknown): v is BridgeError {
  return typeof v === "object" && v !== null && "code" in v && "message" in v;
}

async function invokeBridge<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${BRIDGE_PREFIX}/invoke/${cmd}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args ?? {}),
    });
  } catch {
    throw bridgeError(t("platform:transport.bridgeDown"));
  }
  if (res.status === 200) {
    const type = res.headers.get("Content-Type") ?? "";
    if (type.startsWith("application/octet-stream")) return (await res.arrayBuffer()) as T;
    return (await res.json()) as T;
  }
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    // Not JSON (404, 413, ...): fall through to the generic error.
  }
  throw isCmdErrorBody(body) ? body : bridgeError(t("platform:transport.bridgeHttp", { status: res.status }));
}

/**
 * Runs a backend command: through Tauri inside the app, through the dev bridge in a plain
 * browser during development. Rejects with a `CmdError`-shaped object in both cases.
 */
export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri()) return tauriInvoke<T>(cmd, args);
  // Removed from production builds: the bridge only exists in development.
  if (import.meta.env.DEV) return invokeBridge<T>(cmd, args);
  return Promise.reject(bridgeError(t("platform:transport.noBackend")));
}
