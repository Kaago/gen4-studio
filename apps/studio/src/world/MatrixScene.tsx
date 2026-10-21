import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";
import { getMap, getMapModelGlb } from "../api";
import type { MapDto, MatrixDto } from "../api";
import { permissionColor } from "./colors";
import { MAP_SIZE, buildingBox, cellOrigin } from "./layout";

/** apicula GLB units per tile: a 32-tile map model spans 512 units (measured on vanilla Pt/HGSS maps). */
const MODEL_UNITS_PER_TILE = 16;
const MODEL_SCALE = 1 / MODEL_UNITS_PER_TILE;

interface Loaded {
  map: MapDto;
  model: THREE.Object3D | null;
}

interface SceneHandle {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  content: THREE.Group;
  overlay: THREE.Object3D | null;
  render: () => void;
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
  const colors = new Map<string, THREE.Color>();
  const m4 = new THREE.Matrix4();
  let n = 0;
  for (const { col, row, data } of cells) {
    const origin = cellOrigin(col, row);
    for (let i = 0; i < perCell; i++) {
      const [kind, collision] = data.map.permissions[i] ?? [0, 0];
      const hex = permissionColor(kind, collision);
      let color = colors.get(hex);
      if (!color) colors.set(hex, (color = new THREE.Color(hex)));
      m4.setPosition(origin.x + (i % MAP_SIZE) + 0.5, 0.05, origin.z + Math.floor(i / MAP_SIZE) + 0.5);
      tiles.setMatrixAt(n, m4);
      tiles.setColorAt(n, color);
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
  return { content, overlay };
}

export default function MatrixScene({ matrix, showPermissions }: { matrix: MatrixDto; showPermissions: boolean }) {
  const host = useRef<HTMLDivElement>(null);
  const handle = useRef<SceneHandle | null>(null);
  const showRef = useRef(showPermissions);
  const [status, setStatus] = useState<string | null>(null);

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
    handle.current = { scene, camera, controls, content, overlay: null, render };

    return () => {
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
      const { content, overlay } = buildContent(matrix, loaded);
      // The clones in `content` share geometry and materials with the loaded models, so
      // disposing `content` later releases everything.
      h.scene.remove(h.content);
      disposeTree(h.content);
      h.content = content;
      h.overlay = overlay;
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
