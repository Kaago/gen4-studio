// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { originalStatus } from "../api";
import type { BaseStatus, OriginalStatus } from "../api";
import { found, mark, missing } from "../technik/baseTesting";
import { useOriginal } from "./useOriginal";
import type { OriginalApi, UseOriginalOptions } from "./useOriginal";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  originalStatus: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** The last of `items` (the last render recorded). */
const last = <T,>(items: readonly T[]): T | undefined => items[items.length - 1];

const ready: OriginalStatus = { state: "ready", label: "Platin DE (Testprojekt)", sha1: mark().sha1, counts: { map: 4 } };
const unavailable: OriginalStatus = { state: "unavailable", reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." };

describe("useOriginal", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: OriginalApi;

  function Probe(options: Partial<UseOriginalOptions>) {
    shown = useOriginal({ project: "/a", base: null, ...options });
    return null;
  }
  const show = (options: Partial<UseOriginalOptions> = {}) => act(async () => root.render(<Probe {...options} />));

  beforeEach(() => {
    vi.mocked(originalStatus).mockReset().mockResolvedValue(ready);
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("reads_the_status_once_per_project", async () => {
    await show();
    expect(shown.status).toEqual(ready);
    await show();
    expect(originalStatus).toHaveBeenCalledOnce();
    // The base status of Technik arriving (null → found) is no change of the base.
    await show({ base: found() });
    await show({ base: found() });
    expect(originalStatus).toHaveBeenCalledOnce();
    // Another project starts from nothing and reads its own.
    vi.mocked(originalStatus).mockResolvedValue(unavailable);
    await show({ project: "/b", base: missing() });
    expect(originalStatus).toHaveBeenCalledTimes(2);
    expect(shown.status).toEqual(unavailable);
    // No project: nothing is read, nothing is shown.
    await show({ project: null });
    expect(shown.status).toBeNull();
    expect(originalStatus).toHaveBeenCalledTimes(2);
  });

  it("reads_again_after_the_base_changed", async () => {
    vi.mocked(originalStatus).mockResolvedValue(unavailable);
    await show({ base: missing() });
    expect(shown.status).toEqual(unavailable);
    // „Basis-ROM suchen…“ found it (missing → found): read again.
    vi.mocked(originalStatus).mockResolvedValue(ready);
    await show({ base: found() });
    expect(originalStatus).toHaveBeenCalledTimes(2);
    expect(shown.status).toEqual(ready);
    // Another base assigned (other SHA-1): read again.
    const other: BaseStatus = found({ mark: mark({ sha1: "0".repeat(40) }) });
    await show({ base: other });
    expect(originalStatus).toHaveBeenCalledTimes(3);
    // The same status read again is no change.
    await show({ base: found({ mark: mark({ sha1: "0".repeat(40) }) }) });
    expect(originalStatus).toHaveBeenCalledTimes(3);
    // `refresh` reads at once (an agent assigned a base).
    await act(async () => shown.refresh());
    expect(originalStatus).toHaveBeenCalledTimes(4);
  });

  it("reads_again_when_the_first_base_status_disagrees_with_the_status_read_before", async () => {
    // Read without a base; Technik then finds one (the ROM appeared since): the status is read again.
    vi.mocked(originalStatus).mockResolvedValue(unavailable);
    await show();
    expect(shown.status).toEqual(unavailable);
    vi.mocked(originalStatus).mockResolvedValue(ready);
    await show({ base: found() });
    expect(originalStatus).toHaveBeenCalledTimes(2);
    expect(shown.status).toEqual(ready);
    // Only on the first arrival: the same status again reads nothing.
    await show({ base: found() });
    expect(originalStatus).toHaveBeenCalledTimes(2);
    // The other way: read with the base, which is gone when Technik looks.
    await show({ project: "/b" });
    expect(shown.status).toEqual(ready);
    vi.mocked(originalStatus).mockResolvedValue(unavailable);
    await show({ project: "/b", base: missing() });
    expect(originalStatus).toHaveBeenCalledTimes(4);
    expect(shown.status).toEqual(unavailable);
    // A first base status that agrees reads nothing.
    vi.mocked(originalStatus).mockResolvedValue(ready);
    await show({ project: "/c" });
    await show({ project: "/c", base: found() });
    expect(originalStatus).toHaveBeenCalledTimes(5);
  });

  it("drops_an_answer_for_the_project_before", async () => {
    let answer: (status: OriginalStatus) => void = () => {};
    vi.mocked(originalStatus).mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    await show();
    vi.mocked(originalStatus).mockResolvedValue(unavailable);
    await show({ project: "/b" });
    await act(async () => answer(ready));
    expect(shown.status).toEqual(unavailable);
  });

  it("never_shows_the_status_of_the_project_before", async () => {
    // Every render of the hook, with the project it rendered for: the status of /a must not reach a render for /b (a consumer would act on
    // it, e.g. Technik › Änderungen comparing /b against /a's base).
    const seen: [string | null, OriginalStatus | null][] = [];
    function Recorder({ project }: { project: string | null }) {
      seen.push([project, useOriginal({ project, base: null }).status]);
      return null;
    }
    await act(async () => root.render(<Recorder project="/a" />));
    expect(last(seen)).toEqual(["/a", ready]);
    vi.mocked(originalStatus).mockReturnValue(new Promise(() => {}));
    await act(async () => root.render(<Recorder project="/b" />));
    expect(seen.filter(([project, status]) => project === "/b" && status !== null)).toEqual([]);
    await act(async () => root.render(<Recorder project={null} />));
    expect(seen.filter(([project, status]) => project === null && status !== null)).toEqual([]);
  });

  it("a_failed_read_counts_as_unavailable_with_the_reason", async () => {
    vi.mocked(originalStatus).mockRejectedValue({ code: "error", message: "Kein Projekt geöffnet.", asset: null });
    await show();
    expect(shown.status).toEqual({ state: "unavailable", reason: "Kein Projekt geöffnet." });
  });
});
