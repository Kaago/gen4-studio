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
    errors.length = 0;
    vi.mocked(listMatrices).mockReset().mockResolvedValue(emptyMatrices);
    vi.mocked(listWarps).mockReset().mockResolvedValue(emptyWarps);
  });
  afterEach(() => {
    act(() => root.unmount());
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
    expect((errors[0] as Error).message).toBe("1 Raster nicht lesbar: 0007: bad header");
  });

  it("refresh re-reads both lists", async () => {
    await render();
    await act(async () => latest?.refresh());
    expect(listMatrices).toHaveBeenCalledTimes(2);
    expect(listWarps).toHaveBeenCalledTimes(2);
  });
});
