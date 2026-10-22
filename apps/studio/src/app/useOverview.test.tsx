// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listMatrices, listWarps } from "../api";
import type { MatrixListing, WarpListing } from "../api";
import { createEditBus } from "../bus/editBus";
import type { EditBus } from "../bus/editBus";
import { useOverview } from "./useOverview";
import type { Overview } from "./useOverview";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  listMatrices: vi.fn(),
  listWarps: vi.fn(),
}));

const emptyMatrices: MatrixListing = { matrices: [], unreadable: [] };
const emptyWarps: WarpListing = { links: [], unreadable: [] };

/** A promise the test settles by hand. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe("useOverview", () => {
  let container: HTMLElement;
  let root: Root;
  let bus: EditBus;
  let latest: Overview | null;
  const errors: unknown[] = [];
  const onError = (e: unknown) => errors.push(e);
  const counts = {};

  function Host({ projectRoot, hostCounts }: { projectRoot: string | null; hostCounts: object }) {
    latest = useOverview({ bus, projectRoot, counts: hostCounts, onError });
    return null;
  }
  const render = (projectRoot: string | null = "/p", hostCounts: object = counts) =>
    act(async () => {
      root.render(<Host projectRoot={projectRoot} hostCounts={hostCounts} />);
    });
  const flush = () => act(async () => {});

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    bus = createEditBus();
    latest = null;
    unmounted = false;
    errors.length = 0;
    vi.mocked(listMatrices).mockReset().mockResolvedValue(emptyMatrices);
    vi.mocked(listWarps).mockReset().mockResolvedValue(emptyWarps);
  });
  let unmounted = false;
  const unmount = () => {
    unmounted = true;
    act(() => root.unmount());
  };
  afterEach(() => {
    if (!unmounted) unmount();
    container.remove();
  });

  it("reads both lists once on mount", async () => {
    await render();
    expect(listMatrices).toHaveBeenCalledTimes(1);
    expect(listWarps).toHaveBeenCalledTimes(1);
    expect(latest?.matrices).toEqual([]);
  });

  it("does not read without a project", async () => {
    await render(null);
    expect(listMatrices).not.toHaveBeenCalled();
    expect(listWarps).not.toHaveBeenCalled();
  });

  it("re-reads only the matrices on matrixData", async () => {
    await render();
    await act(async () => bus.emit({ topic: "matrixData" }));
    expect(listMatrices).toHaveBeenCalledTimes(2);
    expect(listWarps).toHaveBeenCalledTimes(1);
  });

  it("re-reads only the warps on eventsRefetch", async () => {
    await render();
    await act(async () => bus.emit({ topic: "eventsRefetch", file: 3 }));
    expect(listMatrices).toHaveBeenCalledTimes(1);
    expect(listWarps).toHaveBeenCalledTimes(2);
  });

  it("re-reads both lists when the counts change", async () => {
    await render("/p", {});
    await render("/p", { Matrix: 3 });
    expect(listMatrices).toHaveBeenCalledTimes(2);
    expect(listWarps).toHaveBeenCalledTimes(2);
  });

  it("never stacks warp reads: two eventData events during a read cause exactly one more", async () => {
    const first = deferred<WarpListing>();
    vi.mocked(listWarps).mockReturnValueOnce(first.promise);
    await render();
    expect(listWarps).toHaveBeenCalledTimes(1);

    await act(async () => {
      bus.emit({ topic: "eventData" });
      bus.emit({ topic: "eventData" });
    });
    expect(listWarps).toHaveBeenCalledTimes(1);

    const link = { event_file: 1, index: 0, x: 0, z: 0, dest_header_id: 2, dest_warp_id: 0 };
    vi.mocked(listWarps).mockResolvedValueOnce({ links: [link], unreadable: [] });
    await act(async () => first.resolve(emptyWarps));
    await flush();
    expect(listWarps).toHaveBeenCalledTimes(2);
    expect(latest?.links).toEqual([link]);
  });

  it("is loaded once the first matrix listing of the project arrived, and not again until the next project's", async () => {
    const first = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(first.promise);
    await render("/a");
    expect(latest?.loaded).toBe(false);
    await act(async () => first.resolve(emptyMatrices));
    expect(latest?.loaded).toBe(true);
    // A later read does not unload it ...
    const again = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(again.promise);
    await act(async () => bus.emit({ topic: "matrixData" }));
    expect(latest?.loaded).toBe(true);
    await act(async () => again.resolve(emptyMatrices));
    // ... another project does, until its own listing arrives.
    const next = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(next.promise);
    await render("/b");
    expect(latest?.loaded).toBe(false);
    await act(async () => next.resolve(emptyMatrices));
    expect(latest?.loaded).toBe(true);
  });

  it("is not loaded when the matrix listing failed, and says so: failed until a later listing succeeds or the project changes", async () => {
    vi.mocked(listMatrices).mockRejectedValueOnce(new Error("boom"));
    await render("/a");
    expect(latest?.loaded).toBe(false);
    expect(latest?.failed).toBe(true);
    expect(errors).toHaveLength(1);
    await act(async () => bus.emit({ topic: "matrixData" }));
    expect(latest?.loaded).toBe(true);
    expect(latest?.failed).toBe(false);
    vi.mocked(listMatrices).mockRejectedValueOnce(new Error("again"));
    await render("/b");
    expect(latest?.loaded).toBe(false);
    expect(latest?.failed).toBe(true);
    vi.mocked(listMatrices).mockReturnValueOnce(new Promise(() => {}));
    await render("/c");
    expect(latest?.failed).toBe(false);
  });

  it("does not fail while loading, nor from a failure of an older project", async () => {
    const old = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(old.promise);
    await render("/a");
    expect(latest?.failed).toBe(false);
    const next = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(next.promise);
    await render("/b");
    await act(async () => old.resolve(emptyMatrices));
    expect(latest?.failed).toBe(false);
  });

  it("is not loaded by the result of an older project", async () => {
    const old = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(old.promise);
    await render("/a");
    const next = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(next.promise);
    await render("/b");
    await act(async () => old.resolve(emptyMatrices));
    expect(latest?.loaded).toBe(false);
  });

  it("drops the result of an older project", async () => {
    const old = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(old.promise);
    await render("/a");
    vi.mocked(listMatrices).mockResolvedValueOnce({ matrices: [{ index: 1, name: "b", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 }], unreadable: [] });
    await render("/b");
    await act(async () => old.resolve({ matrices: [{ index: 9, name: "stale", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 }], unreadable: [] }));
    await flush();
    expect(latest?.matrices.map((m) => m.index)).toEqual([1]);
  });

  it("reports unreadable members once per project", async () => {
    vi.mocked(listMatrices).mockResolvedValue({ matrices: [], unreadable: [[7, "bad header"]] });
    await render();
    await act(async () => bus.emit({ topic: "matrixData" }));
    expect(errors).toHaveLength(1);
    expect((errors[0] as Error).message).toBe("Raster: 1 nicht lesbar (0007: bad header)");
  });

  it("refresh re-reads both lists", async () => {
    await render();
    await act(async () => latest?.refresh());
    expect(listMatrices).toHaveBeenCalledTimes(2);
    expect(listWarps).toHaveBeenCalledTimes(2);
  });

  it("reports unreadable members again after a project change", async () => {
    vi.mocked(listMatrices).mockResolvedValue({ matrices: [], unreadable: [[7, "bad"]] });
    await render("/a");
    await render("/b");
    expect(errors).toHaveLength(2);
  });

  it("reports unreadable event files with the glossary label", async () => {
    vi.mocked(listWarps).mockResolvedValue({ links: [], unreadable: [[3, "bad"]] });
    await render();
    expect((errors[0] as Error).message).toBe("Eventdatei: 1 nicht lesbar (0003: bad)");
  });

  it.each([
    ["listMatrices", () => vi.mocked(listMatrices), "matrixData", () => ({ topic: "matrixData" }) as const],
    ["listWarps", () => vi.mocked(listWarps), "eventData", () => ({ topic: "eventData" }) as const],
  ])("a rejected %s reaches onError and a later event still reads", async (_name, mock, _topic, event) => {
    const failure = new Error("boom");
    mock().mockRejectedValueOnce(failure);
    await render();
    expect(errors).toEqual([failure]);
    const calls = mock().mock.calls.length;
    await act(async () => bus.emit(event()));
    expect(mock().mock.calls.length).toBe(calls + 1);
  });

  it("refresh resolves only after the queued catch-up read", async () => {
    const first = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(first.promise);
    await render();
    let resolved = false;
    let refreshing!: Promise<void>;
    await act(async () => {
      refreshing = latest!.refresh().then(() => {
        resolved = true;
      });
    });
    expect(listMatrices).toHaveBeenCalledTimes(1);
    const second = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(second.promise);
    await act(async () => first.resolve(emptyMatrices));
    expect(listMatrices).toHaveBeenCalledTimes(2);
    expect(resolved).toBe(false);
    await act(async () => second.resolve(emptyMatrices));
    await refreshing;
    expect(resolved).toBe(true);
  });

  it("does nothing after the unmount: no queued read, no error", async () => {
    const first = deferred<MatrixListing>();
    vi.mocked(listMatrices).mockReturnValueOnce(first.promise);
    await render();
    await act(async () => bus.emit({ topic: "matrixData" }));
    expect(listMatrices).toHaveBeenCalledTimes(1);
    unmount();
    vi.mocked(listMatrices).mockClear();
    await act(async () => first.resolve({ matrices: [], unreadable: [[7, "bad"]] }));
    await flush();
    expect(listMatrices).not.toHaveBeenCalled();
    expect(errors).toEqual([]);
  });
});
