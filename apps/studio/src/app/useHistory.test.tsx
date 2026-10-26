// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { history } from "../api";
import type { HistoryDto } from "../api";
import { createEditBus } from "../bus/editBus";
import type { EditBus } from "../bus/editBus";
import { HISTORY_DEBOUNCE_MS, HistoryFeed, useHistory } from "./useHistory";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), history: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useHistory", () => {
  let container: HTMLElement;
  let root: Root;
  let bus: EditBus;
  let shown: HistoryDto;

  function Probe({ revision, enabled = true }: { revision: number; enabled?: boolean }) {
    shown = useHistory(bus, revision, enabled);
    return null;
  }
  const render = (revision: number, enabled = true) => act(async () => root.render(<Probe revision={revision} enabled={enabled} />));
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));

  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(history).mockReset();
    vi.mocked(history).mockResolvedValue({ undo: ["a"], redo: [] });
    bus = createEditBus();
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("reads_the_history_once_after_the_debounce_and_shows_it", async () => {
    await render(0);
    expect(shown).toEqual({ undo: [], redo: [] });
    await wait(HISTORY_DEBOUNCE_MS - 1);
    expect(history).not.toHaveBeenCalled();
    await wait(1);
    expect(history).toHaveBeenCalledOnce();
    expect(shown).toEqual({ undo: ["a"], redo: [] });
  });

  it("reads_again_after_every_bus_topic_but_the_reveal_and_collapses_a_burst_into_one_read", async () => {
    await render(0);
    await wait(HISTORY_DEBOUNCE_MS);
    vi.mocked(history).mockClear();
    vi.mocked(history).mockResolvedValue({ undo: ["a", "b"], redo: [] });
    act(() => {
      bus.emit({ topic: "matrixData" });
      bus.emit({ topic: "eventData" });
      bus.emit({ topic: "mapRefetch", map: 1 });
    });
    await wait(HISTORY_DEBOUNCE_MS);
    expect(history).toHaveBeenCalledOnce();
    expect(shown.undo).toEqual(["a", "b"]);
    vi.mocked(history).mockClear();
    act(() => bus.emit({ topic: "headerReveal" }));
    await wait(HISTORY_DEBOUNCE_MS * 3);
    expect(history).not.toHaveBeenCalled();
  });

  it("reads_again_when_the_revision_changes_for_a_step", async () => {
    await render(0);
    await wait(HISTORY_DEBOUNCE_MS);
    vi.mocked(history).mockClear();
    vi.mocked(history).mockResolvedValue({ undo: [], redo: ["a"] });
    await render(1);
    await wait(HISTORY_DEBOUNCE_MS);
    expect(history).toHaveBeenCalledOnce();
    expect(shown).toEqual({ undo: [], redo: ["a"] });
  });

  it("keeps_the_object_while_the_labels_are_the_same_and_the_last_one_when_a_read_fails", async () => {
    await render(0);
    await wait(HISTORY_DEBOUNCE_MS);
    const before = shown;
    vi.mocked(history).mockResolvedValue({ undo: ["a"], redo: [] });
    act(() => bus.emit({ topic: "matrixData" }));
    await wait(HISTORY_DEBOUNCE_MS);
    expect(shown).toBe(before);
    vi.mocked(history).mockRejectedValue(new Error("no project"));
    act(() => bus.emit({ topic: "matrixData" }));
    await wait(HISTORY_DEBOUNCE_MS);
    expect(shown).toBe(before);
  });

  it("reads_nothing_and_shows_nothing_while_disabled", async () => {
    await render(0);
    await wait(HISTORY_DEBOUNCE_MS);
    expect(shown.undo).toEqual(["a"]);
    vi.mocked(history).mockClear();
    await render(0, false);
    act(() => bus.emit({ topic: "matrixData" }));
    await wait(HISTORY_DEBOUNCE_MS * 2);
    expect(history).not.toHaveBeenCalled();
    expect(shown).toEqual({ undo: [], redo: [] });
  });

  it("a_step_of_another_agent_with_the_same_labels_is_a_new_history", () => {
    const feed = new HistoryFeed();
    const listener = vi.fn();
    feed.subscribe(listener);
    feed.set({ undo: ["Malen"], redo: [], undo_agents: [null], redo_agents: [] });
    const first = feed.get();
    feed.set({ undo: ["Malen"], redo: [], undo_agents: [null], redo_agents: [] });
    expect(feed.get()).toBe(first);
    feed.set({ undo: ["Malen"], redo: [], undo_agents: ["Claude Code"], redo_agents: [] });
    expect(feed.get()).not.toBe(first);
    expect(feed.get().undo_agents).toEqual(["Claude Code"]);
    expect(listener).toHaveBeenCalledTimes(2);
  });
});
