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

  function Probe({ cache, map, onRender, enabled = true }: { cache: ThumbnailCache; map: number | null; onRender: () => void; enabled?: boolean }) {
    onRender();
    const value = useThumbnail(cache, map, 1, enabled);
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

  it("re_requests_a_key_the_cache_evicted", async () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async (b) => `data:${b.byteLength}`, { capacity: 1 });
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} />));
    await flush();
    expect(container.textContent).toBe("data:3");

    cache.request(9, 1); // evicts 4:1, the mounted probe's key
    await flush();
    await flush();
    expect(glb.mock.calls).toEqual([
      [4, 1],
      [9, 1],
      [4, 1],
    ]);
    expect(container.textContent).toBe("data:3");
  });

  it("re_requests_a_key_after_clear", async () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async (b) => `data:${b.byteLength}`);
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} />));
    await flush();
    act(() => cache.clear());
    expect(container.textContent).toBe("undefined");
    await flush();
    expect(glb).toHaveBeenCalledTimes(2);
    expect(container.textContent).toBe("data:3");
  });

  it("re_requests_a_key_cleared_while_in_flight", async () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async (b) => `data:${b.byteLength}`);
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} />));
    act(() => cache.clear()); // before the first picture arrived
    await flush();
    await flush();
    expect(glb).toHaveBeenCalledTimes(2);
    expect(container.textContent).toBe("data:3");
  });

  it("a_null_map_requests_nothing", () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async () => "data:x");
    act(() => root.render(<Probe cache={cache} map={null} onRender={() => {}} />));
    expect(glb).not.toHaveBeenCalled();
    expect(container.textContent).toBe("undefined");
  });

  it("asks_only_while_enabled", async () => {
    const glb = vi.fn(async () => new ArrayBuffer(3));
    const cache = new ThumbnailCache({ glb }, async (b) => `data:${b.byteLength}`);
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} enabled={false} />));
    await flush();
    expect(glb).not.toHaveBeenCalled();
    expect(container.textContent).toBe("undefined");

    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} enabled />));
    await flush();
    expect(glb).toHaveBeenCalledWith(4, 1);
    expect(container.textContent).toBe("data:3");
  });

  it("withdraws_a_queued_request_on_unmount_and_when_disabled", () => {
    const glb = vi.fn(() => new Promise<ArrayBuffer>(() => {}));
    const cache = new ThumbnailCache({ glb }, async () => "data:x", { concurrency: 1 });
    cache.request(1, 1); // occupies the only slot
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} />));
    expect(cache.pending()).toBe(2);
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} enabled={false} />));
    expect(cache.pending()).toBe(1);
    act(() => root.render(<Probe cache={cache} map={4} onRender={() => {}} />));
    expect(cache.pending()).toBe(2);
    act(() => root.render(<span />));
    expect(cache.pending()).toBe(1);
  });

  it("a_cancel_by_one_user_makes_another_user_of_the_same_key_ask_again", async () => {
    const glb = vi.fn((_map: number, _area: number) => new Promise<ArrayBuffer>(() => {}));
    const cache = new ThumbnailCache({ glb }, async () => "data:x", { concurrency: 1 });
    cache.request(1, 1); // occupies the only slot, so key 7 stays queued
    const other = document.createElement("div");
    const otherRoot = createRoot(other);
    act(() => {
      root.render(<Probe cache={cache} map={7} onRender={() => {}} />);
      otherRoot.render(<Probe cache={cache} map={7} onRender={() => {}} />);
    });
    expect(cache.pending()).toBe(2);
    act(() => root.render(<span />)); // its cancel drops the shared queued job, the other user asks again
    expect(cache.pending()).toBe(2);
    act(() => otherRoot.unmount());
    expect(cache.pending()).toBe(1);
  });
});
