import { useEffect, useRef } from "react";
import { errorText, getMap } from "../../api";
import { drainWhenReady, pendingSince } from "../../reload";
import type { RefreshQueue } from "../../reload";
import { reconcileBuildings } from "../buildings";
import type { BuildingRefresh } from "../buildings";
import { dropMissingSelection, loadMissingBuildingModels } from "./buildingModels";
import { reportBuildings } from "./buildingReport";
import { recolourMap } from "./content";
import { refreshEvents } from "./eventFetch";
import type { ReportEvents, SceneRefs } from "./types";

/**
 * Undo or redo restored maps: refetch each and show its overlay colours and buildings in every
 * cell that shows it. The requests are a queue (several can arrive before this runs, for
 * different maps) and are never cancelled by newer ones; a result is dropped when the scene shows
 * another matrix by then, or when a newer request for the same map was made.
 */
export function useMapRefresh(refs: SceneRefs, mapRefresh: RefreshQueue<number> | null) {
  const mapRequests = useRef(new Map<number, number>());
  const mapsSeen = useRef(0);
  useEffect(() => {
    const { items, last } = pendingSince(mapRefresh, mapsSeen.current);
    mapsSeen.current = last;
    const h = refs.handle.current;
    // A scene that is still loading fetches current maps itself (see `history`).
    if (!h?.matrix) return;
    // Results are only applied to the matrix they were requested for.
    const matrixAtStart = h.matrix;
    for (const map of items) {
      const seq = (mapRequests.current.get(map) ?? 0) + 1;
      mapRequests.current.set(map, seq);
      (async () => {
        if (!h.loaded.has(map)) return;
        const fresh = await getMap(map);
        const data = h.loaded.get(map);
        if (h.disposed || refs.handle.current !== h || h.matrix !== matrixAtStart || mapRequests.current.get(map) !== seq || !data) return;
        // An unchanged building keeps its object, which the layer's slots hold: map data and layer
        // never point at different objects.
        const { buildings, changed } = reconcileBuildings(data.map.buildings, fresh.buildings);
        data.map = { ...fresh, buildings };
        recolourMap(h, map);
        const layer = h.buildings;
        if (changed) layer?.syncMap(matrixAtStart, map, buildings);
        dropMissingSelection(h, refs.callbacks.current.onSelectBuilding);
        reportBuildings(h, refs);
        h.render();
        h.refreshHover?.();
        if (layer) await loadMissingBuildingModels(h, layer, refs.area.current, () => h.disposed || refs.handle.current !== h || h.buildings !== layer);
      })().catch((e) => {
        if (!h.disposed) refs.callbacks.current.onError(errorText(e));
      });
    }
  }, [mapRefresh]);
}

/**
 * A building of one map was edited elsewhere (inspector, add, delete): read that map's buildings
 * again and show them; a model that is new (or asked for again) loads in the background. A queue
 * like the one above: requests for different maps made in one batch are all handled. Edits made
 * while the first load still fetches maps stay queued; `sceneBuilt` runs this again.
 */
export function useBuildingRefresh(refs: SceneRefs, buildingRefresh: RefreshQueue<BuildingRefresh> | null, sceneBuilt: number) {
  const buildingRequests = useRef(new Map<number, number>());
  const buildingsSeen = useRef(0);
  useEffect(() => {
    const h = refs.handle.current;
    const { items, last } = drainWhenReady(buildingRefresh, buildingsSeen.current, !!(h?.matrix && h.buildings));
    buildingsSeen.current = last;
    if (!items.length || !h?.matrix || !h.buildings) return;
    const layer = h.buildings;
    const matrixAtStart = h.matrix;
    const stale = () => h.disposed || refs.handle.current !== h || h.buildings !== layer;
    for (const request of items) {
      const seq = (buildingRequests.current.get(request.map) ?? 0) + 1;
      buildingRequests.current.set(request.map, seq);
      (async () => {
        const data = h.loaded.get(request.map);
        if (!data) return;
        const fresh = await getMap(request.map);
        if (stale() || h.matrix !== matrixAtStart) return;
        const superseded = buildingRequests.current.get(request.map) !== seq;
        // A newer request for this map shows the newer buildings; this one's retry still counts.
        if (!superseded) {
          data.map.buildings = fresh.buildings;
          layer.syncMap(matrixAtStart, request.map, fresh.buildings);
        }
        if (request.retryModel !== null) layer.retry(request.retryModel);
        dropMissingSelection(h, refs.callbacks.current.onSelectBuilding);
        reportBuildings(h, refs);
        h.render();
        await loadMissingBuildingModels(h, layer, refs.area.current, stale);
      })().catch((e) => {
        if (!stale()) refs.callbacks.current.onError(errorText(e));
      });
    }
  }, [buildingRefresh, sceneBuilt]);
}

/** Events of one file were edited elsewhere (inspector, add, delete): show them again. The requests are the file numbers, queued like the ones above. */
export function useEventRefresh(refs: SceneRefs, eventRefresh: RefreshQueue<number> | null, report: ReportEvents) {
  const eventsSeen = useRef(0);
  useEffect(() => {
    const { items, last } = pendingSince(eventRefresh, eventsSeen.current);
    eventsSeen.current = last;
    const h = refs.handle.current;
    if (!h?.matrix) return;
    // The requests of one batch are a queue (a request for another file does not replace one for the shown file); a file is read once however often it was asked for.
    for (const file of new Set(items)) {
      if (refs.eventFile.current !== file) continue;
      refreshEvents(h, file, refs.showEvents.current, () => refs.handle.current === h && refs.eventFile.current === file, report).catch((e) => refs.callbacks.current.onError(e));
    }
  }, [eventRefresh]);
}
