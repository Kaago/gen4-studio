import { errorText, getBuildingGlb } from "../../api";
import type { BuildingLayer, BuildingRef } from "../buildingLayer";
import { MODEL_CONCURRENCY, limitConcurrency } from "../load";
import { disposeTree, parseGlb } from "../sceneUtil";
import type { SceneHost } from "./SceneHost";

/**
 * A loader job for building model `id`: fetches and parses it, then hands it to the buildings that
 * use it. A failure is remembered in the layer (the buildings keep their box) and rethrown for the
 * caller's summary. Results that arrive after `stale()` turned true are discarded.
 */
export function buildingModelJob(h: SceneHost, layer: BuildingLayer, id: number, area: number, stale: () => boolean) {
  return async () => {
    if (stale()) return;
    try {
      const model = await getBuildingGlb(id, area).then(parseGlb);
      if (stale()) {
        disposeTree(model);
        return;
      }
      layer.setTemplate(id, model);
      h.queueRender();
    } catch (e) {
      if (!stale()) layer.setFailed(id, errorText(e));
      throw e;
    }
  };
}

/** Loads the building models the layer still lacks, a few at a time; never rejects. */
export async function loadMissingBuildingModels(h: SceneHost, layer: BuildingLayer, area: number, stale: () => boolean) {
  const ids = layer.missingModelIds();
  await limitConcurrency(
    ids.map((id) => buildingModelJob(h, layer, id, area, stale)),
    MODEL_CONCURRENCY,
  );
}

/** Drops the selection when its building no longer exists (after an undo, say). */
export function dropMissingSelection(h: SceneHost, onSelect: (ref: BuildingRef | null) => void) {
  if (!h.selection || !h.buildings || h.buildings.has(h.selection)) return;
  h.selection = null;
  h.buildings.select(null);
  onSelect(null);
}
