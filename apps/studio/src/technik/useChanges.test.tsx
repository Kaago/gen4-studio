// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { changedFromOriginal } from "../api";
import { FOCUS_DELAY_MS } from "../app/useExternalSync";
import { subscribeWindowFocus } from "../platform/windowFocus";
import type { ChangedProgress, ChangedReport } from "../api";
import { report } from "./changesTesting";
import { useChanges } from "./useChanges";
import type { ChangesApi, UseChangesOptions } from "./useChanges";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  changedFromOriginal: vi.fn(),
}));
vi.mock("../platform/windowFocus", () => ({ subscribeWindowFocus: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** The last of `items` (the last render recorded). */
const last = <T,>(items: readonly T[]): T | undefined => items[items.length - 1];

/** Two base ROMs by SHA-1. */
const SHA_X = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
const SHA_Y = "0".repeat(40);

/** A compare that answers when the test says so; `progress` reports through the hook's callback. */
function pending() {
  let answer: (r: ChangedReport) => void = () => {};
  let fail: (e: unknown) => void = () => {};
  let progress: (p: ChangedProgress) => void = () => {};
  vi.mocked(changedFromOriginal).mockImplementationOnce((onProgress) => {
    progress = onProgress;
    return new Promise<ChangedReport>((resolve, reject) => {
      answer = resolve;
      fail = reject;
    });
  });
  return {
    answer: (r: ChangedReport) => act(async () => answer(r)),
    fail: (e: unknown) => act(async () => fail(e)),
    progress: (p: ChangedProgress) => act(async () => progress(p)),
  };
}

describe("useChanges", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: ChangesApi;

  function Probe(options: Partial<UseChangesOptions>) {
    shown = useChanges({ project: "/a", shown: true, base: SHA_X, revision: 0, ...options });
    return null;
  }
  const show = (options: Partial<UseChangesOptions> = {}) => act(async () => root.render(<Probe {...options} />));

  let focusListeners: (() => void)[];
  /** The window gets the focus back. */
  const focus = () => act(async () => focusListeners.forEach((l) => l()));
  const wait = (ms: number) => act(async () => void vi.advanceTimersByTime(ms));

  beforeEach(() => {
    focusListeners = [];
    vi.mocked(subscribeWindowFocus)
      .mockReset()
      .mockImplementation((listener) => {
        focusListeners.push(listener);
        return () => void (focusListeners = focusListeners.filter((l) => l !== listener));
      });
    vi.mocked(changedFromOriginal).mockReset().mockResolvedValue(report());
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("compares_once_when_shown_and_reports_progress", async () => {
    // Technik › Änderungen not on screen, or no base ROM: nothing is compared (it may hash every member).
    await show({ shown: false });
    await show({ base: null });
    expect(changedFromOriginal).not.toHaveBeenCalled();
    expect(shown.report).toBeNull();

    const run = pending();
    await show();
    expect(changedFromOriginal).toHaveBeenCalledOnce();
    expect(shown.busy).toBe(true);
    expect(shown.progress).toBeNull();
    await run.progress({ done: 3, total: 10 });
    expect(shown.progress).toEqual({ done: 3, total: 10 });
    await run.answer(report({ files: 10 }));
    expect(shown.busy).toBe(false);
    expect(shown.progress).toBeNull();
    expect(shown.report?.files).toBe(10);
    expect(shown.error).toBeNull();

    // Shown again without a save or reset in between: no second compare, the report stays.
    await show();
    await show({ shown: false });
    await show();
    expect(changedFromOriginal).toHaveBeenCalledOnce();
    expect(shown.report?.files).toBe(10);

    // „Neu vergleichen“ compares on demand.
    vi.mocked(changedFromOriginal).mockResolvedValueOnce(report({ files: 11 }));
    await act(async () => shown.compare());
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    expect(shown.report?.files).toBe(11);

    // A failure is the German sentence; the report before stays.
    vi.mocked(changedFromOriginal).mockRejectedValueOnce({ code: "base_missing", message: "Die Basis-ROM fehlt.", asset: null });
    await act(async () => shown.compare());
    expect(shown.error).toBe("Die Basis-ROM fehlt.");
    expect(shown.busy).toBe(false);
    expect(shown.report?.files).toBe(11);

    // Another project starts from nothing and compares once for itself; an answer for the project before is dropped.
    const old = pending();
    await act(async () => void shown.compare());
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 77 }));
    await show({ project: "/b" });
    expect(shown.report?.files).toBe(77);
    await old.progress({ done: 1, total: 2 });
    await old.answer(report({ files: 5 }));
    expect(shown.report?.files).toBe(77);
    expect(shown.progress).toBeNull();
    expect(shown.busy).toBe(false);
    // No project: nothing.
    await show({ project: null });
    expect(shown.report).toBeNull();
  });

  it("compares_again_when_the_base_changes_and_drops_the_report_of_the_old_one", async () => {
    // Every render's report, so not even one frame shows the old base's rows under the new base.
    const seen: [string | null, number | undefined][] = [];
    function Recorder({ base }: { base: string | null }) {
      const api = useChanges({ project: "/a", shown: true, base, revision: 0 });
      seen.push([base, api.report?.files]);
      return null;
    }
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 1 }));
    await act(async () => root.render(<Recorder base={SHA_X} />));
    expect(last(seen)).toEqual([SHA_X, 1]);
    // Another base assigned to the same project (no save, no reset): compared against it, nothing of the old report shown meanwhile.
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 2 }));
    const run = pending();
    await act(async () => root.render(<Recorder base={SHA_Y} />));
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    expect(seen.filter(([base, files]) => base === SHA_Y && files === 1)).toEqual([]);
    await run.answer(report({ files: 2 }));
    expect(last(seen)).toEqual([SHA_Y, 2]);
    // The base went missing: nothing to compare against, nothing shown.
    await act(async () => root.render(<Recorder base={null} />));
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    expect(last(seen)).toEqual([null, undefined]);
    // Found again: compared again.
    await act(async () => root.render(<Recorder base={SHA_Y} />));
    expect(changedFromOriginal).toHaveBeenCalledTimes(3);
  });

  it("a_project_without_base_compares_once_a_base_is_assigned", async () => {
    // /a shows with its base; /b opens without one (its status unknown, then unavailable): no compare that could only fail.
    await show();
    expect(changedFromOriginal).toHaveBeenCalledTimes(1);
    const seen: (number | undefined)[] = [];
    function Recorder({ base }: { base: string | null }) {
      seen.push(useChanges({ project: "/b", shown: true, base, revision: 0 }).report?.files);
      return null;
    }
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 3 }));
    await act(async () => root.render(<Recorder base={null} />));
    expect(changedFromOriginal).toHaveBeenCalledTimes(1);
    // A base assigned (no save, no reset in between): compared at once.
    await act(async () => root.render(<Recorder base={SHA_Y} />));
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    expect(last(seen)).toBe(3);
  });

  it("a_new_project_shows_nothing_of_the_one_before", async () => {
    const seen: [string | null, number | undefined, boolean][] = [];
    function Recorder({ project }: { project: string | null }) {
      const api = useChanges({ project, shown: true, base: SHA_X, revision: 0 });
      seen.push([project, api.report?.files, api.busy]);
      return null;
    }
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 4 }));
    await act(async () => root.render(<Recorder project="/a" />));
    expect(last(seen)).toEqual(["/a", 4, false]);
    pending();
    await act(async () => root.render(<Recorder project="/b" />));
    // Not one render for /b carries /a's report.
    expect(seen.filter(([project, files]) => project === "/b" && files === 4)).toEqual([]);
    expect(last(seen)).toEqual(["/b", undefined, true]);
  });

  it("compares_again_after_save_and_reset", async () => {
    await show({ revision: 0 });
    expect(changedFromOriginal).toHaveBeenCalledTimes(1);
    // A save or a reset bumps the revision: compared again while the surface shows.
    await show({ revision: 1 });
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    // Not on screen: the bump waits until the surface shows again, then compares once.
    await show({ revision: 2, shown: false });
    await show({ revision: 3, shown: false });
    expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    await show({ revision: 3 });
    expect(changedFromOriginal).toHaveBeenCalledTimes(3);

    // A bump while a compare runs: no second run beside it, one more after it with the newer state.
    const run = pending();
    await show({ revision: 4 });
    expect(changedFromOriginal).toHaveBeenCalledTimes(4);
    await show({ revision: 5 });
    await show({ revision: 6 });
    expect(changedFromOriginal).toHaveBeenCalledTimes(4);
    vi.mocked(changedFromOriginal).mockResolvedValue(report({ files: 9 }));
    await run.answer(report({ files: 8 }));
    expect(changedFromOriginal).toHaveBeenCalledTimes(5);
    expect(shown.report?.files).toBe(9);
    expect(shown.busy).toBe(false);
  });

  it("compares_again_on_window_focus_as_the_git_status_reads_again", async () => {
    // A write past the app (a bridge call, another tool) moves no revision; the Git status reads again on focus, and so does this list.
    vi.useFakeTimers();
    try {
      await show();
      expect(changedFromOriginal).toHaveBeenCalledOnce();
      // Several quick focus changes make one compare, FOCUS_DELAY_MS after the last.
      await focus();
      await wait(FOCUS_DELAY_MS - 1);
      await focus();
      await wait(FOCUS_DELAY_MS - 1);
      expect(changedFromOriginal).toHaveBeenCalledOnce();
      await wait(1);
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);

      // A focus while a compare runs: none beside it, one more after it.
      const run = pending();
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(changedFromOriginal).toHaveBeenCalledTimes(3);
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(changedFromOriginal).toHaveBeenCalledTimes(3);
      await run.answer(report({ files: 4 }));
      expect(changedFromOriginal).toHaveBeenCalledTimes(4);

      // Not on screen: nothing is compared on focus, but once it shows again.
      await show({ shown: false });
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(changedFromOriginal).toHaveBeenCalledTimes(4);
      await show();
      expect(changedFromOriginal).toHaveBeenCalledTimes(5);
      // Shown again without a focus between: no compare of its own.
      await show({ shown: false });
      await show();
      expect(changedFromOriginal).toHaveBeenCalledTimes(5);

      // Without a project nobody listens.
      await show({ project: null });
      expect(focusListeners).toHaveLength(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it("a_focus_compare_is_quiet", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(changedFromOriginal).mockResolvedValueOnce(report({ files: 1 }));
      await show();
      expect(shown.report?.files).toBe(1);
      // While it runs: not busy, no progress, the list before stays.
      const run = pending();
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      expect(shown.busy).toBe(false);
      await run.progress({ done: 3, total: 10 });
      expect(shown.progress).toBeNull();
      expect(shown.report?.files).toBe(1);
      await run.answer(report({ files: 2 }));
      expect(shown.report?.files).toBe(2);
      expect(shown.busy).toBe(false);

      // A failure is one: it says so, and the list stays.
      const failing = pending();
      await focus();
      await wait(FOCUS_DELAY_MS);
      await failing.fail({ code: "error", message: "Die Basis-ROM ist weg.", asset: null });
      expect(shown.error).toBe("Die Basis-ROM ist weg.");
      expect(shown.report?.files).toBe(2);
      // A quiet run keeps that error while it runs and clears it only with a list that went through.
      const again = pending();
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(shown.error).toBe("Die Basis-ROM ist weg.");
      await again.answer(report({ files: 3 }));
      expect(shown.error).toBeNull();

      // „Neu vergleichen“ during a quiet run: the run after it shows that it compares.
      const quiet = pending();
      await focus();
      await wait(FOCUS_DELAY_MS);
      const loud = pending();
      await act(async () => void shown.compare());
      // Said at once, the quiet run's answer still taken.
      expect(shown.busy).toBe(true);
      await quiet.answer(report({ files: 4 }));
      expect(shown.report?.files).toBe(4);
      expect(shown.busy).toBe(true);
      expect(changedFromOriginal).toHaveBeenCalledTimes(6);
      await loud.answer(report({ files: 5 }));
      expect(shown.busy).toBe(false);
      expect(shown.report?.files).toBe(5);
    } finally {
      vi.useRealTimers();
    }
  });

  it("a_focus_compares_only_while_the_base_comparison_shows", async () => {
    vi.useFakeTimers();
    try {
      await show({ listed: false });
      // Shown with another comparison: the list is still compared when the surface opens (as before), not on focus.
      expect(changedFromOriginal).toHaveBeenCalledOnce();
      await focus();
      await wait(FOCUS_DELAY_MS);
      expect(changedFromOriginal).toHaveBeenCalledOnce();
      // Switched to „Gegen Basis“ after that focus: one quiet compare.
      const run = pending();
      await show({ listed: true });
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      expect(shown.busy).toBe(false);
      await run.answer(report({ files: 7 }));
      // Switched away and back without a focus: nothing more.
      await show({ listed: false });
      await show({ listed: true });
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
