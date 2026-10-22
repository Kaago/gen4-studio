import { memo, useEffect, useRef, useState } from "react";
import { errorText, getMap, getMapModelGlb, mapModelTextureStatus } from "../api";
import type { TextureIssue } from "../api";
import type { Message } from "../Toolbar";
import type { HeaderSummary, MatrixDto, TileBehavior } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import type { BuildingRef } from "./buildingLayer";
import { reconcileBuildings } from "./buildings";
import type { BuildingRefresh } from "./buildings";
import type { EventLock, EventRef, EventRefresh } from "./events";
import { selectMarker } from "./markers";
import { MODEL_CONCURRENCY, limitConcurrency, textureNote } from "./load";
import { MAP_SIZE } from "./layout";
import { disposeTree, parseGlb } from "./sceneUtil";
import { drainWhenReady, fetchCurrent, pendingSince } from "../reload";
import type { RefreshQueue } from "../reload";
import { buildingModelJob, dropMissingSelection, loadMissingBuildingModels } from "./scene/buildingModels";
import { addModel, buildContent, recolourMap, removeModels, setMarkers } from "./scene/content";
import { eventsNote, fetchEvents, refreshEvents } from "./scene/eventFetch";
import { loadNames } from "./scene/tileNames";
import type { Loaded, ReportEvents } from "./scene/types";
import type { SceneHost } from "./scene/SceneHost";
import { useSceneRefs } from "./scene/useSceneRefs";
import { useSceneSetup } from "./scene/useSceneSetup";

interface Props {
  matrix: MatrixDto;
  /**
   * Area data of the shown header; selects the map texture set for every map of the matrix. It
   * belongs to `matrix` (App changes both in one update when another header is shown). Changed
   * alone, or with a changed `modelsToken`, only the models reload, not the maps and markers.
   */
  areaDataId: number;
  /** Increment to reload the models alone (the shown area data's tilesets were edited). */
  modelsToken: number;
  showPermissions: boolean;
  /** Event file of the selected header; its events are shown as markers (null: none). */
  eventFileId: number | null;
  showEvents: boolean;
  /** Header list, for the internal names of warp destinations. */
  headers: HeaderSummary[];
  /** Painting is ignored while false (e.g. during a build). */
  paintEnabled: boolean;
  /**
   * Bumped by every undo and redo, at once (not through a render). A matrix load that was in its
   * map phase meanwhile fetches again, so it never shows the state from before the undo.
   */
  history: { readonly current: number };
  /** A new request makes the scene refetch one map (overlay colours and buildings of all its cells): undo or redo restored it. */
  mapRefresh: RefreshQueue<number> | null;
  /** Called after a tile was changed in the backend. */
  onEdited: () => void;
  onError: (message: string) => void;
  /** Reports what the inspector should show (filled in by tile hover and event markers). */
  onInspect: (model: InspectorModel) => void;
  /** Load progress and the summary of what failed to load (null: nothing to show, clears the progress). */
  onStatus: (message: Message | null) => void;
  /** The selected building (both null: none). The scene outlines it and lets it be dragged. */
  selectedMap: number | null;
  selectedBuilding: number | null;
  /** A click on a building (or null for none). The owner decides what to select and passes it back. */
  onSelectBuilding: (ref: BuildingRef | null) => void;
  /** A building was moved by dragging and the move is written. */
  onBuildingMoved: (map: number) => void;
  /** A new request makes the scene read one map's buildings again (they were edited elsewhere). */
  buildingRefresh: RefreshQueue<BuildingRefresh> | null;
  /** The selected event (null: none). The scene outlines it and lets it be dragged. */
  selectedEvent: EventRef | null;
  /** A click on an event marker (or null for none). The owner decides what to select and passes it back. */
  onSelectEvent: (ref: EventRef | null) => void;
  /** An event was moved by dragging and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** A new request makes the scene read one event file's events again (they were edited elsewhere). */
  eventRefresh: EventRefresh | null;
  /** The shown event file's lock state, reported whenever its events were read (null: not locked, or none shown). */
  onEventsLocked: (lock: EventLock | null) => void;
}

