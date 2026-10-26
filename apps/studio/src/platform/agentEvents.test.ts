import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const tauri = vi.hoisted(() => ({ on: false }));
const listen = vi.hoisted(() => vi.fn());
vi.mock("./transport", () => ({ invoke, inTauri: () => tauri.on }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import type { AgentEvent, AgentPoll, AgentStatus } from "../api";
import { AGENT_POLL_MS, subscribeAgentEvents } from "./agentEvents";

const status = (connections: number): AgentStatus => ({
  enabled: true,
  expert: false,
  read_only: false,
  port: null,
  server_error: null,
  bridge: true,
  connections: Array.from({ length: connections }, (_, i) => ({ id: i + 1, client: "Claude Code", since_ms: 0 })),
  question: null,
});
const changed = (seq: number, index: number): AgentEvent => ({
  kind: "changed",
  seq,
  client: "Claude Code",
  tool: "set_header",
  label: "Header geändert",
  assets: [{ kind: "Event", index }],
});
const pollAnswer = (seq: number, events: AgentEvent[], connections = 0): AgentPoll => ({ seq, events, status: status(connections) });

/** Lets the pending promises of a poll settle without moving the clock. */
const settle = () => vi.advanceTimersByTimeAsync(0);

describe("subscribeAgentEvents in the browser mode", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    tauri.on = false;
    invoke.mockReset();
    listen.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  it("polls_from_last_seq", async () => {
    const answers = [pollAnswer(3, [changed(2, 9)]), pollAnswer(5, [changed(4, 1), { kind: "status", seq: 5 }], 1), pollAnswer(5, [], 1)];
    invoke.mockImplementation(async () => answers.shift() ?? pollAnswer(5, [], 1));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    // The first poll brings the status at once; the events kept from before subscribing are past.
    expect(invoke).toHaveBeenNthCalledWith(1, "agent_poll", { since: 0 });
    expect(onEvents).toHaveBeenCalledTimes(1);
    expect(onEvents).toHaveBeenLastCalledWith([], status(0));
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS - 1);
    expect(invoke).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(invoke).toHaveBeenNthCalledWith(2, "agent_poll", { since: 3 });
    expect(onEvents).toHaveBeenCalledTimes(2);
    expect(onEvents).toHaveBeenLastCalledWith([changed(4, 1), { kind: "status", seq: 5 }], status(1));
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    expect(invoke).toHaveBeenNthCalledWith(3, "agent_poll", { since: 5 });
    // Nothing new: no call.
    expect(onEvents).toHaveBeenCalledTimes(2);
    stop();
  });

  it("stops_on_unsubscribe", async () => {
    let resolve: (poll: AgentPoll) => void = () => {};
    invoke.mockImplementation(() => new Promise<AgentPoll>((r) => (resolve = r)));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    expect(invoke).toHaveBeenCalledTimes(1);
    // Unsubscribed while the poll is under way: its answer is ignored and no further poll follows.
    stop();
    resolve(pollAnswer(2, [changed(1, 3)], 1));
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS * 4);
    expect(onEvents).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("stops_between_polls", async () => {
    invoke.mockResolvedValue(pollAnswer(1, []));
    const stop = subscribeAgentEvents(vi.fn());
    await settle();
    stop();
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS * 4);
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("polls_again_only_after_the_previous_poll_answered", async () => {
    let resolve: (poll: AgentPoll) => void = () => {};
    invoke.mockImplementation(() => new Promise<AgentPoll>((r) => (resolve = r)));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS * 5);
    expect(invoke).toHaveBeenCalledTimes(1);
    resolve(pollAnswer(1, []));
    await settle();
    expect(onEvents).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    expect(invoke).toHaveBeenCalledTimes(2);
    stop();
  });

  it("keeps_polling_after_an_error_silently", async () => {
    invoke.mockRejectedValueOnce({ code: "error", message: "Brücke nicht erreichbar", asset: null });
    invoke.mockResolvedValue(pollAnswer(4, [], 1));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    expect(onEvents).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    // The first answer that arrives counts as the first poll: it brings the status.
    expect(invoke).toHaveBeenLastCalledWith("agent_poll", { since: 0 });
    expect(onEvents).toHaveBeenCalledWith([], status(1));
    stop();
  });

  it("signals_a_gap_when_events_were_lost", async () => {
    // More than the hub keeps happened between two polls: the first event returned is not the next one.
    const answers = [pollAnswer(3, []), pollAnswer(100, [changed(37, 1), changed(100, 2)], 1), pollAnswer(101, [changed(101, 3)], 1)];
    invoke.mockImplementation(async () => answers.shift() ?? pollAnswer(101, [], 1));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    expect(onEvents).toHaveBeenLastCalledWith([], status(0));
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    // The kept events still come, flagged: what came before them is unknown.
    expect(onEvents).toHaveBeenLastCalledWith([changed(37, 1), changed(100, 2)], status(1), { gap: true });
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    // The next one follows on: no gap.
    expect(onEvents).toHaveBeenCalledTimes(3);
    expect(onEvents.mock.calls[2]).toEqual([[changed(101, 3)], status(1)]);
    stop();
  });

  it("follows_a_restarted_hub_whose_seq_went_back", async () => {
    // A restarted bridge counts from 1 again: nothing after 40 yet, then the poll goes on from its seq.
    const answers = [pollAnswer(40, []), pollAnswer(2, [], 1)];
    invoke.mockImplementation(async () => answers.shift() ?? pollAnswer(3, [changed(3, 6)]));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    expect(invoke).toHaveBeenLastCalledWith("agent_poll", { since: 40 });
    // The restart itself delivers the new hub's status, without events.
    expect(onEvents).toHaveBeenCalledTimes(2);
    expect(onEvents).toHaveBeenNthCalledWith(2, [], status(1));
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS);
    expect(invoke).toHaveBeenLastCalledWith("agent_poll", { since: 2 });
    expect(onEvents).toHaveBeenCalledTimes(3);
    expect(onEvents).toHaveBeenLastCalledWith([changed(3, 6)], status(0));
    stop();
  });
});

