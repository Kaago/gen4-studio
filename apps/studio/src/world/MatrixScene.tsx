import { memo, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { errorText, getBuildingGlb, getEvents, getMap, getMapModelGlb, mapModelTextureStatus, setPermission, tileBehaviors, updateBuilding, updateEvent } from "../api";
import type { TextureIssue } from "../api";
import type { Message } from "../Toolbar";
import { headerLabel } from "../workbench/headers";
import type { BuildingDto, EventsDto, HeaderSummary, MapDto, MatrixDto, TileBehavior, WarpDto } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { BuildingLayer } from "./buildingLayer";
import type { BuildingRef } from "./buildingLayer";
import { reconcileBuildings, sameBuilding, settleBuilding } from "./buildings";
import type { BuildingRefresh } from "./buildings";
import { permissionColor } from "./colors";
import { DragController } from "./dragController";
import { clampToMatrix, lockedText } from "./events";
import type { EventRef, EventRefresh } from "./events";
import { describeTile } from "./hover";
import { MARKER_Y, buildMarkers, disposeMarkers, moveMarker, pickMarker, selectMarker } from "./markers";
import type { MarkerSet } from "./markers";
import { MODEL_CONCURRENCY, limitConcurrency, textureNote } from "./load";
import { MAP_SIZE, MODEL_SCALE, cellOrigin } from "./layout";
import { BLOCKED, isClick, matrixCellAt, pickCell } from "./paint";
import { cloneModel, disposeTree, parseGlb } from "./sceneUtil";

interface Loaded {
  map: MapDto;
}

/** Height of the overlay plane; clicks are intersected with the same plane. */
const OVERLAY_Y = 0.05;

interface SceneHandle {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  content: THREE.Group;
  overlay: THREE.Object3D | null;
  render: () => void;
  /** Matrix and loaded maps behind the current content (null until a matrix is loaded). */
  matrix: MatrixDto | null;
  /** The terrain model instances in `content` (one per matrix cell showing a map), for reloading them alone. */
  models: THREE.Object3D[];
  /** What the last full load could not read (names, maps), kept for the summary after a models-only reload. */
  loadProblems: { names: string | null; maps: string[] };
  loaded: Map<number, Loaded>;
  tiles: THREE.InstancedMesh | null;
  /** Map id to the first overlay instance of each matrix cell showing that map. */
  bases: Map<number, number[]>;
  /** Tile behaviour names by kind byte (empty until loaded). */
  names: Map<number, TileBehavior>;
  markers: MarkerSet | null;
  /** The events the markers show (null: none shown, or they could not be read). */
  events: EventsDto | null;
  /** The selected event, as the scene knows it (follows the `selectedEvent` prop). */
  selectedEvent: EventRef | null;
  /** The buildings of the shown matrix (a child of `content`; null until a matrix is loaded). */
  buildings: BuildingLayer | null;
  /** The selected building, as the scene knows it (follows the `selectedMap`/`selectedBuilding` props). */
  selection: BuildingRef | null;
  /** Re-resolves the tile under the pointer on the next frame (set by the setup effect). */
  refreshHover: (() => void) | null;
  /** A render is already queued for the next animation frame. */
  renderQueued: boolean;
  /** The component is gone; late results and queued renders are dropped. */
  disposed: boolean;
}

const colorCache = new Map<string, THREE.Color>();
function overlayColor(kind: number, collision: number, names: Map<number, TileBehavior>): THREE.Color {
  const hex = permissionColor(kind, collision, names.get(kind)?.class ?? null);
  let color = colorCache.get(hex);
  if (!color) colorCache.set(hex, (color = new THREE.Color(hex)));
  return color;
}

let namesCache: Promise<Map<number, TileBehavior>> | null = null;
/** The behaviour table is static, so it is fetched once; a failed fetch is retried on the next load. */
function loadNames(): Promise<Map<number, TileBehavior>> {
  namesCache ??= tileBehaviors()
    .then((list) => new Map(list.map((b) => [b.value, b] as const)))
    .catch((e) => {
      namesCache = null;
      throw e;
    });
  return namesCache;
}

/** Hands the model of map `mapId` to every matrix cell showing that map (clones share geometry and materials). */
function addModel(h: SceneHandle, matrix: MatrixDto, mapId: number, model: THREE.Object3D) {
  let used = false;
  matrix.maps.forEach((id, i) => {
    if (id !== mapId) return;
    const origin = cellOrigin(i % matrix.width, Math.floor(i / matrix.width));
    const instance = cloneModel(model);
    // The model is centred on the map centre.
    instance.scale.setScalar(MODEL_SCALE);
    instance.position.set(origin.x + MAP_SIZE / 2, 0, origin.z + MAP_SIZE / 2);
    h.content.add(instance);
    h.models.push(instance);
    used = true;
  });
  // The clones in `content` are disposed with it; a model without a cell would leak.
  if (!used) disposeTree(model);
}

/** Takes every terrain model out of the scene (the overlay, buildings and markers stay). */
function removeModels(h: SceneHandle) {
  for (const instance of h.models) {
    h.content.remove(instance);
    disposeTree(instance);
  }
  h.models = [];
}

/** Renders on the next animation frame; bursts of arriving models cost one render. */
function queueRender(h: SceneHandle) {
  if (h.renderQueued) return;
  h.renderQueued = true;
  requestAnimationFrame(() => {
    h.renderQueued = false;
    if (!h.disposed) h.render();
  });
}

/**
 * A loader job for building model `id`: fetches and parses it, then hands it to the buildings that
 * use it. A failure is remembered in the layer (the buildings keep their box) and rethrown for the
 * caller's summary. Results that arrive after `stale()` turned true are discarded.
 */
function buildingModelJob(h: SceneHandle, layer: BuildingLayer, id: number, area: number, stale: () => boolean) {
  return async () => {
    if (stale()) return;
    try {
      const model = await getBuildingGlb(id, area).then(parseGlb);
      if (stale()) {
        disposeTree(model);
        return;
      }
      layer.setTemplate(id, model);
      queueRender(h);
    } catch (e) {
      if (!stale()) layer.setFailed(id, errorText(e));
      throw e;
    }
  };
}

/** Loads the building models the layer still lacks, a few at a time; never rejects. */
async function loadMissingBuildingModels(h: SceneHandle, layer: BuildingLayer, area: number, stale: () => boolean) {
  const ids = layer.missingModelIds();
  await limitConcurrency(
    ids.map((id) => buildingModelJob(h, layer, id, area, stale)),
    MODEL_CONCURRENCY,
  );
}

/** Drops the selection when its building no longer exists (after an undo, say). */
function dropMissingSelection(h: SceneHandle, onSelect: (ref: BuildingRef | null) => void) {
  if (!h.selection || !h.buildings || h.buildings.has(h.selection)) return;
  h.selection = null;
  h.buildings.select(null);
  onSelect(null);
}

/** Builds the overlay and the building boxes once all maps are known; models are added later. */
function buildContent(matrix: MatrixDto, loaded: Map<number, Loaded>, names: Map<number, TileBehavior>) {
  const content = new THREE.Group();
  const cells: { col: number; row: number; data: Loaded }[] = [];
  matrix.maps.forEach((id, i) => {
    const data = id === null ? undefined : loaded.get(id);
    if (data) cells.push({ col: i % matrix.width, row: Math.floor(i / matrix.width), data });
  });

  const overlay = new THREE.Group();
  const perCell = MAP_SIZE * MAP_SIZE;
  const plane = new THREE.PlaneGeometry(1, 1);
  plane.rotateX(-Math.PI / 2);
  const planeMaterial = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: 0.45,
    depthTest: false,
    depthWrite: false,
  });
  const tiles = new THREE.InstancedMesh(plane, planeMaterial, cells.length * perCell);
  tiles.renderOrder = 10;
  tiles.frustumCulled = false;
  const m4 = new THREE.Matrix4();
  const bases = new Map<number, number[]>();
  let n = 0;
  for (const { col, row, data } of cells) {
    const origin = cellOrigin(col, row);
    const list = bases.get(data.map.index) ?? [];
    list.push(n);
    bases.set(data.map.index, list);
    for (let i = 0; i < perCell; i++) {
      const [kind, collision] = data.map.permissions[i] ?? [0, 0];
      m4.setPosition(origin.x + (i % MAP_SIZE) + 0.5, OVERLAY_Y, origin.z + Math.floor(i / MAP_SIZE) + 0.5);
      tiles.setMatrixAt(n, m4);
      tiles.setColorAt(n, overlayColor(kind, collision, names));
      n++;
    }
  }
  overlay.add(tiles);
  content.add(overlay);

  const buildings = new BuildingLayer();
  buildings.build(matrix, loaded);
  content.add(buildings.root);
  return { content, overlay, tiles, bases, buildings };
}

