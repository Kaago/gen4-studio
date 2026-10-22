import { useEffect, useRef } from "react";
import type { EditBus } from "../bus/editBus";
import { useBusEvent } from "../bus/useBusEvent";
import type { ThumbnailCache } from "../world/thumbnails";

/**
 * The map pictures are keyed by (map, area) for the whole app, so they go stale when the same keys can mean other
 * pictures: another project was opened (`root` changed) or the shown area data got other tilesets (`modelsReload`).
 * Empties `cache` then; mounting does not.
 */
export function useThumbnailReset(bus: EditBus, root: string | null, cache: ThumbnailCache) {
  const shownRoot = useRef(root);
  useEffect(() => {
    if (shownRoot.current === root) return;
    shownRoot.current = root;
    cache.clear();
  }, [root, cache]);
  useBusEvent(bus, "modelsReload", () => cache.clear());
}

/** Reports once (per project, per tileset change: until the cache is cleared) that pictures cannot be made because the conversion tool is missing. */
export function useThumbnailToolReport(cache: ThumbnailCache, onError: (e: unknown) => void) {
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => cache.onToolMissing((e) => report.current(e)), [cache]);
}