describe("subscribeAgentEvents inside Tauri", () => {
  let handler: (e: { payload: AgentEvent }) => void = () => {};
  const unlisten = vi.fn();

  beforeEach(() => {
    vi.useFakeTimers();
    tauri.on = true;
    invoke.mockReset();
    listen.mockReset();
    unlisten.mockReset();
    listen.mockImplementation(async (_name: string, fn: typeof handler) => {
      handler = fn;
      return unlisten;
    });
  });
  afterEach(() => vi.useRealTimers());

  it("listens_and_rereads_the_status_on_a_status_event", async () => {
    invoke.mockResolvedValue(status(0));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    expect(listen).toHaveBeenCalledWith("agent-event", expect.any(Function));
    // Once listening, the status is read once so it is there before the first event.
    expect(invoke).toHaveBeenCalledWith("agent_status");
    expect(onEvents).toHaveBeenLastCalledWith([], status(0));
    invoke.mockResolvedValue(status(1));
    handler({ payload: { kind: "status", seq: 7 } });
    await settle();
    expect(onEvents).toHaveBeenLastCalledWith([{ kind: "status", seq: 7 }], status(1));
    // A change comes as it is, without reading the status; a lower seq is not dropped.
    handler({ payload: changed(6, 2) });
    expect(onEvents).toHaveBeenLastCalledWith([changed(6, 2)], null);
    expect(invoke).not.toHaveBeenCalledWith("agent_poll", expect.anything());
    await vi.advanceTimersByTimeAsync(AGENT_POLL_MS * 4);
    expect(invoke).not.toHaveBeenCalledWith("agent_poll", expect.anything());
    stop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("passes_on_only_the_newest_status_read", async () => {
    invoke.mockResolvedValue(status(0));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    const reads: ((s: AgentStatus) => void)[] = [];
    invoke.mockImplementation(() => new Promise<AgentStatus>((r) => reads.push(r)));
    handler({ payload: { kind: "status", seq: 2 } });
    handler({ payload: { kind: "status", seq: 3 } });
    // The second read answers first; the first, older one arrives late and brings its event without a status.
    reads[1]!(status(2));
    await settle();
    reads[0]!(status(1));
    await settle();
    expect(onEvents.mock.calls.slice(1)).toEqual([
      [[{ kind: "status", seq: 3 }], status(2)],
      [[{ kind: "status", seq: 2 }], null],
    ]);
    stop();
  });

  it("unlistens_when_stopped_before_listening_began", async () => {
    invoke.mockResolvedValue(status(0));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    stop();
    await settle();
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(onEvents).not.toHaveBeenCalled();
  });

  it("ignores_a_status_read_that_answers_after_unsubscribe", async () => {
    invoke.mockResolvedValue(status(0));
    const onEvents = vi.fn();
    const stop = subscribeAgentEvents(onEvents);
    await settle();
    onEvents.mockClear();
    let resolve: (s: AgentStatus) => void = () => {};
    invoke.mockImplementation(() => new Promise<AgentStatus>((r) => (resolve = r)));
    handler({ payload: { kind: "status", seq: 2 } });
    stop();
    resolve(status(1));
    await settle();
    expect(onEvents).not.toHaveBeenCalled();
  });
});