/** Replaces the scene's markers (`events` null: show none); the selected event keeps its outline. */
function setMarkers(h: SceneHandle, events: EventsDto | null, visible: boolean) {
  if (h.markers) {
    h.scene.remove(h.markers.group);
    disposeMarkers(h.markers);
    h.markers = null;
  }
  h.events = events;
  if (!h.matrix || !events) return;
  h.markers = buildMarkers(h.matrix, events);
  h.markers.group.visible = visible;
  const selected = h.selectedEvent;
  if (selected && selected.file === events.event_file_id) selectMarker(h.markers, selected);
  h.scene.add(h.markers.group);
}

/** The note shown over the scene about the events: why they are missing, or that they are locked. */
function eventsNote(fetched: { events: EventsDto | null; error: string | null }): string | null {
  if (fetched.error) return fetched.error;
  return fetched.events?.locked ? `Events ${lockedText(fetched.events.locked)}` : null;
}

/** Fetches the events of a header's event file; never rejects, a failure comes back as text. */
async function fetchEvents(eventFileId: number | null): Promise<{ events: EventsDto | null; error: string | null }> {
  if (eventFileId === null) return { events: null, error: null };
  try {
    return { events: await getEvents(eventFileId), error: null };
  } catch (e) {
    return { events: null, error: `Events nicht geladen: ${errorText(e)}` };
  }
}

