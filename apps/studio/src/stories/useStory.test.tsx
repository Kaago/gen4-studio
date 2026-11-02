// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { storyGet } from "../api";
import type { ProjectInfo, StoryDto } from "../api";
import { storyDto } from "./storiesTesting";
import { useStory } from "./useStory";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), storyGet: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useStory> | null = null;
function Probe({ id, project, revision }: { id: string | null; project: ProjectInfo | null; revision: number }) {
  seen = useStory(id, { project, revision });
  return null;
}

describe("useStory", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(storyGet).mockReset().mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (id: string | null, p: ProjectInfo | null, revision: number) => act(async () => root.render(<Probe id={id} project={p} revision={revision} />));

  it("reads_the_chosen_story_per_revision", async () => {
    const p = project();
    // Nothing chosen or no project: nothing is read.
    await show(null, p, 0);
    await show("STORY_0000", null, 0);
    expect(storyGet).not.toHaveBeenCalled();
    expect(seen).toEqual({ story: null, error: null, busy: false });
    const first = deferred<StoryDto>();
    vi.mocked(storyGet).mockImplementationOnce(() => first.promise);
    await show("STORY_0000", p, 0);
    expect(seen?.busy).toBe(true);
    first.resolve(storyDto(0));
    await flush();
    expect(storyGet).toHaveBeenCalledWith("STORY_0000");
    expect(seen?.story?.story.id).toBe("STORY_0000");
    expect(seen?.busy).toBe(false);
    // A new revision reads again; the story stays meanwhile (the notes field keeps its text while the write is read back).
    const next = deferred<StoryDto>();
    vi.mocked(storyGet).mockImplementationOnce(() => next.promise);
    await show("STORY_0000", p, 1);
    expect(seen?.story?.story.notes).toBe("Die Rivalin begleitet den Weg.");
    next.resolve(storyDto(0, {}, { notes: "Neu." }));
    await flush();
    expect(seen?.story?.story.notes).toBe("Neu.");
  });

  it("never_shows_another_storys_answer", async () => {
    const p = project();
    const slow = deferred<StoryDto>();
    vi.mocked(storyGet).mockImplementationOnce(() => slow.promise);
    await show("STORY_0000", p, 0);
    const other = deferred<StoryDto>();
    vi.mocked(storyGet).mockImplementationOnce(() => other.promise);
    await show("STORY_0001", p, 0);
    // Another story: nothing older shows while it is read, and the answer for the one before never does.
    slow.resolve(storyDto(0));
    await flush();
    expect(seen?.story).toBeNull();
    expect(seen?.busy).toBe(true);
    other.resolve(storyDto(1, {}, { id: "STORY_0001" }));
    await flush();
    expect(seen?.story?.story.id).toBe("STORY_0001");
    // A refusal is the error, with the kernel's sentence.
    vi.mocked(storyGet).mockRejectedValueOnce({ code: "unknown_id", message: "Unbekannte ID: STORY_0009.", asset: null });
    await show("STORY_0009", p, 0);
    await flush();
    expect(seen?.story).toBeNull();
    expect(seen?.error).toBe("Unbekannte ID: STORY_0009.");
  });
});
