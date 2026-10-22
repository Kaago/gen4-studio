import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";
import { thumbnailKey } from "./thumbnails";
import type { ThumbnailCache } from "./thumbnails";

/**
 * The picture of map `map` drawn with the tileset of `area`: a Data-URL, null when it failed for good, undefined while
 * it is on its way (or `map` is null). While `enabled` it requests the picture on mount, re-requests it when the cache
 * dropped it (eviction, `clear()`), and withdraws a still queued request on unmount or when disabled (a cell out of
 * view). It re-renders only when its own picture changes.
 */
export function useThumbnail(cache: ThumbnailCache, map: number | null, area: number, enabled = true): string | null | undefined {
  // Read by the subscription: a `cancel` of this very hook must not make it ask again.
  const wanted = useRef(false);
  const subscribe = useCallback(
    (onChange: () => void) => {
      if (map === null) return () => {};
      const key = thumbnailKey(map, area);
      return cache.subscribe((changed) => {
        if (changed !== key) return;
        // Idempotent: a no-op unless the cache lost the entry, e.g. `clear()` dropped it while it was still on its way.
        if (wanted.current && cache.get(map, area) === undefined) cache.request(map, area);
        onChange();
      });
    },
    [cache, map, area],
  );
  const getSnapshot = useCallback(() => (map === null ? undefined : cache.get(map, area)), [cache, map, area]);
  const value = useSyncExternalStore(subscribe, getSnapshot);
  useEffect(() => {
    if (map === null || !enabled || value !== undefined) return;
    wanted.current = true;
    cache.request(map, area);
    return () => {
      wanted.current = false;
      cache.cancel(map, area);
    };
  }, [cache, map, area, value, enabled]);
  return value;
}
