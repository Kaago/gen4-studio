import { memo, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { errorText, getMap, getMapModelGlb, mapModelTextureStatus, setPermission, updateBuilding, updateEvent } from "../api";
import type { TextureIssue } from "../api";
import type { Message } from "../Toolbar";
import { headerLabel } from "../workbench/headers";
import type { BuildingDto, EventsDto, HeaderSummary, MatrixDto, TileBehavior, WarpDto } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import type { BuildingRef } from "./buildingLayer";
import { reconcileBuildings, sameBuilding, settleBuilding } from "./buildings";
import type { BuildingRefresh } from "./buildings";
import { DragController } from "./dragController";
import { clampToMatrix } from "./events";
import type { EventLock, EventRef, EventRefresh } from "./events";
import { describeTile } from "./hover";
import { MARKER_Y, disposeMarkers, moveMarker, pickMarker, selectMarker } from "./markers";
import type { MarkerSet } from "./markers";
import { MODEL_CONCURRENCY, limitConcurrency, textureNote } from "./load";
import { MAP_SIZE, cellOrigin } from "./layout";
import { BLOCKED, isClick, matrixCellAt, pickCell } from "./paint";
import { disposeTree, parseGlb } from "./sceneUtil";
import { drainWhenReady, fetchCurrent, pendingSince } from "../reload";
import type { RefreshQueue } from "../reload";
import { buildingModelJob, dropMissingSelection, loadMissingBuildingModels } from "./scene/buildingModels";
import { OVERLAY_Y, addModel, buildContent, recolourMap, removeModels, setMarkers } from "./scene/content";
import { eventsNote, fetchEvents, refreshEvents } from "./scene/eventFetch";
import { queueRender } from "./scene/handle";
import type { SceneHandle } from "./scene/handle";
import { loadNames } from "./scene/tileNames";
import type { Loaded } from "./scene/types";

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
  const handle = useRef<SceneHandle | null>(null);
  const showRef = useRef(showPermissions);
  const showEventsRef = useRef(showEvents);
  // Like the area data, the event file belongs to the header and is read when a matrix loads.
  const eventFileRef = useRef(eventFileId);
  eventFileRef.current = eventFileId;
  const headersRef = useRef(headers);
  headersRef.current = headers;
  const [eventStatus, setEventStatus] = useState<string | null>(null);
  // Counts the scenes built (maps fetched, content in place): queued building edits are read again then.
  const [sceneBuilt, setSceneBuilt] = useState(0);
  // Like the area data, the one the buildings are textured with is read when models are requested.
  const areaRef = useRef(areaDataId);
  areaRef.current = areaDataId;
  const callbacks = useRef({ paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved, onEventsLocked });
  callbacks.current = { paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved, onEventsLocked };
  /** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
  const reportEvents = (fetched: { events: EventsDto | null; error: string | null }) => {
    setEventStatus(eventsNote(fetched));
    const events = fetched.events;
    callbacks.current.onEventsLocked(events?.locked ? { file: events.event_file_id, reason: events.locked } : null);
  };

  // Renderer, camera and controls live as long as the component.
  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor("#1e2430");
    el.appendChild(renderer.domElement);
    renderer.domElement.style.display = "block";

    const scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight("#ffffff", "#667788", 1.6));
    const sun = new THREE.DirectionalLight("#ffffff", 1.5);
    sun.position.set(-40, 100, 60);
    scene.add(sun);
    const camera = new THREE.PerspectiveCamera(50, 1, 0.5, 8000);
    const controls = new OrbitControls(camera, renderer.domElement);
    const content = new THREE.Group();
    scene.add(content);

    const render = () => renderer.render(scene, camera);
    const resize = () => {
      const w = Math.max(el.clientWidth, 1);
      const h = Math.max(el.clientHeight, 1);
      renderer.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      render();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);
    controls.addEventListener("change", render);
    resize();
    const h: SceneHandle = {
      scene,
      camera,
      controls,
      content,
      overlay: null,
      render,
      matrix: null,
      models: [],
      loadProblems: { names: null, maps: [] },
      loaded: new Map(),
      tiles: null,
      bases: new Map(),
      names: new Map(),
      markers: null,
      events: null,
      selectedEvent: null,
      buildings: null,
      selection: null,
      refreshHover: null,
      renderQueued: false,
      disposed: false,
    };
    handle.current = h;

    // Painting: a click (pointer barely moved) sets collision on the tile under the cursor;
    // dragging still orbits.
    const canvas = renderer.domElement;
    const raycaster = new THREE.Raycaster();
    const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -OVERLAY_Y);
    /** Aims the shared raycaster along the view ray through a pointer position. */
    const aim = (clientX: number, clientY: number) => {
      const rect = canvas.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((clientX - rect.left) / rect.width) * 2 - 1,
        -((clientY - rect.top) / rect.height) * 2 + 1,
      );
      raycaster.setFromCamera(ndc, camera);
    };
    /** The nearest event marker under a pointer position, or null (also while markers are hidden). */
    const markerAt = (clientX: number, clientY: number) => {
      if (!h.markers?.group.visible) return null;
      aim(clientX, clientY);
      return pickMarker(h.markers, raycaster);
    };
    /** The tile under a pointer position, or null outside every loaded map cell. */
    const pickTile = (clientX: number, clientY: number) => {
      if (!h.matrix) return null;
      aim(clientX, clientY);
      const hit = raycaster.ray.intersectPlane(plane, new THREE.Vector3());
      if (!hit) return null;
      const m = h.matrix;
      const at = matrixCellAt(hit, m.width, m.height);
      if (!at) return null;
      const mapId = m.maps[at.row * m.width + at.col];
      const tile = mapId === null || mapId === undefined ? null : pickCell(hit, cellOrigin(at.col, at.row));
      if (mapId === null || mapId === undefined || !tile) return null;
      const data = h.loaded.get(mapId);
      if (!data) return null;
      return { mapId, data, col: at.col, row: at.row, x: tile.x, y: tile.y };
    };

    // Hover: the latest pointer position is resolved at most once per animation frame.
    let pointer: { x: number; y: number } | null = null;
    let frame = 0;
    let shown = "";
    const showHover = () => {
      frame = 0;
      // While something is dragged the inspector keeps showing it; the end of the drag looks again.
      if (buildingDrag.dragging || eventDrag.dragging) return;
      // A marker under the pointer wins over the tile below it.
      const marker = pointer ? markerAt(pointer.x, pointer.y) : null;
      if (marker) {
        const key = `event:${marker.kind}:${marker.data.index}`;
        if (key === shown) return;
        shown = key;
        const destId = marker.kind === "warp" ? (marker.data as WarpDto).dest_header_id : null;
        const destHeader = destId === null ? undefined : headersRef.current.find((x) => x.id === destId);
        const destName = destHeader && headerLabel(destHeader);
        callbacks.current.onInspect({ kind: "eventHover", eventFile: h.events?.event_file_id ?? 0, eventKind: marker.kind, index: marker.data.index, data: marker.data, destName });
        return;
      }
      const picked = pointer ? pickTile(pointer.x, pointer.y) : null;
      if (!picked) {
        if (shown !== "") callbacks.current.onInspect({ kind: "none" });
        shown = "";
        return;
      }
      const cell = picked.data.map.permissions[picked.y * MAP_SIZE + picked.x] ?? [0, 0];
      // The collision byte is part of the key so a painted tile refreshes while the pointer rests on it.
      const key = `${picked.mapId}:${picked.col}:${picked.row}:${picked.x}:${picked.y}:${cell[0]}:${cell[1]}`;
      if (key === shown) return;
      shown = key;
      callbacks.current.onInspect({
        kind: "tile",
        map: picked.mapId,
        x: picked.x,
        y: picked.y,
        absX: picked.col * MAP_SIZE + picked.x,
        absZ: picked.row * MAP_SIZE + picked.y,
        info: describeTile(cell, h.names),
      });
    };
    const scheduleHover = () => {
      if (frame === 0) frame = requestAnimationFrame(showHover);
    };
    const onMove = (e: PointerEvent) => {
      pointer = { x: e.clientX, y: e.clientY };
      scheduleHover();
    };
    const onLeave = () => {
      pointer = null;
      scheduleHover();
    };
    h.refreshHover = () => {
      shown = "";
      scheduleHover();
    };

    // Dragging the selected building moves it on the ground plane (buildings snap to half tiles).
    /** What a building drag holds on to: the building as it was at the press and the centre of its map cell (its positions are relative to it). */
    interface BuildingDrag {
      ref: BuildingRef;
      building: BuildingDto;
      centre: { x: number; z: number };
    }
    /** The point of `plane` under a pointer position, or null when the view ray never reaches it. */
    const planePoint = (clientX: number, clientY: number, plane: THREE.Plane) => {
      aim(clientX, clientY);
      const hit = raycaster.ray.intersectPlane(plane, new THREE.Vector3());
      return hit ? { x: hit.x, z: hit.z } : null;
    };
    /** Ends a drag: one `update_building` for the whole move; a refused move puts the building back. */
    const commitBuildingDrag = ({ ref, building, centre }: BuildingDrag, at: { x: number; z: number }) => {
      const next = { ...building, x: at.x - centre.x, z: at.z - centre.z };
      updateBuilding(ref.map, ref.index, { x: next.x, z: next.z }).then(
        () => {
          const data = h.loaded.get(ref.map);
          // The maps may have been reloaded while the command was in flight: then the scene already shows what the backend holds.
          if (!data || !settleBuilding(data.map.buildings, ref.index, building, next)) return;
          h.buildings?.show(ref, next);
          h.render();
          callbacks.current.onBuildingMoved(ref.map);
        },
        (err) => {
          const data = h.loaded.get(ref.map);
          const current = data?.map.buildings[ref.index];
          if (current && sameBuilding(current, building)) {
            h.buildings?.show(ref, building);
            h.render();
          }
          callbacks.current.onError(errorText(err));
        },
      );
      h.refreshHover?.();
    };
    const buildingDrag = new DragController<BuildingDrag, THREE.Plane>({ host: el, canvas, keys: window }, 0.5, {
      begin: (e) => {
        const selected = h.selection;
        if (!selected || !h.buildings || !callbacks.current.paintEnabled) return null;
        // A marker over the building is what a press there means (a click selects it).
        if (markerAt(e.clientX, e.clientY)) return null;
        aim(e.clientX, e.clientY);
        const hit = h.buildings.pick(raycaster);
        if (!hit || hit.ref.map !== selected.map || hit.ref.index !== selected.index) return null;
        const centre = { x: hit.cell.x + MAP_SIZE / 2, z: hit.cell.z + MAP_SIZE / 2 };
        return {
          subject: { ref: hit.ref, building: hit.building, centre },
          plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -hit.building.y),
          origin: { x: centre.x + hit.building.x, z: centre.z + hit.building.z },
        };
      },
      pointOn: planePoint,
      move: ({ ref, building, centre }, at) => {
        h.buildings?.show(ref, { ...building, x: at.x - centre.x, z: at.z - centre.z });
        h.render();
      },
      commit: commitBuildingDrag,
      cancel: ({ ref, building }) => {
        h.buildings?.show(ref, building);
        h.render();
        h.refreshHover?.();
      },
    });
    const detachBuildingDrag = buildingDrag.attach();

    // Dragging the selected event marker moves it on the ground plane in whole tiles (`origin` is the
    // event's tile, so the snapped positions are tiles too). A locked event file cannot be dragged.
    /** What an event drag holds on to: the marker set it started in, the event as it was at the press. */
    interface EventDrag {
      set: MarkerSet;
      file: number;
      kind: EventRef["kind"];
      data: { index: number; x: number; z: number };
    }
    /** Shows the event on its recorded tile again (a drag that did not take, or was refused). */
    const restoreEvent = ({ set, kind, data }: EventDrag) => {
      // The markers may have been rebuilt meanwhile: they show what the backend holds.
      if (h.markers !== set) return;
      moveMarker(set, { kind, index: data.index }, { x: data.x, z: data.z });
      h.render();
    };
    const eventDrag = new DragController<EventDrag, THREE.Plane>({ host: el, canvas, keys: window }, 1, {
      begin: (e) => {
        const selected = h.selectedEvent;
        const events = h.events;
        if (!selected || !events || events.locked !== null || !h.markers || selected.file !== events.event_file_id || !callbacks.current.paintEnabled) return null;
        aim(e.clientX, e.clientY);
        const hit = pickMarker(h.markers, raycaster);
        if (!hit || hit.kind !== selected.kind || hit.data.index !== selected.index) return null;
        return {
          subject: { set: h.markers, file: events.event_file_id, kind: hit.kind, data: { index: hit.data.index, x: hit.data.x, z: hit.data.z } },
          plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -MARKER_Y),
          origin: { x: hit.data.x, z: hit.data.z },
        };
      },
      pointOn: planePoint,
      // Events stay on the matrix: beyond its edge the marker stops, and so does what is written.
      clamp: (at) => (h.matrix ? clampToMatrix(at, h.matrix) : at),
      move: ({ set, kind, data }, at) => {
        if (h.markers !== set) return;
        moveMarker(set, { kind, index: data.index }, at);
        h.render();
      },
      // One `update_event` for the whole move; a refused move puts the marker back.
      commit: (d, at) => {
        updateEvent(d.file, d.kind, d.data.index, { x: at.x, z: at.z }).then(
          async () => {
            callbacks.current.onEventMoved(d.file);
            await refreshEvents(h, d.file, showEventsRef.current, () => eventFileRef.current === d.file, reportEvents);
          },
          (err) => {
            restoreEvent(d);
            callbacks.current.onError(err);
          },
        );
        h.refreshHover?.();
      },
      cancel: (d) => {
        restoreEvent(d);
        h.refreshHover?.();
      },
    });
    const detachEventDrag = eventDrag.attach();

    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
    };
    const onUp = (e: PointerEvent) => {
      const start = down;
      down = null;
      if (!start || e.button !== 0 || !isClick(start, { x: e.clientX, y: e.clientY })) return;
      const cb = callbacks.current;
      // A marker under the click is selected instead of anything below it.
      const marker = markerAt(e.clientX, e.clientY);
      if (marker && h.events) {
        cb.onSelectEvent({ file: h.events.event_file_id, kind: marker.kind, index: marker.data.index });
        return;
      }
      // A building under the click is selected instead of painting the tile below it.
      if (h.buildings) {
        aim(e.clientX, e.clientY);
        const hit = h.buildings.pick(raycaster);
        if (hit) {
          cb.onSelectBuilding(hit.ref);
          return;
        }
      }
      if (!cb.paintEnabled || !h.overlay?.visible) return;
      const picked = pickTile(e.clientX, e.clientY);
      if (!picked) return;
      const { mapId, data, x, y } = picked;
      const i = y * MAP_SIZE + x;
      const kind = data.map.permissions[i]?.[0] ?? 0;
      const collision = e.shiftKey ? 0x00 : BLOCKED;
      setPermission(mapId, x, y, kind, collision).then(
        () => {
          data.map.permissions[i] = [kind, collision];
          // The matrix may have been swapped while the command was in flight.
          if (h.loaded.get(mapId) === data) recolourMap(h, mapId);
          h.render();
          h.refreshHover?.();
          callbacks.current.onEdited();
        },
        (err) => callbacks.current.onError(errorText(err)),
      );
    };
    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerleave", onLeave);

    return () => {
      detachBuildingDrag();
      detachEventDrag();
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerleave", onLeave);
      cancelAnimationFrame(frame);
      callbacks.current.onInspect({ kind: "none" });
      callbacks.current.onStatus(null);
      const current = handle.current;
      handle.current = null;
      h.disposed = true;
      if (current?.markers) disposeMarkers(current.markers);
      current?.buildings?.dispose();
      observer.disconnect();
      controls.removeEventListener("change", render);
      controls.dispose();
      disposeTree(current ? current.content : content);
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, []);

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
        const problems: SceneHandle["loadProblems"] = { names: null, maps: [] };
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
        queueRender(h);
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
