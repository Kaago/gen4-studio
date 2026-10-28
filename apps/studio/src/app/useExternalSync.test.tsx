// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { externalSync } from "../api";
import type { SyncOutcome, SyncProgress } from "../api";
import { subscribeWindowFocus } from "../platform/windowFocus";
import { FOCUS_DELAY_MS, useExternalSync } from "./useExternalSync";
import type { ExternalSyncApi } from "./useExternalSync";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  externalSync: vi.fn(),
}));
vi.mock("../platform/windowFocus", () => ({ subscribeWindowFocus: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const outcome = (over: Partial<SyncOutcome> = {}): SyncOutcome => ({ adopted: [], applied: [], conflicts: [], broken: [], reloaded: [], full: false, ms: 3, ...over });

type Opts = Parameters<typeof useExternalSync>[0];

/** A sync that answers when the test says so; `progress` reports through the hook's callback. */
function pending() {
  let answer: (o: SyncOutcome) => void = () => {};
  let fail: (e: unknown) => void = () => {};
  let progress: (p: SyncProgress) => void = () => {};
  vi.mocked(externalSync).mockImplementationOnce((onProgress) => {
    progress = onProgress;
    return new Promise<SyncOutcome>((resolve, reject) => {
      answer = resolve;
      fail = reject;
    });
  });
  return {
    answer: (o: SyncOutcome = outcome()) => act(async () => answer(o)),
    fail: (e: unknown) => act(async () => fail(e)),
    progress: (p: SyncProgress) => act(async () => progress(p)),
  };
}

describe("useExternalSync", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: ExternalSyncApi;
  let focusListeners: (() => void)[];
  const onOutcome = vi.fn();

  function Probe(options: Partial<Opts>) {
    shown = useExternalSync({ project: "/p", textProject: true, busy: false, onOutcome, ...options });
    return null;
  }
  const show = (options: Partial<Opts> = {}) => act(async () => root.render(<Probe {...options} />));
  /** The window gets the focus (every listener the hook holds now). */
  const focus = () => act(async () => focusListeners.forEach((l) => l()));
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));

  beforeEach(() => {
    vi.useFakeTimers();
    onOutcome.mockReset();
    focusListeners = [];
    vi.mocked(subscribeWindowFocus).mockReset().mockImplementation((listener) => {
      focusListeners.push(listener);
      return () => void (focusListeners = focusListeners.filter((l) => l !== listener));
    });
    vi.mocked(externalSync).mockReset().mockResolvedValue(outcome());
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("runs_on_open_of_a_text_project_only", async () => {
    // A DSPRE folder, or no project: nothing runs, and no focus is listened to.
    await show({ textProject: false });
    await show({ project: null, textProject: false });
    expect(externalSync).not.toHaveBeenCalled();
    expect(focusListeners).toHaveLength(0);

    const run = pending();
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    expect(shown.running).toBe(true);
    await run.progress({ done: 40, total: 400 });
    expect(shown.progress).toEqual({ done: 40, total: 400 });
    const done = outcome({ adopted: [{ key: "moves/0001", label: "Attacke 0001", assets: [{ kind: "Move", index: 1 }] }], full: true });
    await run.answer(done);
    expect(shown.running).toBe(false);
    expect(shown.progress).toBeNull();
    expect(shown.last).toBe(done);
    expect(shown.error).toBeNull();
    expect(onOutcome).toHaveBeenCalledExactlyOnceWith(done);
    // Rendering again with the same project runs nothing more.
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    // Another text project runs again, and forgets what the old one found.
    const next = pending();
    await show({ project: "/q" });
    expect(shown.last).toBeNull();
    expect(externalSync).toHaveBeenCalledTimes(2);
    await next.answer();
  });

  it("focus_runs_after_the_delay", async () => {
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    expect(focusListeners).toHaveLength(1);
    await focus();
    await wait(FOCUS_DELAY_MS - 1);
    expect(externalSync).toHaveBeenCalledOnce();
    await wait(1);
    expect(externalSync).toHaveBeenCalledTimes(2);
    // A failing focus sync says nothing but keeps the error.
    vi.mocked(externalSync).mockRejectedValueOnce({ code: "error", message: "data/ nicht lesbar", asset: null });
    await focus();
    await wait(FOCUS_DELAY_MS);
    expect(externalSync).toHaveBeenCalledTimes(3);
    expect(shown.error).toBe("data/ nicht lesbar");
    expect(onOutcome).toHaveBeenCalledTimes(2);
    // Leaving the text project stops listening.
    await show({ textProject: false });
    expect(focusListeners).toHaveLength(0);
  });

  it("focus_during_a_running_sync_runs_once_more_after_it", async () => {
    const first = pending();
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    await focus();
    await wait(FOCUS_DELAY_MS);
    await focus();
    await wait(FOCUS_DELAY_MS);
    // At most one run at a time.
    expect(externalSync).toHaveBeenCalledOnce();
    const second = pending();
    await first.answer();
    // Exactly one more, after it.
    expect(externalSync).toHaveBeenCalledTimes(2);
    expect(shown.running).toBe(true);
    await second.answer();
    await wait(FOCUS_DELAY_MS * 3);
    expect(externalSync).toHaveBeenCalledTimes(2);
    expect(shown.running).toBe(false);
  });

  it("no_sync_while_building_then_once_after", async () => {
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    await show({ busy: true });
    await focus();
    await wait(FOCUS_DELAY_MS);
    await focus();
    await wait(FOCUS_DELAY_MS * 2);
    expect(externalSync).toHaveBeenCalledOnce();
    await show({ busy: false });
    expect(externalSync).toHaveBeenCalledTimes(2);
    await wait(FOCUS_DELAY_MS * 2);
    expect(externalSync).toHaveBeenCalledTimes(2);
    // A build without a focus meanwhile runs nothing after it.
    await show({ busy: true });
    await show({ busy: false });
    expect(externalSync).toHaveBeenCalledTimes(2);
  });

  it("many_focus_events_make_one_run", async () => {
    await show();
    expect(externalSync).toHaveBeenCalledOnce();
    for (let i = 0; i < 10; i++) {
      await focus();
      await wait(100);
    }
    expect(externalSync).toHaveBeenCalledOnce();
    await wait(FOCUS_DELAY_MS);
    expect(externalSync).toHaveBeenCalledTimes(2);
  });

  it("run_answers_the_outcome_and_waits_for_a_running_sync", async () => {
    const first = pending();
    await show();
    let answered: SyncOutcome | null | undefined;
    const second = pending();
    await act(async () => void shown.run().then((o) => (answered = o)));
    expect(externalSync).toHaveBeenCalledOnce();
    await first.answer();
    expect(externalSync).toHaveBeenCalledTimes(2);
    expect(answered).toBeUndefined();
    const mine = outcome({ conflicts: [{ key: "moves/0001", label: "Attacke 0001", assets: [], unsaved: false, other: "dspre" }] });
    await second.answer(mine);
    expect(answered).toBe(mine);
    // A failing run answers null; its error is kept.
    vi.mocked(externalSync).mockRejectedValueOnce({ code: "error", message: "kaputt", asset: null });
    await act(async () => void shown.run().then((o) => (answered = o)));
    expect(answered).toBeNull();
    expect(shown.error).toBe("kaputt");
  });
});
