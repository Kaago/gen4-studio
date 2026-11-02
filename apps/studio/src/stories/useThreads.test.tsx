// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listThreads } from "../api";
import type { ProjectInfo, ThreadListDto } from "../api";
import { THREADS, threadList, threadOf } from "./storiesTesting";
import { useThreads } from "./useThreads";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listThreads: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (root = "/p"): ProjectInfo => ({ root, game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useThreads> | null = null;
function Probe({ project, revision }: { project: ProjectInfo | null; revision: number }) {
  seen = useThreads({ project, revision });
  return null;
}

describe("useThreads", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(listThreads).mockReset().mockResolvedValue(THREADS);
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (p: ProjectInfo | null, revision: number) => act(async () => root.render(<Probe project={p} revision={revision} />));

  it("reads_per_project_and_revision_and_keeps_the_older_list_meanwhile", async () => {
    // No project: nothing is read.
    await show(null, 0);
    expect(listThreads).not.toHaveBeenCalled();
    expect(seen?.list).toBeNull();
    const p = project();
    await show(p, 0);
    await flush();
    expect(listThreads).toHaveBeenCalledOnce();
    expect(seen?.list?.threads).toHaveLength(3);
    // The same revision reads nothing; a new one reads again and the older list stays meanwhile.
    await show(p, 0);
    expect(listThreads).toHaveBeenCalledOnce();
    const next = deferred<ThreadListDto>();
    vi.mocked(listThreads).mockImplementationOnce(() => next.promise);
    await show(p, 1);
    expect(seen?.list?.threads).toHaveLength(3);
    next.resolve(threadList([threadOf(0x4031)]));
    await flush();
    expect(seen?.list?.threads).toHaveLength(1);
    // `reload` reads again without a new revision.
    await act(async () => seen!.reload());
    await flush();
    expect(listThreads).toHaveBeenCalledTimes(3);
  });

  it("drops_an_answer_a_newer_read_overtook_and_another_projects_list", async () => {
    const slow = deferred<ThreadListDto>();
    vi.mocked(listThreads).mockImplementationOnce(() => slow.promise);
    await show(project("/a"), 0);
    // Another project: the first answer never shows, nor the old project's list while the new one is read.
    vi.mocked(listThreads).mockResolvedValueOnce(threadList([threadOf(0x4099)]));
    await show(project("/b"), 0);
    await flush();
    slow.resolve(THREADS);
    await flush();
    expect(seen?.list?.threads.map((t) => t.var)).toEqual([0x4099]);
    // A refusal is the error, with the kernel's sentence.
    vi.mocked(listThreads).mockRejectedValueOnce({ code: "refused", message: "Kein Projekt offen.", asset: null });
    await show(project("/c"), 0);
    await flush();
    expect(seen?.list).toBeNull();
    expect(seen?.error).toBe("Kein Projekt offen.");
  });
});
