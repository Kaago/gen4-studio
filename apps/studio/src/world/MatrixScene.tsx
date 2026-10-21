import { memo, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";
import { errorText, getEvents, getMap, getMapModelGlb, setPermission, tileBehaviors } from "../api";
import type { Message } from "../Toolbar";
import { headerLabel } from "../workbench/headers";
import type { EventsDto, HeaderSummary, MapDto, MatrixDto, TileBehavior, WarpDto } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { permissionColor } from "./colors";
import { MARKER_COLORS, markerPosition } from "./events";
import type { EventData, EventKind } from "./events";
import { describeTile } from "./hover";
import { MODEL_CONCURRENCY, limitConcurrency } from "./load";
import { MAP_SIZE, buildingBox, cellOrigin } from "./layout";
import { BLOCKED, isClick, matrixCellAt, pickCell } from "./paint";

/** apicula GLB units per tile: a 32-tile map model spans 512 units (measured on vanilla Pt/HGSS maps). */
const MODEL_UNITS_PER_TILE = 16;
const MODEL_SCALE = 1 / MODEL_UNITS_PER_TILE;

interface Loaded {
  map: MapDto;
}

/** Height of the overlay plane; clicks are intersected with the same plane. */
const OVERLAY_Y = 0.05;

/** Centre height of the flat event markers; the tall warp beacon stands on the ground. */
const MARKER_Y = 0.3;
const BEACON_HEIGHT = 1.2;

/** The event markers of the shown header: one instanced mesh per class, plus a tall one for warps. */
interface MarkerSet {
  group: THREE.Group;
  /** Event class and data per instance, for every mesh in `group`. */
  entries: Map<THREE.InstancedMesh, { kind: EventKind; data: EventData[] }>;
}

interface SceneHandle {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  content: THREE.Group;
  overlay: THREE.Object3D | null;
  render: () => void;
  /** Matrix and loaded maps behind the current content (null until a matrix is loaded). */
  matrix: MatrixDto | null;
  loaded: Map<number, Loaded>;
  tiles: THREE.InstancedMesh | null;
  /** Map id to the first overlay instance of each matrix cell showing that map. */
  bases: Map<number, number[]>;
  /** Tile behaviour names by kind byte (empty until loaded). */
  names: Map<number, TileBehavior>;
  markers: MarkerSet | null;
  /** Re-resolves the tile under the pointer on the next frame (set by the setup effect). */
  refreshHover: (() => void) | null;
}

const colorCache = new Map<string, THREE.Color>();
function overlayColor(kind: number, collision: number, names: Map<number, TileBehavior>): THREE.Color {
  const hex = permissionColor(kind, collision, names.get(kind)?.class ?? null);
  let color = colorCache.get(hex);
  if (!color) colorCache.set(hex, (color = new THREE.Color(hex)));
  return color;
}

function disposeTree(root: THREE.Object3D) {
  root.traverse((obj) => {
    const mesh = obj as THREE.Mesh;
    mesh.geometry?.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
    for (const material of materials) {
      for (const value of Object.values(material)) {
        if (value instanceof THREE.Texture) value.dispose();
      }
      material.dispose();
    }
    (obj as THREE.InstancedMesh).dispose?.();
  });
}

function parseGlb(buffer: ArrayBuffer): Promise<THREE.Object3D> {
  return new Promise((resolve, reject) => {
    new GLTFLoader().parse(buffer, "", (gltf) => resolve(gltf.scene), reject);
  });
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
    const instance = cloneSkinned(model);
    instance.traverse((obj) => {
      const mesh = obj as THREE.SkinnedMesh;
      if (mesh.isSkinnedMesh) mesh.frustumCulled = false;
      if (mesh.isMesh) {
        const material = (Array.isArray(mesh.material) ? mesh.material : [mesh.material]) as THREE.Material[];
        for (const m of material) m.side = THREE.DoubleSide;
      }
    });
    // The model is centred on the map centre.
    instance.scale.setScalar(MODEL_SCALE);
    instance.position.set(origin.x + MAP_SIZE / 2, 0, origin.z + MAP_SIZE / 2);
    h.content.add(instance);
    used = true;
  });
  // The clones in `content` are disposed with it; a model without a cell would leak.
  if (!used) disposeTree(model);
}

/** Builds the overlay and building objects once all maps are known; models are added later. */
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

  const boxes: { cx: number; cy: number; cz: number; sx: number; sy: number; sz: number }[] = [];
  for (const { col, row, data } of cells) {
    const origin = cellOrigin(col, row);
    for (const b of data.map.buildings) boxes.push(buildingBox(b, origin));
  }
  const boxMesh = new THREE.InstancedMesh(
    new THREE.BoxGeometry(1, 1, 1),
    new THREE.MeshBasicMaterial({ color: "#f59e0b", wireframe: true }),
    boxes.length,
  );
  boxMesh.frustumCulled = false;
  const q = new THREE.Quaternion();
  boxes.forEach((b, i) => {
    m4.compose(new THREE.Vector3(b.cx, b.cy, b.cz), q, new THREE.Vector3(b.sx, b.sy, b.sz));
    boxMesh.setMatrixAt(i, m4);
  });
  content.add(boxMesh);
  return { content, overlay, tiles, bases };
}

