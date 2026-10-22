import type { Transport } from "@codemirror/lsp-client";

/** A JSON-RPC message as the fake server sees it (requests have an `id`, notifications do not). */
export interface Rpc {
  id?: number;
  method: string;
  params?: unknown;
}

/**
 * An in-memory language server for tests: a `Transport` that answers requests from `methods` (a handler may return a promise; an unknown method
 * answers `null`), records everything it receives, and can push notifications. Only used by tests.
 */
export class FakeLspServer implements Transport {
  readonly received: Rpc[] = [];
  readonly methods: Record<string, (params: any) => unknown | Promise<unknown>> = {};
  capabilities: Record<string, unknown> = {};
  private readonly handlers = new Set<(value: string) => void>();

  send(message: string): void {
    const msg = JSON.parse(message) as Rpc;
    this.received.push(msg);
    if (msg.id === undefined || !msg.method) return;
    const id = msg.id;
    queueMicrotask(() => {
      const answer =
        msg.method === "initialize" ? { capabilities: { textDocumentSync: 1, ...this.capabilities } } : (this.methods[msg.method]?.(msg.params) ?? null);
      void Promise.resolve(answer).then((result) => this.emit({ jsonrpc: "2.0", id, result }));
    });
  }

  subscribe(handler: (value: string) => void): void {
    this.handlers.add(handler);
  }

  unsubscribe(handler: (value: string) => void): void {
    this.handlers.delete(handler);
  }

  /** Pushes a server notification to the client. */
  notify(method: string, params: unknown): void {
    this.emit({ jsonrpc: "2.0", method, params });
  }

  /** The requests and notifications received for `method`. */
  calls(method: string): Rpc[] {
    return this.received.filter((m) => m.method === method);
  }

  private emit(value: unknown): void {
    const text = JSON.stringify(value);
    for (const handler of [...this.handlers]) handler(text);
  }
}
