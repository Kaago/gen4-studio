import { useCallback, useEffect, useSyncExternalStore } from "react";
import { thumbnailKey } from "./thumbnails";
import type { ThumbnailCache } from "./thumbnails";

/**
 * The picture of map `map` drawn with the tileset of `area`: a Data-URL, null when it failed for good, undefined while
 * it is on its way (or `map` is null). Requests it on mount, re-requests it when the cache dropped it (eviction,
 * `clear()`), and re-renders only when its own picture changes.
 */
export function useThumbnail(cache: ThumbnailCache, map: number | null, area: number): string | null | undefined {
  const subscribe = useCallback(
    (onChange: () => void) => {
      if (map === null) return () => {};
      const key = thumbnailKey(map, area);
      return cache.subscribe((changed) => {
        if (changed !== key) return;
        // Idempotent: a no-op unless the cache lost the entry, e.g. `clear()` dropped it while it was still on its way.
        if (cache.get(map, area) === undefined) cache.request(map, area);
        onChange();
      });
    },
    [cache, map, area],
  );
  const getSnapshot = useCallback(() => (map === null ? undefined : cache.get(map, area)), [cache, map, area]);
  const value = useSyncExternalStore(subscribe, getSnapshot);
  useEffect(() => {
    if (map !== null && value === undefined) cache.request(map, area);
  }, [cache, map, area, value]);
  return value;
}