/**
 * Reads one event file's events again and shows them (their markers are rebuilt). Dropped when
 * `current()` turned false meanwhile (another matrix or event file is shown now).
 */
async function refreshEvents(h: SceneHandle, file: number, visible: boolean, current: () => boolean, setNote: (note: string | null) => void) {
  const matrixAtStart = h.matrix;
  const fetched = await fetchEvents(file);
  if (h.disposed || h.matrix !== matrixAtStart || !current()) return;
  setMarkers(h, fetched.events, visible);
  setNote(eventsNote(fetched));
  h.render();
  h.refreshHover?.();
}

/** Recolours every overlay instance of `mapId` from the map's current permissions. */
function recolourMap(h: SceneHandle, mapId: number) {
  const data = h.loaded.get(mapId);
  const bases = h.bases.get(mapId);
  if (!data || !bases || !h.tiles) return;
  for (const base of bases) {
    data.map.permissions.forEach(([kind, collision], i) => {
      h.tiles?.setColorAt(base + i, overlayColor(kind, collision, h.names));
    });
  }
  if (h.tiles.instanceColor) h.tiles.instanceColor.needsUpdate = true;
}

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
  /** Increment to reload all maps of the matrix from the backend and recolour (after undo/redo). */
  reloadToken: number;
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
  buildingRefresh: BuildingRefresh | null;
  /** The selected event (null: none). The scene outlines it and lets it be dragged. */
  selectedEvent: EventRef | null;
  /** A click on an event marker (or null for none). The owner decides what to select and passes it back. */
  onSelectEvent: (ref: EventRef | null) => void;
  /** An event was moved by dragging and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** A new request makes the scene read one event file's events again (they were edited elsewhere). */
  eventRefresh: EventRefresh | null;
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
  reloadToken,
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
  // Like the area data, the one the buildings are textured with is read when models are requested.
  const areaRef = useRef(areaDataId);
  areaRef.current = areaDataId;
  const callbacks = useRef({ paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved });
  callbacks.current = { paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved };

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
            await refreshEvents(h, d.file, showEventsRef.current, () => eventFileRef.current === d.file, setEventStatus);
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
        // Fetched next to the maps; a failure only costs the markers, never the scene.
        const eventsPending = fetchEvents(eventFileRef.current);
        // Without names the overlay is uncoloured by class and the hover shows raw values only.
        const problems: SceneHandle["loadProblems"] = { names: null, maps: [] };
        const namesPending = loadNames().catch((e) => {
          problems.names = `Kachelnamen nicht geladen: ${errorText(e)}`;
          return new Map<number, TileBehavior>();
        });
        const ids = [...new Set(matrix.maps.filter((m): m is number => m !== null))];
        const loaded = new Map<number, Loaded>();
        const mapsPending = Promise.all(
          ids.map(async (id) => {
            try {
              loaded.set(id, { map: await getMap(id) });
            } catch (e) {
              problems.maps.push(`Karte ${id}: ${errorText(e)}`);
            }
          }),
        );
        const [names, fetchedEvents] = await Promise.all([namesPending, eventsPending, mapsPending]);
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
        setEventStatus(eventsNote(fetchedEvents));

        const w = matrix.width * MAP_SIZE;
        const d = matrix.height * MAP_SIZE;
        const span = Math.max(w, d);
        h.controls.target.set(w / 2, 0, d / 2);
        h.camera.position.set(w / 2, span * 0.9, d / 2 + span * 0.7);
        h.controls.update();
        h.render();
        h.refreshHover?.();
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

  // Undo/redo changed maps behind the scene's back: refetch every shown map and recolour.
  useEffect(() => {
    const h = handle.current;
    if (reloadToken === 0 || !h?.matrix) return;
    let cancelled = false;
    // Results are only applied to the matrix they were requested for.
    const matrixAtStart = h.matrix;
    (async () => {
      const ids = [...h.loaded.keys()];
      const [fresh, fetchedEvents] = await Promise.all([Promise.all(ids.map((id) => getMap(id))), fetchEvents(eventFileRef.current)]);
      if (cancelled || handle.current !== h || h.matrix !== matrixAtStart) return;
      setMarkers(h, fetchedEvents.events, showEventsRef.current);
      setEventStatus(eventsNote(fetchedEvents));
      const layer = h.buildings;
      fresh.forEach((map, i) => {
        const data = h.loaded.get(ids[i]);
        if (!data) return;
        // Only maps whose buildings changed are rebuilt (a matrix can show hundreds of them); an
        // unchanged one keeps its building objects, which the layer's slots hold: map data and
        // layer never point at different objects.
        const { buildings, changed } = reconcileBuildings(data.map.buildings, map.buildings);
        data.map = { ...map, buildings };
        recolourMap(h, ids[i]);
        if (changed) layer?.syncMap(matrixAtStart, ids[i], buildings);
      });
      dropMissingSelection(h, callbacks.current.onSelectBuilding);
      h.render();
      h.refreshHover?.();
      if (layer) await loadMissingBuildingModels(h, layer, areaRef.current, () => cancelled || handle.current !== h || h.buildings !== layer);
    })().catch((e) => {
      if (!cancelled) callbacks.current.onError(errorText(e));
    });
    return () => {
      cancelled = true;
    };
  }, [reloadToken]);

  // The selection (made by a click, the header list, ...) is outlined; the scene also drags it.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    h.selection = selectedMap === null || selectedBuilding === null ? null : { map: selectedMap, index: selectedBuilding };
    h.buildings?.select(h.selection);
    h.render();
  }, [selectedMap, selectedBuilding]);

  // A building of one map was edited elsewhere (inspector, add, delete): read that map's buildings
  // again and show them; a model that is new (or asked for again) loads in the background.
  useEffect(() => {
    const h = handle.current;
    const request = buildingRefresh;
    if (!request || !h?.matrix || !h.buildings) return;
    const layer = h.buildings;
    const matrixAtStart = h.matrix;
    let cancelled = false;
    const stale = () => cancelled || handle.current !== h || h.buildings !== layer;
    (async () => {
      const data = h.loaded.get(request.map);
      if (!data) return;
      const fresh = await getMap(request.map);
      if (stale() || h.matrix !== matrixAtStart) return;
      data.map.buildings = fresh.buildings;
      layer.syncMap(matrixAtStart, request.map, fresh.buildings);
      if (request.retryModel !== null) layer.retry(request.retryModel);
      dropMissingSelection(h, callbacks.current.onSelectBuilding);
      h.render();
      await loadMissingBuildingModels(h, layer, areaRef.current, stale);
    })().catch((e) => {
      if (!cancelled) callbacks.current.onError(errorText(e));
    });
    return () => {
      cancelled = true;
    };
  }, [buildingRefresh]);

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
    refreshEvents(h, request.file, showEventsRef.current, () => handle.current === h && eventFileRef.current === request.file, setEventStatus).catch((e) => callbacks.current.onError(e));
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