/** Builds the marker meshes of `events`; events outside the matrix get no marker. */
function buildMarkers(matrix: MatrixDto, events: EventsDto): MarkerSet {
  const group = new THREE.Group();
  const entries: MarkerSet["entries"] = new Map();
  const classes: { kind: EventKind; list: EventData[] }[] = [
    { kind: "warp", list: events.warps },
    { kind: "object", list: events.objects },
    { kind: "coord", list: events.coords },
    { kind: "bg", list: events.bg },
  ];
  const m4 = new THREE.Matrix4();
  for (const { kind, list } of classes) {
    const placed = list.flatMap((data) => {
      const at = markerPosition(data.x, data.z, matrix);
      return at ? [{ data, at }] : [];
    });
    if (placed.length === 0) continue;
    const geometries = [new THREE.BoxGeometry(0.8, 0.4, 0.8)];
    if (kind === "warp") geometries.push(new THREE.BoxGeometry(0.8, BEACON_HEIGHT, 0.8));
    const material = new THREE.MeshBasicMaterial({ color: MARKER_COLORS[kind], depthTest: false, transparent: true, opacity: 0.9 });
    geometries.forEach((geometry, g) => {
      const mesh = new THREE.InstancedMesh(geometry, material, placed.length);
      mesh.renderOrder = 11;
      mesh.frustumCulled = false;
      placed.forEach(({ at }, i) => {
        m4.setPosition(at.wx, g === 0 ? MARKER_Y : BEACON_HEIGHT / 2, at.wz);
        mesh.setMatrixAt(i, m4);
      });
      group.add(mesh);
      entries.set(mesh, { kind, data: placed.map((p) => p.data) });
    });
  }
  return { group, entries };
}

/** Replaces the scene's markers (`events` null: show none). */
function setMarkers(h: SceneHandle, events: EventsDto | null, visible: boolean) {
  if (h.markers) {
    h.scene.remove(h.markers.group);
    disposeTree(h.markers.group);
    h.markers = null;
  }
  if (!h.matrix || !events) return;
  h.markers = buildMarkers(h.matrix, events);
  h.markers.group.visible = visible;
  h.scene.add(h.markers.group);
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
  /** Area data of the selected header; selects the map texture set for every map of the matrix. */
  areaDataId: number;
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
}

