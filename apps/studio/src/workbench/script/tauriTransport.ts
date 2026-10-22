import type { Transport } from "@codemirror/lsp-client";
import { lspSend, lspStart } from "../../api";

/**
 * The `@codemirror/lsp-client` transport over the Tauri commands `lsp_start`, `lsp_send` and the message channel.
 *
 * `Transport.send` is synchronous, the backend call is not: a failed `lspSend` (queue full, server dead) cannot
 * throw into the client, so it goes to the `onError` listeners instead. The server has lost its state after such an
 * error, and also after an automatic restart (which gives no signal at all); the owner is expected to disconnect
 * and reconnect the client, which repeats `initialize` and `didOpen`.
 */
export class TauriTransport implements Transport {
  private readonly handlers = new Set<(value: string) => void>();
  private readonly errorListeners = new Set<(error: unknown) => void>();
  private unsubscribeSource: (() => void) | null;

  constructor(
    private readonly sendBody: (body: string) => Promise<void>,
    subscribe: (cb: (body: string) => void) => () => void,
  ) {
    this.unsubscribeSource = subscribe((body) => {
      for (const handler of [...this.handlers]) handler(body);
    });
  }

  send(message: string): void {
    if (!this.unsubscribeSource) throw new Error("rotom-lsp transport is closed");
    this.sendBody(message).catch((error: unknown) => {
      for (const listener of [...this.errorListeners]) listener(error);
    });
  }

  subscribe(handler: (value: string) => void): void {
    this.handlers.add(handler);
  }

  unsubscribe(handler: (value: string) => void): void {
    this.handlers.delete(handler);
  }

  /** Calls `listener` with the rejection of every failed send; returns the function that removes it. */
  onError(listener: (error: unknown) => void): () => void {
    this.errorListeners.add(listener);
    return () => void this.errorListeners.delete(listener);
  }

  /** Detaches from the message source; later sends throw. Does not stop the server (`lspStop` does). */
  dispose(): void {
    this.unsubscribeSource?.();
    this.unsubscribeSource = null;
    this.handlers.clear();
    this.errorListeners.clear();
  }
}

/** Starts rotom-lsp and returns a transport on a fresh channel. Rejects with the backend error (`tool_missing`). */
export async function createTauriTransport(): Promise<TauriTransport> {
  const listeners = new Set<(body: string) => void>();
  await lspStart((body) => listeners.forEach((l) => l(body)));
  return new TauriTransport(lspSend, (cb) => {
    listeners.add(cb);
    return () => void listeners.delete(cb);
  });
}
