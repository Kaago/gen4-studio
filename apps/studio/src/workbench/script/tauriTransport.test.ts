import { beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({ lspStart: vi.fn(), lspSend: vi.fn() }));
vi.mock("../../api", () => api);

import { TauriTransport, createTauriTransport } from "./tauriTransport";

/** A subscribe function backed by a plain listener set, like the Channel behind `createTauriTransport`. */
function source() {
  const listeners = new Set<(s: string) => void>();
  return {
    listeners,
    emit: (s: string) => listeners.forEach((l) => l(s)),
    subscribe: (cb: (s: string) => void) => {
      listeners.add(cb);
      return () => void listeners.delete(cb);
    },
  };
}

describe("TauriTransport", () => {
  it("sends_each_message_through_the_send_function_in_order", () => {
    const send = vi.fn(async (_: string) => {});
    const t = new TauriTransport(send, source().subscribe);
    t.send("a");
    t.send("b");
    expect(send.mock.calls).toEqual([["a"], ["b"]]);
  });

  it("delivers_channel_messages_to_every_subscribed_handler_until_unsubscribed", () => {
    const src = source();
    const t = new TauriTransport(async () => {}, src.subscribe);
    const one = vi.fn();
    const two = vi.fn();
    t.subscribe(one);
    t.subscribe(two);
    src.emit("m1");
    t.unsubscribe(one);
    src.emit("m2");
    expect(one.mock.calls).toEqual([["m1"]]);
    expect(two.mock.calls).toEqual([["m1"], ["m2"]]);
  });

  it("reports_a_rejected_send_to_the_error_listeners_so_the_owner_can_resynchronise", async () => {
    const failure = { code: "error", message: "rotom-lsp beendet sich wiederholt" };
    const t = new TauriTransport(async () => Promise.reject(failure), source().subscribe);
    const onError = vi.fn();
    t.onError(onError);
    t.send("x");
    await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(failure));
  });

  it("stops_reporting_errors_to_a_listener_that_was_removed", async () => {
    const t = new TauriTransport(async () => Promise.reject(new Error("e")), source().subscribe);
    const onError = vi.fn();
    const off = t.onError(onError);
    off();
    t.send("x");
    await Promise.resolve();
    await Promise.resolve();
    expect(onError).not.toHaveBeenCalled();
  });

  it("throws_on_send_after_dispose_and_leaves_the_source", () => {
    const src = source();
    const send = vi.fn(async (_: string) => {});
    const t = new TauriTransport(send, src.subscribe);
    t.dispose();
    expect(src.listeners.size).toBe(0);
    expect(() => t.send("x")).toThrow(/closed/);
    expect(send).not.toHaveBeenCalled();
  });
});

describe("createTauriTransport", () => {
  beforeEach(() => {
    api.lspStart.mockReset();
    api.lspSend.mockReset();
  });

  it("starts_the_server_and_forwards_channel_messages_to_the_handler", async () => {
    let onMessage: ((s: string) => void) | undefined;
    api.lspStart.mockImplementation(async (cb: (s: string) => void) => void (onMessage = cb));
    api.lspSend.mockResolvedValue(undefined);
    const t = await createTauriTransport();
    const handler = vi.fn();
    t.subscribe(handler);
    onMessage?.('{"jsonrpc":"2.0"}');
    expect(handler).toHaveBeenCalledWith('{"jsonrpc":"2.0"}');
    t.send("body");
    expect(api.lspSend).toHaveBeenCalledWith("body");
  });

  it("rejects_with_the_backend_error_when_rotom_lsp_is_missing", async () => {
    const missing = { code: "tool_missing", message: "rotom-lsp fehlt — Completion und Diagnosen aus", asset: null };
    api.lspStart.mockRejectedValue(missing);
    await expect(createTauriTransport()).rejects.toBe(missing);
  });
});
