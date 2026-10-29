// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { gitStatus } from "../api";
import type { GitStatus } from "../api";
import { subscribeWindowFocus } from "../platform/windowFocus";
import { FOCUS_DELAY_MS } from "../app/useExternalSync";
import { gitStatusOf } from "./gitTesting";
import { useGitStatus } from "./useGitStatus";
import type { GitStatusApi } from "./useGitStatus";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  gitStatus: vi.fn(),
}));
vi.mock("../platform/windowFocus", () => ({ subscribeWindowFocus: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Opts = Parameters<typeof useGitStatus>[0];

/** A read that answers when the test says so. */
function pending() {
  let answer: (s: GitStatus) => void = () => {};
  let fail: (e: unknown) => void = () => {};
  vi.mocked(gitStatus).mockImplementationOnce(
    () =>
      new Promise<GitStatus>((resolve, reject) => {
        answer = resolve;
        fail = reject;
      }),
  );
  return {
    answer: (s: GitStatus = gitStatusOf()) => act(async () => answer(s)),
    fail: (e: unknown) => act(async () => fail(e)),
  };
}

describe("useGitStatus", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: GitStatusApi;
  let focusListeners: (() => void)[];

  function Probe(options: Partial<Opts>) {
    shown = useGitStatus({ project: "/p", revision: 0, ...options });
    return null;
  }
  const show = (options: Partial<Opts> = {}) => act(async () => root.render(<Probe {...options} />));
  const focus = () => act(async () => focusListeners.forEach((l) => l()));
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));

  beforeEach(() => {
    vi.useFakeTimers();
    focusListeners = [];
    vi.mocked(subscribeWindowFocus).mockReset().mockImplementation((listener) => {
      focusListeners.push(listener);
      return () => void (focusListeners = focusListeners.filter((l) => l !== listener));
    });
    vi.mocked(gitStatus).mockReset().mockResolvedValue(gitStatusOf());
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("reads_on_open_and_after_a_revision", async () => {
    // No project: nothing is read, nothing is listened to.
    await show({ project: null });
    expect(gitStatus).not.toHaveBeenCalled();
    expect(shown.status).toBeNull();
    expect(focusListeners).toHaveLength(0);

    vi.mocked(gitStatus).mockResolvedValueOnce(gitStatusOf({ changes: 2 }));
    await show();
    expect(gitStatus).toHaveBeenCalledTimes(1);
    expect(shown.status?.changes).toBe(2);
    expect(shown.error).toBeNull();

    // A save, a build, an agent's write: the revision moves, the status is read again.
    vi.mocked(gitStatus).mockResolvedValueOnce(gitStatusOf({ changes: 3 }));
    await show({ revision: 1 });
    expect(gitStatus).toHaveBeenCalledTimes(2);
    expect(shown.status?.changes).toBe(3);

    // „Git einrichten“ asks itself, and waits for the answer.
    vi.mocked(gitStatus).mockResolvedValueOnce(gitStatusOf({ changes: 0 }));
    await act(async () => shown.refresh());
    expect(gitStatus).toHaveBeenCalledTimes(3);
    expect(shown.status?.changes).toBe(0);

    // A failed read keeps its reason; the next one that goes through clears it.
    vi.mocked(gitStatus).mockRejectedValueOnce({ code: "error", message: "Das Repository ist beschädigt.", asset: null });
    await show({ revision: 2 });
    expect(shown.error).toBe("Das Repository ist beschädigt.");
    await show({ revision: 3 });
    expect(shown.error).toBeNull();

    // Another project: what was read for the old one is gone until its own status answers.
    const read = pending();
    await show({ project: "/q", revision: 3 });
    expect(shown.status).toBeNull();
    await read.answer(gitStatusOf({ branch: "dev" }));
    expect(shown.status?.branch).toBe("dev");

    // The project closes: nothing shows.
    await show({ project: null, revision: 3 });
    expect(shown.status).toBeNull();
  });

  it("focus_reads_again_after_the_delay", async () => {
    await show();
    expect(gitStatus).toHaveBeenCalledTimes(1);
    // Several quick focus changes make one read, FOCUS_DELAY_MS after the last.
    await focus();
    await wait(FOCUS_DELAY_MS - 100);
    await focus();
    await wait(FOCUS_DELAY_MS - 1);
    expect(gitStatus).toHaveBeenCalledTimes(1);
    vi.mocked(gitStatus).mockResolvedValueOnce(gitStatusOf({ changes: 5 }));
    await wait(1);
    expect(gitStatus).toHaveBeenCalledTimes(2);
    expect(shown.status?.changes).toBe(5);

    // After the project closed a pending focus reads nothing.
    await focus();
    await show({ project: null });
    await wait(FOCUS_DELAY_MS);
    expect(gitStatus).toHaveBeenCalledTimes(2);
    expect(focusListeners).toHaveLength(0);
  });

  it("never_two_reads_at_once", async () => {
    const first = pending();
    await show();
    expect(gitStatus).toHaveBeenCalledTimes(1);
    // Three reasons while the first read runs: they wait, and share one read after it.
    const second = pending();
    await show({ revision: 1 });
    await show({ revision: 2 });
    let refreshed = false;
    void shown.refresh().then(() => (refreshed = true));
    expect(gitStatus).toHaveBeenCalledTimes(1);
    await first.answer(gitStatusOf({ changes: 1 }));
    expect(gitStatus).toHaveBeenCalledTimes(2);
    expect(shown.status?.changes).toBe(1);
    expect(refreshed).toBe(false);
    await second.answer(gitStatusOf({ changes: 4 }));
    expect(refreshed).toBe(true);
    expect(shown.status?.changes).toBe(4);
    // Nothing more came meanwhile: no third read.
    await wait(FOCUS_DELAY_MS * 2);
    expect(gitStatus).toHaveBeenCalledTimes(2);

    // A read of the old project that answers after another opened is dropped; the new one's waits for it, then reads.
    const old = pending();
    await show({ revision: 3 });
    const next = pending();
    await show({ project: "/q", revision: 3 });
    expect(gitStatus).toHaveBeenCalledTimes(3);
    await old.answer(gitStatusOf({ branch: "alt" }));
    expect(shown.status).toBeNull();
    expect(gitStatus).toHaveBeenCalledTimes(4);
    await next.answer(gitStatusOf({ branch: "neu" }));
    expect(shown.status?.branch).toBe("neu");
  });

  it("no_read_runs_after_unmount", async () => {
    const first = pending();
    await show();
    // A reason while the first read runs queues one more; then the app goes away.
    await show({ revision: 1 });
    act(() => root.unmount());
    await first.answer(gitStatusOf({ changes: 9 }));
    await wait(FOCUS_DELAY_MS);
    expect(gitStatus).toHaveBeenCalledTimes(1);
    // afterEach unmounts again: a fresh root to unmount.
    root = createRoot(container);
  });
});
