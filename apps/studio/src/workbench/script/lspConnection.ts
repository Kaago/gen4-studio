import { LSPClient, languageServerExtensions } from "@codemirror/lsp-client";
import { lspExtras } from "./lspExtras";
import type { TauriTransport } from "./tauriTransport";

/**
 * The synthetic notification the backend pushes after it restarted rotom-lsp (`lsp.rs`, `RESTARTED_NOTIFICATION`).
 * The new server has lost `initialize` and every opened document; it is not part of the protocol.
 */
export const RESTARTED_METHOD = "$/g4/restarted";

/** The least time between two resyncs caused by failing sends (a dead server fails every one of them). */
export const RESYNC_COOLDOWN_MS = 2000;
/** Resyncs in a row that got no message from the server; after that the connection gives up. */
export const MAX_SILENT_RESYNCS = 5;

export interface LspConnection {
  readonly client: LSPClient;
  /** Tells the server that files changed on disk (`workspace/didChangeWatchedFiles`); `type` 1 created, 2 changed. */
  filesChanged(changes: { uri: string; type: 1 | 2 }[]): void;
  /** Disconnects the client, then releases the transport (one tick later). Does not stop the server (`lspStop` does). */
  close(): void;
}

/**
 * A client on `transport` that starts over (`disconnect`, `connect`: `initialize` again, `didOpen` for every open
 * document) when the backend restarted the server and when a send failed (the server lost its state, see
 * `TauriTransport`). A restart notification resyncs at once; send failures at most once per `RESYNC_COOLDOWN_MS`,
 * and `onGiveUp` is called when `MAX_SILENT_RESYNCS` resyncs in a row brought no answer.
 */
export function connectLsp(transport: TauriTransport, options: { rootUri: string; onGiveUp: () => void }): LspConnection {
  let closed = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let lastResync = Number.NEGATIVE_INFINITY;
  let silent = 0;

  const client = new LSPClient({
    rootUri: options.rootUri,
    extensions: [...languageServerExtensions(), lspExtras],
    notificationHandlers: {
      [RESTARTED_METHOD]: () => {
        requestResync(true);
        return true;
      },
    },
  });

  function connect() {
    client.connect(transport);
    // A failed initialize is reported by the next resync or the give-up, not as an unhandled rejection.
    client.initializing.catch(() => {});
  }

  function resync() {
    timer = null;
    if (closed) return;
    client.disconnect();
    if (++silent > MAX_SILENT_RESYNCS) {
      closed = true;
      options.onGiveUp();
      return;
    }
    lastResync = Date.now();
    connect();
  }

  function requestResync(now: boolean) {
    if (closed || timer !== null) return;
    const wait = now ? 0 : Math.max(0, lastResync + RESYNC_COOLDOWN_MS - Date.now());
    if (wait === 0) resync();
    else timer = setTimeout(resync, wait);
  }

  // Any message of the server proves it is there; the synthetic restart notice does not.
  transport.subscribe((body) => {
    if (!body.includes(RESTARTED_METHOD)) silent = 0;
  });
  transport.onError(() => requestResync(false));
  connect();

  return {
    client,
    filesChanged(changes) {
      if (!closed) client.notification("workspace/didChangeWatchedFiles", { changes });
    },
    close() {
      closed = true;
      if (timer !== null) clearTimeout(timer);
      timer = null;
      client.disconnect();
      // The client sends its notifications (a `didClose` from a destroyed editor, say) in a promise callback that may
      // still be queued: a transport disposed right now would make that send throw.
      setTimeout(() => transport.dispose(), 0);
    },
  };
}
