import type { SceneHost } from "./SceneHost";
import type { MapBuildings, SceneRefs } from "./types";

/** The buildings of the maps the scene holds, in matrix order, each map with its building models; maps without buildings are left out. */
export function buildingsOf(h: Pick<SceneHost, "matrix" | "loaded">): MapBuildings[] {
  const shown = [...new Set((h.matrix?.maps ?? []).filter((m): m is number => m !== null))];
  return shown.flatMap((map) => {
    const buildings = h.loaded.get(map)?.map.buildings ?? [];
    return buildings.length > 0 ? [{ map, models: buildings.map((b) => b.model_id) }] : [];
  });
}

/** Tells the owner which buildings the shown maps have now (after a load, an undo, an add or a delete). */
export function reportBuildings(h: SceneHost, refs: SceneRefs): void {
  refs.callbacks.current.onBuildings(buildingsOf(h));
}