function MatrixScene({
  matrix,
  areaDataId,
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
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const handle = useRef<SceneHandle | null>(null);
  const showRef = useRef(showPermissions);
  // Read when a matrix loads, not a reload trigger of its own: the header choice and its
  // matrix arrive in separate state updates.
  const areaRef = useRef(areaDataId);
  areaRef.current = areaDataId;
  const showEventsRef = useRef(showEvents);
  // Like the area data, the event file belongs to the header and is read when a matrix loads.
  const eventFileRef = useRef(eventFileId);
  eventFileRef.current = eventFileId;
  const headersRef = useRef(headers);
  headersRef.current = headers;
  const [eventStatus, setEventStatus] = useState<string | null>(null);
  const callbacks = useRef({ paintEnabled, onEdited, onError, onInspect, onStatus });
  callbacks.current = { paintEnabled, onEdited, onError, onInspect, onStatus };

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
      loaded: new Map(),
      tiles: null,
      bases: new Map(),
      names: new Map(),
      markers: null,
      refreshHover: null,
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
    const pickMarker = (clientX: number, clientY: number) => {
      const set = h.markers;
      if (!set?.group.visible) return null;
      aim(clientX, clientY);
      const hit = raycaster.intersectObjects([...set.entries.keys()], false)[0];
      if (!hit || !(hit.object instanceof THREE.InstancedMesh) || hit.instanceId === undefined) return null;
      const entry = set.entries.get(hit.object);
      const data = entry?.data[hit.instanceId];
      return entry && data ? { kind: entry.kind, data } : null;
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
      // A marker under the pointer wins over the tile below it.
      const marker = pointer ? pickMarker(pointer.x, pointer.y) : null;
      if (marker) {
        const key = `event:${marker.kind}:${marker.data.index}`;
        if (key === shown) return;
        shown = key;
        const destId = marker.kind === "warp" ? (marker.data as WarpDto).dest_header_id : null;
        const destHeader = destId === null ? undefined : headersRef.current.find((x) => x.id === destId);
        const destName = destHeader && headerLabel(destHeader);
        callbacks.current.onInspect({ kind: "event", eventKind: marker.kind, index: marker.data.index, data: marker.data, destName });
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

    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
    };
    const onUp = (e: PointerEvent) => {
      const start = down;
      down = null;
      if (!start || e.button !== 0 || !isClick(start, { x: e.clientX, y: e.clientY })) return;
      const cb = callbacks.current;
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
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerleave", onLeave);
      cancelAnimationFrame(frame);
      callbacks.current.onInspect({ kind: "none" });
      callbacks.current.onStatus(null);
      const current = handle.current;
      handle.current = null;
      if (current?.markers) disposeTree(current.markers.group);
      observer.disconnect();
      controls.removeEventListener("change", render);
      controls.dispose();
      disposeTree(current ? current.content : content);
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, []);

  // (Re)load the matrix content: maps first (overlay and markers appear as soon as they are
  // there), then the models, a few at a time, each hung into the scene as it arrives.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    let cancelled = false;
    const area = areaRef.current;
    /** Status updates of a superseded load are dropped. */
    const report = (message: Message | null) => {
      if (!cancelled) callbacks.current.onStatus(message);
    };
    report({ kind: "info", text: "Lade Karten ..." });
    setEventStatus(null);
    // Fetched next to the maps; a failure only costs the markers, never the scene.
    const eventsPending = fetchEvents(eventFileRef.current);
    // Without names the overlay is uncoloured by class and the hover shows raw values only.
    let namesError: string | null = null;
    const namesPending = loadNames().catch((e) => {
      namesError = `Kachelnamen nicht geladen: ${errorText(e)}`;
      return new Map<number, TileBehavior>();
    });
    // Frames bursts of arriving models into at most one render per animation frame.
    let renderQueued = false;
    const queueRender = () => {
      if (renderQueued) return;
      renderQueued = true;
      requestAnimationFrame(() => {
        renderQueued = false;
        if (handle.current === h) h.render();
      });
    };

    (async () => {
      const ids = [...new Set(matrix.maps.filter((m): m is number => m !== null))];
      const loaded = new Map<number, Loaded>();
      const mapErrors: string[] = [];
      const mapsPending = Promise.all(
        ids.map(async (id) => {
          try {
            loaded.set(id, { map: await getMap(id) });
          } catch (e) {
            mapErrors.push(`Karte ${id}: ${errorText(e)}`);
          }
        }),
      );
      const [names, fetchedEvents] = await Promise.all([namesPending, eventsPending, mapsPending]);
      if (cancelled || handle.current !== h) return;
      const { content, overlay, tiles, bases } = buildContent(matrix, loaded, names);
      h.scene.remove(h.content);
      disposeTree(h.content);
      h.content = content;
      h.overlay = overlay;
      h.matrix = matrix;
      h.loaded = loaded;
      h.tiles = tiles;
      h.bases = bases;
      h.names = names;
      overlay.visible = showRef.current;
      h.scene.add(content);
      setMarkers(h, fetchedEvents.events, showEventsRef.current);
      setEventStatus(fetchedEvents.error);

      const w = matrix.width * MAP_SIZE;
      const d = matrix.height * MAP_SIZE;
      const span = Math.max(w, d);
      h.controls.target.set(w / 2, 0, d / 2);
      h.camera.position.set(w / 2, span * 0.9, d / 2 + span * 0.7);
      h.controls.update();
      h.render();
      h.refreshHover?.();

      // Models: a task that has not started when the load is superseded never starts; a started
      // one finishes, but its scene is disposed instead of added.
      const modelIds = [...loaded.keys()];
      const tasks = modelIds.map((id) => async () => {
        if (cancelled || handle.current !== h) return;
        const model = await getMapModelGlb(id, area).then(parseGlb);
        if (cancelled || handle.current !== h) {
          disposeTree(model);
          return;
        }
        addModel(h, matrix, id, model);
        queueRender();
      });
      let lastReport = 0;
      const progress = (done: number, total: number) => {
        // The status bar re-renders the whole workbench: at most ten updates per second.
        const now = performance.now();
        if (done < total && now - lastReport < 100) return;
        lastReport = now;
        report({ kind: "info", text: `Modelle ${done}/${total}` });
      };
      progress(0, modelIds.length);
      const results = await limitConcurrency(tasks, MODEL_CONCURRENCY, progress);
      const modelErrors = results.flatMap((r, i) => (r.status === "rejected" ? [`Map ${modelIds[i]}: ${errorText(r.reason)}`] : []));
      const problems = [
        namesError,
        mapErrors.length ? `${mapErrors.length} Karte(n) nicht geladen: ${mapErrors[0]}` : null,
        modelErrors.length ? `${modelErrors.length} Modell(e) nicht geladen: ${modelErrors[0]}` : null,
      ].filter((p): p is string => p !== null);
      report(problems.length ? { kind: "error", text: problems.join(" · ") } : null);
    })().catch((e) => report({ kind: "error", text: errorText(e) }));

    return () => {
      cancelled = true;
    };
  }, [matrix]);

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
      setEventStatus(fetchedEvents.error);
      fresh.forEach((map, i) => {
        const data = h.loaded.get(ids[i]);
        if (!data) return;
        data.map = map;
        recolourMap(h, ids[i]);
      });
      h.render();
      h.refreshHover?.();
    })().catch((e) => {
      if (!cancelled) callbacks.current.onError(errorText(e));
    });
    return () => {
      cancelled = true;
    };
  }, [reloadToken]);

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
