// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { changedFromOriginal } from "../api";
import type { ChangedProgress, ChangedReport } from "../api";
import { report } from "./changesTesting";
import { useChanges } from "./useChanges";
import type { ChangesApi, UseChangesOptions } from "./useChanges";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  changedFromOriginal: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

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
    shown = useChanges({ project: "/a", shown: true, ready: true, revision: 0, ...options });
    return null;
  }
  const show = (options: Partial<UseChangesOptions> = {}) => act(async () => root.render(<Probe {...options} />));

  beforeEach(() => {
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
    await show({ ready: false });
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
});
