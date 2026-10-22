import { describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import { ThumbnailCache, TransientRenderError, thumbnailArea, thumbnailKey } from "./thumbnails";

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
}
function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/** A source whose `glb` calls stay open until the test settles them, and a render that echoes the buffer length. */
function setup(options?: { capacity?: number; concurrency?: number }) {
  const calls: { key: string; bytes: Deferred<ArrayBuffer> }[] = [];
  const glb = vi.fn((map: number, area: number) => {
    const bytes = deferred<ArrayBuffer>();
    calls.push({ key: thumbnailKey(map, area), bytes });
    return bytes.promise;
  });
  const render = vi.fn(async (buffer: ArrayBuffer) => `data:image/png;base64,${buffer.byteLength}`);
  const cache = new ThumbnailCache({ glb }, render, options);
  return { cache, glb, render, calls };
}

describe("ThumbnailCache", () => {
  it("renders_once_per_key_and_shares_requests", async () => {
    const { cache, glb, render, calls } = setup();
    const seen: string[] = [];
    cache.subscribe((key) => seen.push(key));

    cache.request(5, 1);
    cache.request(5, 1);
    cache.request(5, 1);
    expect(cache.get(5, 1)).toBeUndefined();
    expect(cache.pending()).toBe(1);

    calls[0].bytes.resolve(new ArrayBuffer(7));
    await flush();

    expect(glb).toHaveBeenCalledTimes(1);
    expect(render).toHaveBeenCalledTimes(1);
    expect(cache.get(5, 1)).toBe("data:image/png;base64,7");
    expect(seen).toEqual(["5:1"]);
    expect(cache.pending()).toBe(0);

    cache.request(5, 1);
    await flush();
    expect(glb).toHaveBeenCalledTimes(1);
  });

  it("limits_concurrency_in_request_order", async () => {
    const { cache, glb, calls } = setup({ concurrency: 2 });
    for (const map of [1, 2, 3, 4, 5]) cache.request(map, 0);
    expect(glb).toHaveBeenCalledTimes(2);
    expect(calls.map((c) => c.key)).toEqual(["1:0", "2:0"]);
    expect(cache.pending()).toBe(5);

    calls[0].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(glb).toHaveBeenCalledTimes(3);
    expect(calls.map((c) => c.key)).toEqual(["1:0", "2:0", "3:0"]);

    calls[1].bytes.resolve(new ArrayBuffer(1));
    calls[2].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(calls.map((c) => c.key)).toEqual(["1:0", "2:0", "3:0", "4:0", "5:0"]);
    expect(cache.pending()).toBe(2);
  });

  it("a_failed_thumbnail_is_not_requested_again", async () => {
    const { cache, glb, calls } = setup();
    const seen: string[] = [];
    cache.subscribe((key) => seen.push(key));

    cache.request(9, 2);
    calls[0].bytes.reject({ code: "tool_missing", message: "apicula missing", asset: null });
    await flush();

    expect(cache.get(9, 2)).toBeNull();
    expect(cache.failure(9, 2)).toBe("apicula missing");
    expect(seen).toEqual(["9:2"]);

    cache.request(9, 2);
    await flush();
    expect(glb).toHaveBeenCalledTimes(1);
    expect(seen).toEqual(["9:2"]);
  });

  it("a_failing_render_counts_as_a_failure", async () => {
    const { cache, render, calls } = setup();
    render.mockRejectedValueOnce(new Error("no webgl"));
    cache.request(1, 1);
    calls[0].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(cache.get(1, 1)).toBeNull();
    expect(cache.failure(1, 1)).toBe("Error: no webgl");
  });

  it("reports_a_missing_tool_once_until_the_cache_is_cleared", async () => {
    const { cache, calls } = setup({ concurrency: 3 });
    const reported: string[] = [];
    cache.onToolMissing((e) => reported.push(e.message));
    const missing = { code: "tool_missing", message: "apicula missing", asset: null };
    for (const map of [1, 2, 3]) cache.request(map, 0);
    calls[0].bytes.reject(missing);
    calls[1].bytes.reject(missing);
    calls[2].bytes.reject({ code: "error", message: "boom", asset: null });
    await flush();
    expect(reported).toEqual(["apicula missing"]);
    cache.clear();
    cache.request(1, 0);
    calls[3].bytes.reject(missing);
    await flush();
    expect(reported).toEqual(["apicula missing", "apicula missing"]);
  });

  it("forgets_a_transient_render_failure_so_the_picture_can_be_asked_for_again", async () => {
    const { cache, glb, render, calls } = setup();
    const seen: string[] = [];
    cache.subscribe((key) => seen.push(key));
    render.mockRejectedValueOnce(new TransientRenderError("WebGL-Kontext verloren"));
    cache.request(4, 1);
    calls[0].bytes.resolve(new ArrayBuffer(3));
    await flush();
    // Neither a picture nor a failure: a blank picture is never cached, and nothing is pending.
    expect(cache.get(4, 1)).toBeUndefined();
    expect(cache.failure(4, 1)).toBeNull();
    expect(cache.pending()).toBe(0);
    expect(seen).toEqual(["4:1"]);
    cache.request(4, 1);
    expect(glb).toHaveBeenCalledTimes(2);
    calls[1].bytes.resolve(new ArrayBuffer(3));
    await flush();
    expect(cache.get(4, 1)).toBe("data:image/png;base64,3");
  });

  it("evicts_the_oldest_entry_beyond_capacity", async () => {
    const { cache, calls } = setup({ capacity: 2, concurrency: 1 });
    for (const map of [1, 2, 3]) {
      cache.request(map, 0);
      calls[calls.length - 1].bytes.resolve(new ArrayBuffer(map));
      await flush();
    }
    expect(cache.get(1, 0)).toBeUndefined();
    expect(cache.get(2, 0)).toBe("data:image/png;base64,2");
    expect(cache.get(3, 0)).toBe("data:image/png;base64,3");
  });

  it("eviction_is_least_recently_used_and_tells_subscribers", async () => {
    const { cache, calls } = setup({ capacity: 2, concurrency: 1 });
    const seen: string[] = [];
    for (const map of [1, 2]) {
      cache.request(map, 0);
      calls[calls.length - 1].bytes.resolve(new ArrayBuffer(map));
      await flush();
    }
    cache.get(1, 0); // 1 was used last, so 2 is the oldest
    cache.subscribe((key) => seen.push(key));
    cache.request(3, 0);
    calls[calls.length - 1].bytes.resolve(new ArrayBuffer(3));
    await flush();
    expect(cache.get(2, 0)).toBeUndefined();
    expect(cache.get(1, 0)).toBeDefined();
    expect(seen).toEqual(["3:0", "2:0"]);
  });

  it("a_failed_entry_survives_eviction_pressure", async () => {
    const { cache, glb, calls } = setup({ capacity: 1, concurrency: 1 });
    cache.request(1, 0);
    calls[0].bytes.reject({ code: "x", message: "boom", asset: null });
    await flush();
    for (const map of [2, 3, 4]) {
      cache.request(map, 0);
      calls[calls.length - 1].bytes.resolve(new ArrayBuffer(map));
      await flush();
    }
    expect(cache.get(1, 0)).toBeNull();
    cache.request(1, 0);
    expect(glb).toHaveBeenCalledTimes(4);
  });

  it("the_concurrency_cap_holds_after_a_rejection_with_queued_jobs", async () => {
    const { cache, glb, calls } = setup({ concurrency: 1 });
    for (const map of [1, 2, 3]) cache.request(map, 0);
    expect(glb).toHaveBeenCalledTimes(1);
    calls[0].bytes.reject({ code: "x", message: "boom", asset: null });
    await flush();
    expect(glb).toHaveBeenCalledTimes(2);
    expect(calls.map((c) => c.key)).toEqual(["1:0", "2:0"]);
    calls[1].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(glb).toHaveBeenCalledTimes(3);
  });

  it("clear_forgets_everything_including_failures", async () => {
    const { cache, glb, calls } = setup();
    cache.request(1, 0);
    cache.request(2, 0);
    calls[0].bytes.resolve(new ArrayBuffer(1));
    calls[1].bytes.reject({ code: "x", message: "boom", asset: null });
    await flush();
    expect(cache.get(1, 0)).toBeDefined();
    expect(cache.get(2, 0)).toBeNull();

    cache.clear();
    expect(cache.get(1, 0)).toBeUndefined();
    expect(cache.get(2, 0)).toBeUndefined();
    expect(cache.failure(2, 0)).toBeNull();

    cache.request(2, 0);
    expect(glb).toHaveBeenCalledTimes(3);
  });

  it("clear_drops_work_that_was_still_running", async () => {
    const { cache, calls } = setup();
    const seen: string[] = [];
    cache.subscribe((key) => seen.push(key));
    cache.request(1, 0);
    cache.clear();
    expect(seen).toEqual(["1:0"]);
    calls[0].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(cache.get(1, 0)).toBeUndefined();
    expect(seen).toEqual(["1:0"]);
    expect(cache.pending()).toBe(0);
  });

  it("cancel_drops_a_queued_request_and_leaves_a_running_one_alone", async () => {
    const { cache, glb, calls } = setup({ concurrency: 1 });
    const seen: string[] = [];
    cache.subscribe((key) => seen.push(key));
    cache.request(1, 1); // runs
    cache.request(2, 1); // queued
    cache.request(3, 1); // queued
    expect(cache.pending()).toBe(3);

    cache.cancel(2, 1);
    expect(cache.pending()).toBe(2);
    expect(seen).toEqual(["2:1"]);
    cache.cancel(1, 1); // running: untouched
    cache.cancel(9, 9); // unknown: untouched
    expect(cache.pending()).toBe(2);
    expect(seen).toEqual(["2:1"]);

    calls[0].bytes.resolve(new ArrayBuffer(1));
    await flush();
    calls[1].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(glb.mock.calls).toEqual([
      [1, 1],
      [3, 1],
    ]);
    expect(cache.get(1, 1)).toBeDefined();
    expect(cache.get(2, 1)).toBeUndefined();
    expect(cache.pending()).toBe(0);

    cache.request(2, 1); // a cancelled key can be requested again
    await flush();
    expect(glb).toHaveBeenCalledWith(2, 1);
  });

  it("unsubscribe_stops_notifications", async () => {
    const { cache, calls } = setup();
    const seen: string[] = [];
    const off = cache.subscribe((key) => seen.push(key));
    off();
    cache.request(1, 0);
    calls[0].bytes.resolve(new ArrayBuffer(1));
    await flush();
    expect(seen).toEqual([]);
  });
});

describe("thumbnailArea", () => {
  const headers = [{ id: 7, area_data_id: 3 }] as HeaderSummary[];
  it("thumbnailArea_prefers_the_cell_header", () => {
    expect(thumbnailArea(7, headers, 1)).toBe(3);
    expect(thumbnailArea(null, headers, 1)).toBe(1);
    expect(thumbnailArea(99, headers, 1)).toBe(1);
  });
});
