// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const listen = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import type { AgentChangedEvent, AgentEvent, AgentStatus } from "../api";
import { useAgents } from "./useAgents";
import type { Agents } from "./useAgents";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const status = (readOnly: boolean, connections = 1): AgentStatus => ({
  enabled: true,
  expert: false,
  read_only: readOnly,
  port: 49152,
  bridge: false,
  connections: Array.from({ length: connections }, (_, i) => ({ id: i + 1, client: "Claude Code", since_ms: 0 })),
  question: null,
});

describe("useAgents", () => {
  let container: HTMLElement;
  let root: Root;
  let agents: Agents;
  let emit: (event: AgentEvent) => void = () => {};
  let current: AgentStatus;
  const unlisten = vi.fn();
  const onChanged = vi.fn();

  function Probe({ onChanged }: { onChanged: (event: AgentChangedEvent) => void }) {
    agents = useAgents({ onChanged });
    return null;
  }
  const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(0)));
  const render = async (fn = onChanged) => {
    await act(async () => root.render(<Probe onChanged={fn} />));
    await settle();
  };

  beforeEach(() => {
    vi.useFakeTimers();
    current = status(false);
    invoke.mockReset();
    invoke.mockImplementation(async (cmd: string) => (cmd === "agent_status" ? current : undefined));
    listen.mockReset();
    unlisten.mockReset();
    listen.mockImplementation(async (_name: string, fn: (e: { payload: AgentEvent }) => void) => {
      emit = (payload) => fn({ payload });
      return unlisten;
    });
    onChanged.mockReset();
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("changed_event_calls_onChanged_with_assets", async () => {
    await render();
    const first: AgentEvent = { kind: "changed", seq: 3, client: "Claude Code", tool: "set_header", label: "Header 3 geändert", assets: [{ kind: "Event", index: 3 }] };
    const second: AgentEvent = { kind: "changed", seq: 4, client: "Claude Code", tool: "create_area", label: null, assets: [{ kind: "Matrix", index: 2 }, { kind: "HeaderTable", index: 0 }] };
    await act(async () => {
      emit(first);
      emit({ kind: "status", seq: 5 });
      emit(second);
    });
    expect(onChanged.mock.calls.map(([event]) => event)).toEqual([first, second]);
    expect(onChanged.mock.calls[1]![0].assets).toEqual([{ kind: "Matrix", index: 2 }, { kind: "HeaderTable", index: 0 }]);
  });

  it("calls_the_latest_onChanged", async () => {
    await render();
    const later = vi.fn();
    await render(later);
    await act(async () => emit({ kind: "changed", seq: 1, client: "Agent", tool: "undo", label: null, assets: [] }));
    expect(onChanged).not.toHaveBeenCalled();
    expect(later).toHaveBeenCalledTimes(1);
    // A new callback does not subscribe again.
    expect(listen).toHaveBeenCalledTimes(1);
  });

  it("status_event_rereads_status", async () => {
    await render();
    expect(agents.status).toEqual(status(false));
    current = status(true, 2);
    invoke.mockClear();
    await act(async () => emit({ kind: "status", seq: 2 }));
    await settle();
    expect(invoke).toHaveBeenCalledWith("agent_status");
    expect(agents.status).toEqual(status(true, 2));
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("acts_and_reads_the_status_afterwards", async () => {
    await render();
    current = status(true);
    await act(() => agents.setReadOnly(true));
    expect(invoke).toHaveBeenCalledWith("agent_set_read_only", { on: true });
    expect(agents.status).toEqual(status(true));
    await act(() => agents.answer(4, "write"));
    expect(invoke).toHaveBeenCalledWith("agent_answer", { question: 4, answer: "write" });
    current = status(true, 0);
    await act(() => agents.disconnect());
    expect(invoke).toHaveBeenCalledWith("agent_disconnect");
    expect(agents.status).toEqual(status(true, 0));
    current = status(false, 3);
    await act(() => agents.refresh());
    expect(agents.status).toEqual(status(false, 3));
  });

  it("rethrows_a_failed_action", async () => {
    await render();
    const error = { code: "error", message: "Die Frage ist nicht mehr offen", asset: null };
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "agent_answer") throw error;
      return current;
    });
    await act(async () => {
      await expect(agents.answer(9, "deny")).rejects.toBe(error);
    });
  });

  it("unsubscribes_on_unmount", async () => {
    await render();
    act(() => root.unmount());
    expect(unlisten).toHaveBeenCalledTimes(1);
    root = createRoot(container);
  });
});
