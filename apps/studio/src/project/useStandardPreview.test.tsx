// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { projectPreviewGet, projectPreviewSet } from "../api";
import type { PreviewDto } from "../api";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { previewWritten } from "./previewMessages";
import { useStandardPreview } from "./useStandardPreview";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), projectPreviewGet: vi.fn(), projectPreviewSet: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Zweiblattdorf in Platin: header 411, map 3, tileset 7. */
const TOWN = { header: 411, map: 3, areaDataId: 7 };
const stored = (source: PreviewDto["source"], png: string | null = source === null ? null : "GESPEICHERT", standardMap: PreviewDto["standardMap"] = TOWN): PreviewDto => ({ source, png, standardMap });

describe("useStandardPreview", () => {
  let container: HTMLElement;
  let root: Root;
  const render = vi.fn<(map: number, areaDataId: number) => Promise<string>>();
  const same = vi.fn<(a: string, b: string) => Promise<boolean>>();
  /** The idle callbacks asked for and not yet run. */
  let idle: (() => void)[] = [];

  /** A project as App has it once opened (a new object per opening). */
  const opened = (root = "/p/pt") => ({ root });

  function Probe({ project }: { project: { root: string } | null }) {
    useStandardPreview(project, render, same);
    return null;
  }
  const show = (project: { root: string } | null) => act(async () => root.render(<Probe project={project} />));
  /** The computer is calm: the idle callbacks run, and what they started settles. */
  const calm = () =>
    act(async () => {
      const due = idle;
      idle = [];
      for (const run of due) run();
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

  beforeEach(() => {
    idle = [];
    vi.stubGlobal("requestIdleCallback", (run: () => void) => idle.push(run));
    vi.stubGlobal("cancelIdleCallback", () => {});
    render.mockReset().mockResolvedValue("NEU");
    same.mockReset().mockImplementation(async (a, b) => a === b);
    vi.mocked(projectPreviewGet).mockReset().mockResolvedValue(stored("standard"));
    vi.mocked(projectPreviewSet).mockReset().mockImplementation(async ({ png }) => stored("standard", png ?? null));
    resetToasts();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("waits_for_a_calm_moment_then_writes_the_start_towns_map_picture_as_the_standard_slot", async () => {
    const heard: PreviewDto[] = [];
    const stop = previewWritten.subscribe((dto) => heard.push(dto));
    await show(opened());
    expect(projectPreviewGet).not.toHaveBeenCalled();
    await calm();
    stop();
    // Read and written by the project's folder: a job of one project never touches another one's slot.
    expect(projectPreviewGet).toHaveBeenCalledWith("/p/pt");
    expect(projectPreviewGet).not.toHaveBeenCalledWith();
    expect(render).toHaveBeenCalledWith(3, 7);
    expect(same).toHaveBeenCalledWith("NEU", "GESPEICHERT");
    expect(projectPreviewSet).toHaveBeenCalledWith({ path: "/p/pt", slot: "standard", png: "NEU" });
    // The open preview (Technik) hears of the new picture.
    expect(heard).toEqual([stored("standard", "NEU")]);
  });

  it("writes_nothing_when_the_picture_is_the_stored_one", async () => {
    render.mockResolvedValue("GESPEICHERT");
    await show(opened());
    await calm();
    expect(render).toHaveBeenCalledOnce();
    expect(projectPreviewSet).not.toHaveBeenCalled();
  });

  it("writes_without_comparing_when_the_standard_file_cannot_be_seen", async () => {
    // An own picture hides the standard file, and without any picture there is none: the picture is written (a write is cheap).
    for (const source of ["eigen", null] as const) {
      vi.mocked(projectPreviewGet).mockResolvedValue(stored(source));
      vi.mocked(projectPreviewSet).mockClear();
      await show(opened());
      await calm();
      expect(projectPreviewSet).toHaveBeenCalledWith({ path: "/p/pt", slot: "standard", png: "NEU" });
    }
    expect(same).not.toHaveBeenCalled();
  });

  it("writes_nothing_without_a_start_town", async () => {
    // No recipe (Diamant, Perl): no standard picture, no message.
    vi.mocked(projectPreviewGet).mockResolvedValue(stored(null, null, null));
    await show(opened());
    await calm();
    expect(render).not.toHaveBeenCalled();
    expect(projectPreviewSet).not.toHaveBeenCalled();
    expect(listToasts()).toEqual([]);
  });

  it("stays_silent_when_reading_drawing_or_writing_fails", async () => {
    // A rejection that escaped would fail this test run as an unhandled error. apicula missing: the drawing fails.
    render.mockRejectedValueOnce({ code: "tool_missing", message: "apicula fehlt", asset: null });
    await show(opened());
    await calm();
    expect(projectPreviewSet).not.toHaveBeenCalled();
    // The write is refused.
    vi.mocked(projectPreviewSet).mockRejectedValueOnce(new Error("disk full"));
    await show(opened());
    await calm();
    expect(projectPreviewSet).toHaveBeenCalledOnce();
    // The read fails.
    vi.mocked(projectPreviewGet).mockRejectedValueOnce(new Error("bridge down"));
    await show(opened());
    await calm();
    // The comparison throws.
    same.mockRejectedValueOnce(new Error("no canvas"));
    await show(opened());
    await calm();
    expect(same).toHaveBeenCalledTimes(2);
    expect(projectPreviewSet).toHaveBeenCalledOnce();
    expect(listToasts()).toEqual([]);
  });

  it("writes_nothing_when_another_project_was_opened_while_drawing", async () => {
    // The kernel switched to project B while A's town was drawn: asked by A's folder, it no longer names a start town (A is not open), so
    // the picture (drawn from B's models) is dropped; A's slot keeps its picture and B's is never touched.
    vi.mocked(projectPreviewGet).mockResolvedValueOnce(stored("standard")).mockResolvedValueOnce(stored("standard", "GESPEICHERT", null));
    await show(opened());
    await calm();
    expect(render).toHaveBeenCalledOnce();
    expect(projectPreviewGet).toHaveBeenCalledTimes(2);
    expect(vi.mocked(projectPreviewGet).mock.calls).toEqual([["/p/pt"], ["/p/pt"]]);
    expect(projectPreviewSet).not.toHaveBeenCalled();
    // The same when the open project names another start town by then.
    vi.mocked(projectPreviewGet).mockResolvedValueOnce(stored("standard")).mockResolvedValueOnce(stored("standard", "GESPEICHERT", { header: 1, map: 9, areaDataId: 2 }));
    await show(opened());
    await calm();
    expect(projectPreviewSet).not.toHaveBeenCalled();
  });

  it("runs_once_per_opened_project", async () => {
    const project = opened();
    await show(project);
    await calm();
    await show(project);
    await calm();
    expect(render).toHaveBeenCalledOnce();
    // Closed: nothing. Opened again (a new project object, also for the same folder): once more.
    await show(null);
    await calm();
    expect(render).toHaveBeenCalledOnce();
    await show(opened());
    await calm();
    expect(render).toHaveBeenCalledTimes(2);
  });

  it("drops_the_wait_when_the_project_goes_before_the_calm_moment", async () => {
    const cancel = vi.fn();
    vi.stubGlobal("cancelIdleCallback", cancel);
    await show(opened());
    await show(null);
    expect(cancel).toHaveBeenCalled();
    await calm();
    expect(projectPreviewGet).not.toHaveBeenCalled();
  });

  it("waits_two_seconds_where_there_is_no_idle_callback", async () => {
    vi.stubGlobal("requestIdleCallback", undefined);
    vi.useFakeTimers();
    try {
      await show(opened());
      await act(async () => void vi.advanceTimersByTime(1999));
      expect(projectPreviewGet).not.toHaveBeenCalled();
      await act(async () => void vi.advanceTimersByTime(1));
      expect(projectPreviewGet).toHaveBeenCalledWith("/p/pt");
    } finally {
      vi.useRealTimers();
    }
  });
});