function MatrixScene({
  matrix,
  areaDataId,
  modelsToken,
  showPermissions,
  eventFileId,
  showEvents,
  headers,
  paintEnabled,
  history,
  mapRefresh,
  onEdited,
  onError,
  onInspect,
  onStatus,
  selectedMap,
  selectedBuilding,
  onSelectBuilding,
  onBuildingMoved,
  buildingRefresh,
  selectedEvent,
  onSelectEvent,
  onEventMoved,
  eventRefresh,
  onEventsLocked,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [eventStatus, setEventStatus] = useState<string | null>(null);
  // Counts the scenes built (maps fetched, content in place): queued building edits are read again then.
  const [sceneBuilt, setSceneBuilt] = useState(0);
  const refs = useSceneRefs({
    showPermissions,
    showEvents,
    eventFileId,
    headers,
    areaDataId,
    callbacks: { paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved, onEventsLocked },
  });
  const { handle, callbacks, showPermissions: showRef, showEvents: showEventsRef, eventFile: eventFileRef, area: areaRef } = refs;
  /** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
  const reportEvents: ReportEvents = (fetched) => {
    setEventStatus(eventsNote(fetched));
    const events = fetched.events;
    callbacks.current.onEventsLocked(events?.locked ? { file: events.event_file_id, reason: events.locked } : null);
  };

  // Renderer, camera and controls live as long as the component.
  useSceneSetup(host, refs, reportEvents);

  // (Re)load the scene. A new matrix loads the maps first (overlay and markers appear as soon as
  // they are there), then the models, a few at a time, each hung into the scene as it arrives.
  // The same matrix with another area data (or `modelsToken`) reloads only the models. Every run
  // supersedes the one before it: that one's pending work is dropped when it comes back.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    let cancelled = false;
    const area = areaDataId;
    // The scene shows this matrix's maps already when only the models are to be replaced.
    const modelsOnly = h.matrix === matrix;
    /** Status updates of a superseded load are dropped. */
    const report = (message: Message | null) => {
      if (!cancelled) callbacks.current.onStatus(message);
    };
    if (!modelsOnly) {
      report({ kind: "info", text: "Lade Karten ..." });
      setEventStatus(null);
    }
    const stale = () => cancelled || handle.current !== h;

    (async () => {
      if (modelsOnly) {
        removeModels(h);
        // The building models are textured by the area data too: they show boxes until reloaded.
        h.buildings?.resetModels();
        h.render();
      } else {
        // Without names the overlay is uncoloured by class and the hover shows raw values only.
        const problems: SceneHost["loadProblems"] = { names: null, maps: [] };
        const namesPending = loadNames().catch((e) => {
          problems.names = `Kachelnamen nicht geladen: ${errorText(e)}`;
          return new Map<number, TileBehavior>();
        });
        const ids = [...new Set(matrix.maps.filter((m): m is number => m !== null))];
        // Maps and events are fetched together; an undo or redo that lands meanwhile makes them
        // be fetched again. A failure of the events only costs the markers, never the scene.
        // The names are awaited inside, so no undo can land between the history check and the build.
        const { loaded, fetchedEvents, mapProblems, names } = await fetchCurrent(
          async () => {
            const loaded = new Map<number, Loaded>();
            const mapProblems: string[] = [];
            const [fetchedEvents, , names] = await Promise.all([
              fetchEvents(eventFileRef.current),
              Promise.all(
                ids.map(async (id) => {
                  try {
                    loaded.set(id, { map: await getMap(id) });
                  } catch (e) {
                    mapProblems.push(`Karte ${id}: ${errorText(e)}`);
                  }
                }),
              ),
              namesPending,
            ]);
            return { loaded, fetchedEvents, mapProblems, names };
          },
          () => history.current,
        );
        problems.maps = mapProblems;
        if (cancelled || handle.current !== h) return;
        const { content, overlay, tiles, bases, buildings } = buildContent(matrix, loaded, names);
        h.scene.remove(h.content);
        h.buildings?.dispose();
        disposeTree(h.content);
        h.content = content;
        h.buildings = buildings;
        buildings.select(h.selection);
        h.models = [];
        h.loadProblems = problems;
        h.overlay = overlay;
        h.matrix = matrix;
        h.loaded = loaded;
        h.tiles = tiles;
        h.bases = bases;
        h.names = names;
        overlay.visible = showRef.current;
        h.scene.add(content);
        setMarkers(h, fetchedEvents.events, showEventsRef.current);
        reportEvents(fetchedEvents);

        const w = matrix.width * MAP_SIZE;
        const d = matrix.height * MAP_SIZE;
        const span = Math.max(w, d);
        h.controls.target.set(w / 2, 0, d / 2);
        h.camera.position.set(w / 2, span * 0.9, d / 2 + span * 0.7);
        h.controls.update();
        h.render();
        h.refreshHover?.();
        setSceneBuilt((n) => n + 1);
      }

      // Models: a task that has not started when the load is superseded never starts; a started
      // one finishes, but its scene is disposed instead of added.
      const modelIds = [...h.loaded.keys()];
      const textureIssues: TextureIssue[] = [];
      const mapTasks = modelIds.map((id) => async () => {
        if (stale()) return;
        const model = await getMapModelGlb(id, area).then(parseGlb);
        // The backend falls back to an untextured model when the tileset is unusable; ask why.
        // A failing status call only costs the note.
        await mapModelTextureStatus(id, area)
          .then((issue) => issue && textureIssues.push(issue))
          .catch(() => undefined);
        if (stale()) {
          disposeTree(model);
          return;
        }
        addModel(h, matrix, id, model);
        h.queueRender();
      });
      // Building models come after the terrain, through the same bounded loader.
      const layer = h.buildings;
      const buildingIds = layer ? layer.missingModelIds() : [];
      const buildingTasks = layer ? buildingIds.map((id) => buildingModelJob(h, layer, id, area, stale)) : [];
      const labels = [...modelIds.map((id) => `Map ${id}`), ...buildingIds.map((id) => `Gebäudemodell ${id}`)];
      const tasks = [...mapTasks, ...buildingTasks];
      let lastReport = 0;
      const progress = (done: number, total: number) => {
        // The status bar re-renders the whole workbench: at most ten updates per second.
        const now = performance.now();
        if (done < total && now - lastReport < 100) return;
        lastReport = now;
        report({ kind: "info", text: `Modelle ${done}/${total}` });
      };
      progress(0, tasks.length);
      const results = await limitConcurrency(tasks, MODEL_CONCURRENCY, progress);
      const modelErrors = results.flatMap((r, i) => (r.status === "rejected" ? [`${labels[i]}: ${errorText(r.reason)}`] : []));
      const { names: namesError, maps: mapErrors } = h.loadProblems;
      const problems = [
        namesError,
        mapErrors.length ? `${mapErrors.length} Karte(n) nicht geladen: ${mapErrors[0]}` : null,
        modelErrors.length ? `${modelErrors.length} Modell(e) nicht geladen: ${modelErrors[0]}` : null,
      ].filter((p): p is string => p !== null);
      const note = textureNote(textureIssues);
      if (problems.length) report({ kind: "error", text: [...problems, note].filter((p) => p !== null).join(" · ") });
      else report(note ? { kind: "info", text: note } : null);
    })().catch((e) => report({ kind: "error", text: errorText(e) }));

    return () => {
      cancelled = true;
    };
  }, [matrix, areaDataId, modelsToken]);

  // Undo or redo restored maps: refetch each and show its overlay colours and buildings in every
  // cell that shows it. The requests are a queue (several can arrive before this runs, for
  // different maps) and are never cancelled by newer ones; a result is dropped when the scene shows
  // another matrix by then, or when a newer request for the same map was made.
  const mapRequests = useRef(new Map<number, number>());
  const mapsSeen = useRef(0);
  useEffect(() => {
    const { items, last } = pendingSince(mapRefresh, mapsSeen.current);
    mapsSeen.current = last;
    const h = handle.current;
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
        if (h.disposed || handle.current !== h || h.matrix !== matrixAtStart || mapRequests.current.get(map) !== seq || !data) return;
        // An unchanged building keeps its object, which the layer's slots hold: map data and layer
        // never point at different objects.
        const { buildings, changed } = reconcileBuildings(data.map.buildings, fresh.buildings);
        data.map = { ...fresh, buildings };
        recolourMap(h, map);
        const layer = h.buildings;
        if (changed) layer?.syncMap(matrixAtStart, map, buildings);
        dropMissingSelection(h, callbacks.current.onSelectBuilding);
        h.render();
        h.refreshHover?.();
        if (layer) await loadMissingBuildingModels(h, layer, areaRef.current, () => h.disposed || handle.current !== h || h.buildings !== layer);
      })().catch((e) => {
        if (!h.disposed) callbacks.current.onError(errorText(e));
      });
    }
  }, [mapRefresh]);

  // The selection (made by a click, the header list, ...) is outlined; the scene also drags it.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    h.selection = selectedMap === null || selectedBuilding === null ? null : { map: selectedMap, index: selectedBuilding };
    h.buildings?.select(h.selection);
    h.render();
  }, [selectedMap, selectedBuilding]);

  // A building of one map was edited elsewhere (inspector, add, delete): read that map's buildings
  // again and show them; a model that is new (or asked for again) loads in the background. A queue
  // like the one above: requests for different maps made in one batch are all handled.
  const buildingRequests = useRef(new Map<number, number>());
  const buildingsSeen = useRef(0);
  useEffect(() => {
    const h = handle.current;
    // Edits made while the first load still fetches maps stay queued; `sceneBuilt` runs this again.
    const { items, last } = drainWhenReady(buildingRefresh, buildingsSeen.current, !!(h?.matrix && h.buildings));
    buildingsSeen.current = last;
    if (!items.length || !h?.matrix || !h.buildings) return;
    const layer = h.buildings;
    const matrixAtStart = h.matrix;
    const stale = () => h.disposed || handle.current !== h || h.buildings !== layer;
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
        dropMissingSelection(h, callbacks.current.onSelectBuilding);
        h.render();
        await loadMissingBuildingModels(h, layer, areaRef.current, stale);
      })().catch((e) => {
        if (!stale()) callbacks.current.onError(errorText(e));
      });
    }
  }, [buildingRefresh, sceneBuilt]);

  // The selected event (made by a click, the header list, ...) is outlined; the scene also drags it.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    h.selectedEvent = selectedEvent;
    if (h.markers) selectMarker(h.markers, selectedEvent && selectedEvent.file === h.events?.event_file_id ? selectedEvent : null);
    h.render();
  }, [selectedEvent]);

  // Events of one file were edited elsewhere (inspector, add, delete): show them again.
  useEffect(() => {
    const h = handle.current;
    const request = eventRefresh;
    if (!request || !h?.matrix || eventFileRef.current !== request.file) return;
    refreshEvents(h, request.file, showEventsRef.current, () => handle.current === h && eventFileRef.current === request.file, reportEvents).catch((e) => callbacks.current.onError(e));
  }, [eventRefresh]);

  useEffect(() => {
    showRef.current = showPermissions;
    const h = handle.current;
    if (h?.overlay) {
      h.overlay.visible = showPermissions;
      h.render();
    }
  }, [showPermissions]);

  useEffect(() => {
    showEventsRef.current = showEvents;
    const h = handle.current;
    if (h?.markers) {
      h.markers.group.visible = showEvents;
      h.render();
      // Hidden markers must stop being hovered, shown ones start to be.
      h.refreshHover?.();
    }
  }, [showEvents]);

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      <div ref={host} style={{ position: "absolute", inset: 0 }} />
      {eventStatus && (
        <div style={{ position: "absolute", left: 8, bottom: 8, color: "#e5e7eb", font: "12px system-ui", maxWidth: "70%" }}>
          {eventStatus}
        </div>
      )}
    </div>
  );
}

// Hover moves only the inspector state in App; the scene itself must not re-render for it.
export default memo(MatrixScene);
