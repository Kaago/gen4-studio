// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { storyList } from "../api";
import type { ProjectInfo, StoryListDto } from "../api";
import { storiesOf, storyRow } from "./storiesTesting";
import { useStories } from "./useStories";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), storyList: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (root = "/p"): ProjectInfo => ({ root, game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useStories> | null = null;
function Probe({ project, revision }: { project: ProjectInfo | null; revision: number }) {
  seen = useStories({ project, revision });
  return null;
}

describe("useStories", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(storyList).mockReset().mockResolvedValue(storiesOf([storyRow(0), storyRow(1)]));
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
    expect(storyList).not.toHaveBeenCalled();
    expect(seen?.list).toBeNull();
    expect(seen?.revision).toBeNull();
    const p = project();
    await show(p, 0);
    await flush();
    expect(storyList).toHaveBeenCalledOnce();
    expect(seen?.list?.stories).toHaveLength(2);
    // The same revision reads nothing; a new one (a story created, its notes changed, an undo) reads again, the older list stays meanwhile.
    await show(p, 0);
    expect(storyList).toHaveBeenCalledOnce();
    const next = deferred<StoryListDto>();
    vi.mocked(storyList).mockImplementationOnce(() => next.promise);
    await show(p, 1);
    expect(seen?.list?.stories).toHaveLength(2);
    // The older list says the revision it was read at, so a reader of a story it names waits for the new one (gate 3d.6, U2).
    expect(seen?.revision).toBe(0);
    next.resolve(storiesOf([storyRow(0)]));
    await flush();
    expect(seen?.list?.stories).toHaveLength(1);
    expect(seen?.revision).toBe(1);
    // `reload` reads again without a new revision.
    await act(async () => seen!.reload());
    await flush();
    expect(storyList).toHaveBeenCalledTimes(3);
  });

  it("drops_an_answer_a_newer_read_overtook_and_another_projects_list", async () => {
    const slow = deferred<StoryListDto>();
    vi.mocked(storyList).mockImplementationOnce(() => slow.promise);
    await show(project("/a"), 0);
    vi.mocked(storyList).mockResolvedValueOnce(storiesOf([storyRow(7)]));
    await show(project("/b"), 0);
    await flush();
    slow.resolve(storiesOf([storyRow(0), storyRow(1)]));
    await flush();
    expect(seen?.list?.stories.map((s) => s.number)).toEqual([7]);
    // A refusal is the error, with the kernel's sentence.
    vi.mocked(storyList).mockRejectedValueOnce({ code: "refused", message: "Kein Projekt offen.", asset: null });
    await show(project("/c"), 0);
    await flush();
    expect(seen?.list).toBeNull();
    expect(seen?.error).toBe("Kein Projekt offen.");
  });
});
