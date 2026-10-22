// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThumbnailCache } from "./thumbnails";
import { useThumbnail } from "./useThumbnail";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("useThumbnail", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function Probe({ cache, map, onRender }: { cache: ThumbnailCache; map: number | null; onRender: () => void }) {
    onRender();
    const value = useThumbnail(cache, map, 1);
    return <span>{String(value)}</span>;
  }

  it("requests_on_mount_and_rerenders_only_for_its_own_key", async () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async (b) => `data:${b.byteLength}`);
    const onRender = vi.fn();
    act(() => root.render(<Probe cache={cache} map={4} onRender={onRender} />));
    expect(container.textContent).toBe("undefined");
    expect(glb).toHaveBeenCalledWith(4, 1);

    await flush();
    expect(container.textContent).toBe("data:3");
    const renders = onRender.mock.calls.length;

    cache.request(9, 1);
    await flush();
    expect(cache.get(9, 1)).toBe("data:3");
    expect(onRender.mock.calls.length).toBe(renders);
  });

  it("a_null_map_requests_nothing", () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async () => "data:x");
    act(() => root.render(<Probe cache={cache} map={null} onRender={() => {}} />));
    expect(glb).not.toHaveBeenCalled();
    expect(container.textContent).toBe("undefined");
  });
});
