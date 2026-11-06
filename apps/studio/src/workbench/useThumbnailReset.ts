import { useEffect, useRef } from "react";
import type { EditBus } from "../bus/editBus";
import { useBusEvent } from "../bus/useBusEvent";
import { resetTileNames } from "../world/scene/tileNames";
import { resetWorldTextures } from "../world/scene/worldMesh";
import { resetSprites } from "../world/sprites";
import type { ThumbnailCache } from "../world/thumbnails";

/**
 * The map pictures are keyed by (map, area) for the whole app, so they go stale when the same keys can mean other
 * pictures: another project was opened (`root` changed) or the shown area data got other tilesets (`modelsReload`).
 * Empties `cache` then; mounting does not. The object sprites are keyed by graphics id for the whole app too, but
 * only depend on the project: they are dropped when `root` changes, not on `modelsReload`. So are the tile behaviour names, which depend on the
 * project's game family. The textures of the world meshes go on both: they depend on the project and on the package files, which a
 * reset to original rewrites under the same numbers (`modelsReload`).
 */
export function useThumbnailReset(bus: EditBus, root: string | null, cache: ThumbnailCache) {
  const shownRoot = useRef(root);
  useEffect(() => {
    if (shownRoot.current === root) return;
    shownRoot.current = root;
    cache.clear();
    resetSprites();
    resetWorldTextures();
    resetTileNames();
  }, [root, cache]);
  useBusEvent(bus, "modelsReload", () => {
    cache.clear();
    // The package's files were written under the same numbers: the world meshes take their textures anew.
    resetWorldTextures();
  });
}

/** Reports once (per project, per tileset change: until the cache is cleared) that pictures cannot be made because the conversion tool is missing. */
export function useThumbnailToolReport(cache: ThumbnailCache, onError: (e: unknown) => void) {
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => cache.onToolMissing((e) => report.current(e)), [cache]);
}
