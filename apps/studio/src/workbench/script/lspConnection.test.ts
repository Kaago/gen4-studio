// @vitest-environment jsdom
import { EditorView } from "@codemirror/view";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FakeLspServer } from "./fakeLspServer";
import { MAX_SILENT_RESYNCS, RESTARTED_METHOD, RESYNC_COOLDOWN_MS, connectLsp } from "./lspConnection";
import type { LspConnection } from "./lspConnection";
import { TauriTransport } from "./tauriTransport";

const URI = "file:///p/expanded/scripts/0001.rotom";

describe("connectLsp", () => {
  let server: FakeLspServer;
  let transport: TauriTransport;
  let failing: boolean;
  let connection: LspConnection | null;
  let view: EditorView | null;
  const onGiveUp = vi.fn();

  beforeEach(() => {
    server = new FakeLspServer();
    failing = false;
    connection = null;
    view = null;
    onGiveUp.mockReset();
    transport = new TauriTransport(
      async (body) => {
        if (failing) throw { code: "error", message: "rotom-lsp beendet sich wiederholt", asset: null };
        server.send(body);
      },
      (cb) => {
        server.subscribe(cb);
        return () => server.unsubscribe(cb);
      },
    );
  });
  afterEach(async () => {
    view?.destroy();
    connection?.close();
    vi.useRealTimers();
    await new Promise((resolve) => setTimeout(resolve, 0));
  });

  const connect = () => (connection = connectLsp(transport, { rootUri: "file:///p", onGiveUp }));
  const openDocument = (client: LspConnection["client"]) =>
    (view = new EditorView({ doc: "script Main #1:\n", extensions: [client.plugin(URI, "rotom")], parent: document.body }));
  const settle = () => vi.advanceTimersByTimeAsync(0);

  it("initializes_with_the_project_root_and_opens_documents", async () => {
    connect();
    openDocument(connection!.client);
    await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
    expect(server.calls("initialize")).toHaveLength(1);
    expect(server.calls("initialize")[0]!.params).toMatchObject({ rootUri: "file:///p" });
  });

  it("resyncs_on_the_restart_notification_with_initialize_and_didOpen_for_the_open_documents", async () => {
    connect();
    openDocument(connection!.client);
    await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
    server.notify(RESTARTED_METHOD, undefined);
    await vi.waitFor(() => expect(server.calls("initialize")).toHaveLength(2));
    await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(2));
    expect(server.calls("textDocument/didOpen")[1]!.params).toMatchObject({ textDocument: { uri: URI, text: "script Main #1:\n" } });
    // The client is usable again: the second initialize was answered.
    await expect(connection!.client.initializing).resolves.toBeNull();
    expect(onGiveUp).not.toHaveBeenCalled();
  });

  it("resyncs_when_a_send_fails_and_coalesces_failures_within_the_cooldown", async () => {
    vi.useFakeTimers();
    connect();
    openDocument(connection!.client);
    await settle();
    expect(server.calls("initialize")).toHaveLength(1);
    failing = true;
    connection!.client.notification("textDocument/didSave", { textDocument: { uri: URI } });
    await settle();
    // The first failure resyncs at once (the resync's own initialize fails, too).
    expect(server.calls("initialize")).toHaveLength(1);
    failing = false;
    await vi.advanceTimersByTimeAsync(RESYNC_COOLDOWN_MS - 1);
    expect(server.calls("initialize")).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(1);
    await settle();
    expect(server.calls("initialize")).toHaveLength(2);
    expect(server.calls("textDocument/didOpen")).toHaveLength(2);
    expect(onGiveUp).not.toHaveBeenCalled();
  });

  it("gives_up_after_resyncs_that_got_no_answer_and_stops_resyncing", async () => {
    vi.useFakeTimers();
    failing = true;
    connect();
    for (let i = 0; i < MAX_SILENT_RESYNCS + 3; i++) await vi.advanceTimersByTimeAsync(RESYNC_COOLDOWN_MS);
    expect(onGiveUp).toHaveBeenCalledTimes(1);
    const sends = vi.fn();
    transport.onError(sends);
    await vi.advanceTimersByTimeAsync(RESYNC_COOLDOWN_MS * 5);
    expect(onGiveUp).toHaveBeenCalledTimes(1);
    expect(sends).not.toHaveBeenCalled();
  });

  it("counts_a_message_of_the_server_as_proof_of_life", async () => {
    vi.useFakeTimers();
    connect();
    await settle();
    // Initialize was answered: failures now and then never add up to a give-up.
    for (let i = 0; i < MAX_SILENT_RESYNCS + 3; i++) {
      failing = true;
      connection!.client.notification("textDocument/didSave", { textDocument: { uri: URI } });
      await settle();
      failing = false;
      await vi.advanceTimersByTimeAsync(RESYNC_COOLDOWN_MS);
    }
    expect(onGiveUp).not.toHaveBeenCalled();
    expect(server.calls("initialize").length).toBeGreaterThan(MAX_SILENT_RESYNCS);
  });

  it("close_disconnects_and_releases_the_transport", async () => {
    connect();
    await vi.waitFor(() => expect(server.calls("initialize")).toHaveLength(1));
    connection!.close();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(() => transport.send("{}")).toThrow(/closed/);
    server.notify(RESTARTED_METHOD, undefined);
    await Promise.resolve();
    expect(server.calls("initialize")).toHaveLength(1);
  });
});
