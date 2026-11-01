// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { threadGraph } from "../api";
import type { ProjectInfo, ThreadGraphDto } from "../api";
import { graphOf } from "./storiesTesting";
import { useThreadGraph } from "./useThreadGraph";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), threadGraph: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useThreadGraph> | null = null;
function Probe({ variable, story, project, revision }: { variable: number | null; story: string | null; project: ProjectInfo | null; revision: number }) {
  seen = useThreadGraph(variable, story, { project, revision });
  return null;
}

describe("useThreadGraph", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(threadGraph).mockReset().mockImplementation(async (variable) => graphOf(variable));
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (variable: number | null, story: string | null, p: ProjectInfo | null, revision: number) =>
    act(async () => root.render(<Probe variable={variable} story={story} project={p} revision={revision} />));

  it("reads_on_selection_with_its_story_and_after_a_revision", async () => {
    const p = project();
    await show(null, null, p, 0);
    await show(0x4031, null, null, 0);
    expect(threadGraph).not.toHaveBeenCalled();
    expect(seen).toEqual({ graph: null, error: null, busy: false });
    const first = deferred<ThreadGraphDto>();
    vi.mocked(threadGraph).mockImplementationOnce(() => first.promise);
    await show(0x4031, null, p, 0);
    expect(seen).toEqual({ graph: null, error: null, busy: true });
    first.resolve(graphOf(0x4031));
    await flush();
    expect(threadGraph).toHaveBeenLastCalledWith(0x4031, undefined);
    expect(seen?.graph?.var).toBe(0x4031);
    expect(seen?.busy).toBe(false);
    // With a story, its id goes along; a new revision reads again.
    await show(0x4031, "STORY_RIVALIN", p, 0);
    await flush();
    expect(threadGraph).toHaveBeenLastCalledWith(0x4031, "STORY_RIVALIN");
    await show(0x4031, "STORY_RIVALIN", p, 1);
    await flush();
    expect(threadGraph).toHaveBeenCalledTimes(3);
  });

  it("drops_an_older_answer", async () => {
    const p = project();
    const slow = deferred<ThreadGraphDto>();
    vi.mocked(threadGraph).mockImplementationOnce(() => slow.promise);
    await show(0x4031, null, p, 0);
    // Another thread is chosen before the first answer: that answer never shows, nor the old graph while the new one is read.
    const next = deferred<ThreadGraphDto>();
    vi.mocked(threadGraph).mockImplementationOnce(() => next.promise);
    await show(0x4040, null, p, 0);
    slow.resolve(graphOf(0x4031));
    await flush();
    expect(seen).toEqual({ graph: null, error: null, busy: true });
    next.resolve(graphOf(0x4040));
    await flush();
    expect(seen?.graph?.var).toBe(0x4040);
    // A refusal is the error, with the kernel's sentence.
    vi.mocked(threadGraph).mockRejectedValueOnce({ code: "unknown_id", message: "STORY_X gibt es nicht.", asset: null });
    await show(0x4040, "STORY_X", p, 0);
    await flush();
    expect(seen).toEqual({ graph: null, error: "STORY_X gibt es nicht.", busy: false });
  });
});
