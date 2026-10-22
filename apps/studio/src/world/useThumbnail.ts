import { useCallback, useEffect, useSyncExternalStore } from "react";
import { thumbnailKey } from "./thumbnails";
import type { ThumbnailCache } from "./thumbnails";

/**
 * The picture of map `map` drawn with the tileset of `area`: a Data-URL, null when it failed for good, undefined while
 * it is on its way (or `map` is null). Requests it on mount and re-renders only when its own picture changes.
 */
export function useThumbnail(cache: ThumbnailCache, map: number | null, area: number): string | null | undefined {
  const subscribe = useCallback(
    (onChange: () => void) => {
      if (map === null) return () => {};
      const key = thumbnailKey(map, area);
      return cache.subscribe((changed) => {
        if (changed === key) onChange();
      });
    },
    [cache, map, area],
  );
  const getSnapshot = useCallback(() => (map === null ? undefined : cache.get(map, area)), [cache, map, area]);
  const value = useSyncExternalStore(subscribe, getSnapshot);
  useEffect(() => {
    if (map !== null) cache.request(map, area);
  }, [cache, map, area]);
  return value;
}
