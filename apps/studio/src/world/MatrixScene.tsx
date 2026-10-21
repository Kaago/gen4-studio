import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";
import { getMap, getMapModelGlb, setPermission } from "../api";
import type { MapDto, MatrixDto } from "../api";
import { permissionColor } from "./colors";
import { MAP_SIZE, buildingBox, cellOrigin } from "./layout";
import { BLOCKED, isClick, matrixCellAt, pickCell } from "./paint";

/** apicula GLB units per tile: a 32-tile map model spans 512 units (measured on vanilla Pt/HGSS maps). */
const MODEL_UNITS_PER_TILE = 16;
const MODEL_SCALE = 1 / MODEL_UNITS_PER_TILE;

interface Loaded {
  map: MapDto;
  model: THREE.Object3D | null;
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
  loaded: Map<number, Loaded>;
  tiles: THREE.InstancedMesh | null;
  /** Map id to the first overlay instance of each matrix cell showing that map. */
  bases: Map<number, number[]>;
}

const colorCache = new Map<string, THREE.Color>();
function overlayColor(kind: number, collision: number): THREE.Color {
  const hex = permissionColor(kind, collision);
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

async function loadMap(index: number): Promise<Loaded> {
  const map = await getMap(index);
  const model = await getMapModelGlb(index).then(parseGlb);
  return { map, model };
}

/** Builds the overlay and building objects once all maps are known. */
function buildContent(matrix: MatrixDto, loaded: Map<number, Loaded>) {
  const content = new THREE.Group();
  const cells: { col: number; row: number; data: Loaded }[] = [];
  matrix.maps.forEach((id, i) => {
    const data = id === null ? undefined : loaded.get(id);
    if (data) cells.push({ col: i % matrix.width, row: Math.floor(i / matrix.width), data });
  });

  for (const { col, row, data } of cells) {
    if (!data.model) continue;
    const origin = cellOrigin(col, row);
    const instance = cloneSkinned(data.model);
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
    content.add(instance);
  }

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
      tiles.setColorAt(n, overlayColor(kind, collision));
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

/** Recolours every overlay instance of `mapId` from the map's current permissions. */
function recolourMap(h: SceneHandle, mapId: number) {
  const data = h.loaded.get(mapId);
  const bases = h.bases.get(mapId);
  if (!data || !bases || !h.tiles) return;
  for (const base of bases) {
    data.map.permissions.forEach(([kind, collision], i) => {
      h.tiles?.setColorAt(base + i, overlayColor(kind, collision));
    });
  }
  if (h.tiles.instanceColor) h.tiles.instanceColor.needsUpdate = true;
}

interface Props {
  matrix: MatrixDto;
  showPermissions: boolean;
  /** Painting is ignored while false (e.g. during a build). */
  paintEnabled: boolean;
  /** Increment to reload all maps of the matrix from the backend and recolour (after undo/redo). */
  reloadToken: number;
  /** Called after a tile was changed in the backend. */
  onEdited: () => void;
  onError: (message: string) => void;
}

export default function MatrixScene({ matrix, showPermissions, paintEnabled, reloadToken, onEdited, onError }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const handle = useRef<SceneHandle | null>(null);
  const showRef = useRef(showPermissions);
  const [status, setStatus] = useState<string | null>(null);
  const callbacks = useRef({ paintEnabled, onEdited, onError });
  callbacks.current = { paintEnabled, onEdited, onError };

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
    };
    handle.current = h;

    // Painting: a click (pointer barely moved) sets collision on the tile under the cursor;
    // dragging still orbits.
    const canvas = renderer.domElement;
    const raycaster = new THREE.Raycaster();
    const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -OVERLAY_Y);
    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
    };
    const onUp = (e: PointerEvent) => {
      const start = down;
      down = null;
      if (!start || e.button !== 0 || !isClick(start, { x: e.clientX, y: e.clientY })) return;
      const cb = callbacks.current;
      if (!cb.paintEnabled || !h.overlay?.visible || !h.matrix) return;
      const rect = canvas.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1,
      );
      raycaster.setFromCamera(ndc, camera);
      const hit = raycaster.ray.intersectPlane(plane, new THREE.Vector3());
      if (!hit) return;
      const m = h.matrix;
      const at = matrixCellAt(hit, m.width, m.height);
      if (!at) return;
      const mapId = m.maps[at.row * m.width + at.col];
      const tile = mapId === null || mapId === undefined ? null : pickCell(hit, cellOrigin(at.col, at.row));
      if (mapId === null || mapId === undefined || !tile) return;
      const data = h.loaded.get(mapId);
      if (!data) return;
      const i = tile.y * MAP_SIZE + tile.x;
      const kind = data.map.permissions[i]?.[0] ?? 0;
      const collision = e.shiftKey ? 0x00 : BLOCKED;
      setPermission(mapId, tile.x, tile.y, kind, collision).then(
        () => {
          data.map.permissions[i] = [kind, collision];
          // The matrix may have been swapped while the command was in flight.
          if (h.loaded.get(mapId) === data) recolourMap(h, mapId);
          h.render();
          callbacks.current.onEdited();
        },
        (err) => callbacks.current.onError(String(err)),
      );
    };
    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointerup", onUp);

    return () => {
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointerup", onUp);
      const current = handle.current;
      handle.current = null;
      observer.disconnect();
      controls.removeEventListener("change", render);
      controls.dispose();
      disposeTree(current ? current.content : content);
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, []);

  // (Re)load the matrix content.
  useEffect(() => {
    const h = handle.current;
    if (!h) return;
    let cancelled = false;
    setStatus("Lade Modelle ...");

    (async () => {
      const ids = [...new Set(matrix.maps.filter((m): m is number => m !== null))];
      const loaded = new Map<number, Loaded>();
      const errors: string[] = [];
      await Promise.all(
        ids.map(async (id) => {
          try {
            loaded.set(id, await loadMap(id));
          } catch (e) {
            errors.push(`Map ${id}: ${String(e)}`);
            // Keep the map's permissions and buildings even when only the model failed.
            try {
              loaded.set(id, { map: await getMap(id), model: null });
            } catch {
              /* reported above */
            }
          }
        }),
      );
      if (cancelled || handle.current !== h) {
        for (const l of loaded.values()) if (l.model) disposeTree(l.model);
        return;
      }
      const { content, overlay, tiles, bases } = buildContent(matrix, loaded);
      // The clones in `content` share geometry and materials with the loaded models, so
      // disposing `content` later releases everything.
      h.scene.remove(h.content);
      disposeTree(h.content);
      h.content = content;
      h.overlay = overlay;
      h.matrix = matrix;
      h.loaded = loaded;
      h.tiles = tiles;
      h.bases = bases;
      overlay.visible = showRef.current;
      h.scene.add(content);

      const w = matrix.width * MAP_SIZE;
      const d = matrix.height * MAP_SIZE;
      const span = Math.max(w, d);
      h.controls.target.set(w / 2, 0, d / 2);
      h.camera.position.set(w / 2, span * 0.9, d / 2 + span * 0.7);
      h.controls.update();
      h.render();
      setStatus(errors.length ? `${errors.length} Modell(e) nicht geladen: ${errors[0]}` : null);
    })().catch((e) => {
      if (!cancelled) setStatus(String(e));
    });

    return () => {
      cancelled = true;
    };
  }, [matrix]);

  // Undo/redo changed maps behind the scene's back: refetch every shown map and recolour.
  useEffect(() => {
    const h = handle.current;
    if (reloadToken === 0 || !h?.matrix) return;
    let cancelled = false;
    (async () => {
      const ids = [...h.loaded.keys()];
      const fresh = await Promise.all(ids.map((id) => getMap(id)));
      if (cancelled || handle.current !== h) return;
      fresh.forEach((map, i) => {
        const data = h.loaded.get(ids[i]);
        if (!data) return;
        data.map = map;
        recolourMap(h, ids[i]);
      });
      h.render();
    })().catch((e) => {
      if (!cancelled) callbacks.current.onError(String(e));
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

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      <div ref={host} style={{ position: "absolute", inset: 0 }} />
      {status && (
        <div style={{ position: "absolute", left: 8, bottom: 8, color: "#e5e7eb", font: "12px system-ui", maxWidth: "70%" }}>
          {status}
        </div>
      )}
    </div>
  );
}
