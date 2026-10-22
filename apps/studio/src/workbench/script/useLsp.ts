import type { LSPClient } from "@codemirror/lsp-client";
import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, isCmdError, lspStop } from "../../api";
import type { ProjectInfo } from "../../api";
import { rootUri } from "./documents";
import { connectLsp } from "./lspConnection";
import type { LspConnection } from "./lspConnection";
import { createTauriTransport } from "./tauriTransport";

/** "starting": the server is being started; "missing": no rotom-lsp binary; "stopped": it did not start, died or does not answer. */
export type LspState = "starting" | "running" | "stopped" | "missing";

export interface LspStatus {
  /** The connected client while the state is "running". */
  client: LSPClient | null;
  state: LspState;
  /** The backend's words for "missing" and "stopped". */
  note: string | null;
  /** Starts the server again (the header's button for "stopped"). */
  restart: () => void;
}

interface Connected {
  client: LSPClient | null;
  state: LspState;
  note: string | null;
}

const IDLE: Connected = { client: null, state: "starting", note: null };

/**
 * rotom-lsp for the open project, started while `enabled` (the project is adopted) and ended with the project. A new
 * channel and transport for every start; the client resyncs itself (see `connectLsp`). The editor works without it: a
 * missing or dead server only turns the language services off.
 */
export function useLsp(project: ProjectInfo | null, enabled: boolean): LspStatus {
  const [connected, setConnected] = useState<Connected>(IDLE);
  const [attempt, setAttempt] = useState(0);
  /** The newest start; a start that finishes late and finds a newer one must not stop the server that one is bringing up. */
  const newest = useRef(0);
  /** The last `lspStop`: a start waits for it, so it cannot stop the server it just started. */
  const stopping = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    if (!project || !enabled) {
      setConnected(IDLE);
      return;
    }
    let cancelled = false;
    let connection: LspConnection | null = null;
    const mine = ++newest.current;
    setConnected(IDLE);
    void (async () => {
      await stopping.current;
      if (cancelled) return;
      let transport;
      try {
        transport = await createTauriTransport();
      } catch (e) {
        if (!cancelled) setConnected({ client: null, state: isCmdError(e) && e.code === "tool_missing" ? "missing" : "stopped", note: errorText(e) });
        return;
      }
      if (cancelled) {
        // Left before the server was up: drop the channel; the server only stops when no newer start is on its way to replace it.
        transport.dispose();
        if (newest.current === mine) stopping.current = lspStop().catch(() => {});
        return;
      }
      connection = connectLsp(transport, {
        rootUri: rootUri(project.root),
        onGiveUp: () => {
          if (!cancelled) setConnected({ client: null, state: "stopped", note: "rotom-lsp antwortet nicht" });
        },
      });
      setConnected({ client: connection.client, state: "running", note: null });
    })();
    return () => {
      cancelled = true;
      if (connection) {
        // The client first, then the transport (inside `close`), then the server.
        connection.close();
        stopping.current = lspStop().catch(() => {});
      }
    };
  }, [project, enabled, attempt]);

  const restart = useCallback(() => setAttempt((n) => n + 1), []);
  return { ...connected, restart };
}
