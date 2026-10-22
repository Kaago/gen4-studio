// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createEditBus } from "../bus/editBus";
import type { EditBus } from "../bus/editBus";
import { ThumbnailCache } from "../world/thumbnails";
import { useThumbnailReset } from "./useThumbnailReset";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useThumbnailReset", () => {
  let container: HTMLElement;
  let root: Root;
  let bus: EditBus;
  let cache: ThumbnailCache;
  let clear: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
    bus = createEditBus();
    cache = new ThumbnailCache({ glb: async () => new ArrayBuffer(1) }, async () => "data:x");
    clear = vi.spyOn(cache, "clear");
  });
  afterEach(() => act(() => root.unmount()));

  function Probe({ projectRoot }: { projectRoot: string | null }) {
    useThumbnailReset(bus, projectRoot, cache);
    return null;
  }

  it("clears_when_another_project_is_opened_but_not_on_mount_or_a_rerender", () => {
    act(() => root.render(<Probe projectRoot="/a" />));
    expect(clear).not.toHaveBeenCalled();
    act(() => root.render(<Probe projectRoot="/a" />));
    expect(clear).not.toHaveBeenCalled();
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(clear).toHaveBeenCalledTimes(1);
  });

  it("clears_on_models_reload_only", () => {
    act(() => root.render(<Probe projectRoot="/a" />));
    bus.emit({ topic: "matrixData" });
    expect(clear).not.toHaveBeenCalled();
    bus.emit({ topic: "modelsReload" });
    expect(clear).toHaveBeenCalledTimes(1);
  });

  it("forgets_pictures_of_the_old_project", async () => {
    act(() => root.render(<Probe projectRoot="/a" />));
    cache.request(1, 1);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(cache.get(1, 1)).toBe("data:x");
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(cache.get(1, 1)).toBeUndefined();
  });
});
