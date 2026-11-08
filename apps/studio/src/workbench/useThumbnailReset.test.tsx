// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createEditBus } from "../bus/editBus";
import type { EditBus } from "../bus/editBus";
import * as paletteImages from "../world/paint/paletteImages";
import * as tileNames from "../world/scene/tileNames";
import * as sprites from "../world/sprites";
import { ThumbnailCache } from "../world/thumbnails";
import { useThumbnailReset, useThumbnailToolReport } from "./useThumbnailReset";

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

  it("forgets_the_object_sprites_with_another_project_but_not_on_models_reload", () => {
    const reset = vi.spyOn(sprites, "resetSprites");
    act(() => root.render(<Probe projectRoot="/a" />));
    act(() => root.render(<Probe projectRoot="/a" />));
    bus.emit({ topic: "modelsReload" });
    expect(reset).not.toHaveBeenCalled();
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(reset).toHaveBeenCalledTimes(1);
    reset.mockRestore();
  });

  it("forgets_the_tile_names_with_another_project_but_not_on_models_reload", () => {
    const reset = vi.spyOn(tileNames, "resetTileNames");
    act(() => root.render(<Probe projectRoot="/a" />));
    act(() => root.render(<Probe projectRoot="/a" />));
    bus.emit({ topic: "modelsReload" });
    expect(reset).not.toHaveBeenCalled();
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(reset).toHaveBeenCalledTimes(1);
    reset.mockRestore();
  });

  it("forgets_the_palette_pictures_with_another_project_and_on_models_reload", () => {
    const reset = vi.spyOn(paletteImages, "resetTileImages");
    act(() => root.render(<Probe projectRoot="/a" />));
    act(() => root.render(<Probe projectRoot="/a" />));
    expect(reset).not.toHaveBeenCalled();
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(reset).toHaveBeenCalledTimes(1);
    bus.emit({ topic: "modelsReload" });
    expect(reset).toHaveBeenCalledTimes(2);
    reset.mockRestore();
  });

  it("forgets_pictures_of_the_old_project", async () => {
    act(() => root.render(<Probe projectRoot="/a" />));
    cache.request(1, 1);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(cache.get(1, 1)).toBe("data:x");
    act(() => root.render(<Probe projectRoot="/b" />));
    expect(cache.get(1, 1)).toBeUndefined();
  });

  it("reports_a_missing_tool_through_on_error_once_and_stops_after_unmount", async () => {
    const missing = { code: "tool_missing", message: "apicula missing", asset: null };
    const tool = new ThumbnailCache({ glb: () => Promise.reject(missing) }, async () => "data:x");
    const onError = vi.fn();
    function Reporter() {
      useThumbnailToolReport(tool, onError);
      return null;
    }
    act(() => root.render(<Reporter />));
    tool.request(1, 1);
    tool.request(2, 1);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith(missing);
    act(() => root.render(<></>));
    tool.clear();
    tool.request(3, 1);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(onError).toHaveBeenCalledTimes(1);
  });
});
