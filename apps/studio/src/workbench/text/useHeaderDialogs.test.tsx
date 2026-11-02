// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogsOfHeader } from "../../api";
import type { HeaderDialogs } from "../../api";
import { figure, headerDialogs } from "./textTesting";
import { HeaderDialogsCache, HeaderDialogsCacheContext, useHeaderDialogs } from "./useHeaderDialogs";
import type { FoundHeaderDialogs } from "./useHeaderDialogs";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), dialogsOfHeader: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("useHeaderDialogs", () => {
  let container: HTMLElement;
  let root: Root;
  let cache: HeaderDialogsCache;
  const seen: Record<string, FoundHeaderDialogs> = {};
  function Probe({ name, header, revision }: { name: string; header: number | null; revision: number }) {
    seen[name] = useHeaderDialogs(header, revision);
    return null;
  }
  const show = (probes: { name: string; header: number | null; revision: number }[]) =>
    act(() =>
      root.render(
        <HeaderDialogsCacheContext.Provider value={cache}>
          {probes.map((p) => (
            <Probe key={p.name} {...p} />
          ))}
        </HeaderDialogsCacheContext.Provider>,
      ),
    );

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    cache = new HeaderDialogsCache();
    for (const key of Object.keys(seen)) delete seen[key];
    vi.mocked(dialogsOfHeader).mockReset().mockImplementation(async (header) => headerDialogs(header, [figure("object", 0)]));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("reads nothing without a header", async () => {
    show([{ name: "a", header: null, revision: 0 }]);
    await flush();
    expect(dialogsOfHeader).not.toHaveBeenCalled();
    expect(seen.a).toEqual({ dialogs: null, error: null, loading: false });
  });

  it("reads a header once per revision, for every user of the cache", async () => {
    show([
      { name: "a", header: 12, revision: 0 },
      { name: "b", header: 12, revision: 0 },
    ]);
    expect(seen.a?.loading).toBe(true);
    await flush();
    expect(dialogsOfHeader).toHaveBeenCalledTimes(1);
    expect(seen.a?.dialogs?.header).toBe(12);
    expect(seen.b?.dialogs).toBe(seen.a?.dialogs);
    expect(seen.a?.loading).toBe(false);
    // Unmounted and shown again (the list folded and opened): the cache answers, nothing is read.
    show([]);
    show([{ name: "c", header: 12, revision: 0 }]);
    expect(seen.c?.dialogs?.header).toBe(12);
    expect(seen.c?.loading).toBe(false);
    await flush();
    expect(dialogsOfHeader).toHaveBeenCalledTimes(1);
  });

  it("reads again when the revision changes and keeps the last answer meanwhile", async () => {
    show([{ name: "a", header: 12, revision: 0 }]);
    await flush();
    const first = seen.a?.dialogs;
    let answer: (d: HeaderDialogs) => void = () => {};
    vi.mocked(dialogsOfHeader).mockImplementation(() => new Promise<HeaderDialogs>((resolve) => (answer = resolve)));
    show([{ name: "a", header: 12, revision: 1 }]);
    expect(dialogsOfHeader).toHaveBeenCalledTimes(2);
    expect(seen.a?.dialogs).toBe(first);
    expect(seen.a?.loading).toBe(true);
    await act(async () => answer(headerDialogs(12, [])));
    expect(seen.a?.dialogs?.figures).toEqual([]);
    expect(seen.a?.loading).toBe(false);
  });

  it("drops an answer a newer revision overtook", async () => {
    const answers: ((d: HeaderDialogs) => void)[] = [];
    vi.mocked(dialogsOfHeader).mockImplementation(() => new Promise<HeaderDialogs>((resolve) => answers.push(resolve)));
    show([{ name: "a", header: 12, revision: 0 }]);
    show([{ name: "a", header: 12, revision: 1 }]);
    await act(async () => answers[1]!(headerDialogs(12, [figure("bg", 0)])));
    await act(async () => answers[0]!(headerDialogs(12, [figure("object", 5)])));
    expect(seen.a?.dialogs?.figures.map((f) => f.kind)).toEqual(["bg"]);
  });

  it("says why a header could not be read", async () => {
    vi.mocked(dialogsOfHeader).mockRejectedValue({ code: "error", message: "Eventdatei 9 fehlt", asset: null });
    show([{ name: "a", header: 12, revision: 0 }]);
    await flush();
    expect(seen.a?.dialogs).toBeNull();
    expect(seen.a?.error).toContain("Eventdatei 9 fehlt");
    expect(seen.a?.loading).toBe(false);
  });

  it("a cleared cache reads again and drops the answers of before", async () => {
    let answer: (d: HeaderDialogs) => void = () => {};
    vi.mocked(dialogsOfHeader).mockImplementationOnce(() => new Promise<HeaderDialogs>((resolve) => (answer = resolve)));
    show([{ name: "a", header: 12, revision: 0 }]);
    cache.clear();
    await act(async () => answer(headerDialogs(12, [figure("object", 9)])));
    expect(cache.get(12)).toBeUndefined();
    const read = await act(() => cache.load(12, 0));
    expect(read?.figures.map((f) => f.index)).toEqual([0]);
  });
});
