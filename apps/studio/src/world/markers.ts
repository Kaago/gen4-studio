import * as THREE from "three";
import type { EventsDto, MatrixDto } from "../api";
import { MARKER_COLORS, markerPosition } from "./events";
import { cellYAtTile } from "./layout";
import type { EventData, EventKind } from "./events";
import { disposeTree } from "./sceneUtil";

/** Centre height of the flat event markers; the tall warp beacon stands on the ground. */
export const MARKER_Y = 0.3;
const BEACON_HEIGHT = 1.2;
/** Height of the flat plane that shows the area of a coordinate trigger (above the permission overlay). */
const AREA_Y = 0.08;

/** Height of each marker mesh above the ground of its cell (the flat marker and the beacon differ). */
const meshBase = new WeakMap<THREE.InstancedMesh, number>();

/** An event by class and index within the class (the identity the editor binds to). */
export interface MarkerRef {
  kind: EventKind;
  index: number;
}

/** The event markers of the shown header: one instanced mesh per class, plus a tall one for warps. */
export interface MarkerSet {
  group: THREE.Group;
  matrix: MatrixDto;
  /** Event class and data per instance, for every mesh in `group` that can be picked. */
  entries: Map<THREE.InstancedMesh, { kind: EventKind; data: EventData[] }>;
  /** The flat planes showing the `width x length` area of coordinate triggers (instance i belongs to `areaData[i]`). */
  area: THREE.InstancedMesh | null;
  areaData: Extract<EventData, { width: number }>[];
  /** The outline of the selected marker (null: none). */
  outline: THREE.LineSegments | null;
  selected: MarkerRef | null;
}

/** An event marker under a ray. */
export interface MarkerHit {
  kind: EventKind;
  data: EventData;
}

const sameRef = (a: MarkerRef | null, b: MarkerRef | null) => a !== null && b !== null && a.kind === b.kind && a.index === b.index;

/** The instance matrix of an area plane at height `y` covering `width x length` tiles from tile (x, z). */
function areaMatrix(x: number, y: number, z: number, width: number, length: number, into: THREE.Matrix4): THREE.Matrix4 {
  return into.compose(new THREE.Vector3(x + width / 2, y, z + length / 2), new THREE.Quaternion(), new THREE.Vector3(Math.max(width, 0), 1, Math.max(length, 0)));
}

/** Builds the marker meshes of `events`; events outside the matrix get no marker. */
export function buildMarkers(matrix: MatrixDto, events: EventsDto): MarkerSet {
  const group = new THREE.Group();
  const entries: MarkerSet["entries"] = new Map();
  const classes: { kind: EventKind; list: EventData[] }[] = [
    { kind: "warp", list: events.warps },
    { kind: "object", list: events.objects },
    { kind: "coord", list: events.coords },
    { kind: "bg", list: events.bg },
  ];
  const m4 = new THREE.Matrix4();
  let area: THREE.InstancedMesh | null = null;
  let areaData: MarkerSet["areaData"] = [];
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
      const base = g === 0 ? MARKER_Y : BEACON_HEIGHT / 2;
      meshBase.set(mesh, base);
      mesh.renderOrder = 11;
      mesh.frustumCulled = false;
      placed.forEach(({ data, at }, i) => {
        m4.setPosition(at.wx, base + cellYAtTile(matrix, data.x, data.z), at.wz);
        mesh.setMatrixAt(i, m4);
      });
      group.add(mesh);
      entries.set(mesh, { kind, data: placed.map((p) => p.data) });
    });
    if (kind === "coord") {
      // The area a trigger reacts to, as a flat translucent plane (not pickable: the marker is).
      areaData = placed.map((p) => p.data as MarkerSet["areaData"][number]);
      const plane = new THREE.PlaneGeometry(1, 1);
      plane.rotateX(-Math.PI / 2);
      const areaMaterial = new THREE.MeshBasicMaterial({ color: MARKER_COLORS.coord, depthTest: false, depthWrite: false, transparent: true, opacity: 0.3, side: THREE.DoubleSide });
      area = new THREE.InstancedMesh(plane, areaMaterial, areaData.length);
      area.renderOrder = 10.5;
      area.frustumCulled = false;
      areaData.forEach((c, i) => area?.setMatrixAt(i, areaMatrix(c.x, AREA_Y + cellYAtTile(matrix, c.x, c.z), c.z, c.width, c.length, m4)));
      group.add(area);
    }
  }
  return { group, matrix, entries, area, areaData, outline: null, selected: null };
}

/** Frees the GPU resources of a marker set (the caller takes its group out of the scene). */
export function disposeMarkers(set: MarkerSet) {
  disposeTree(set.group);
}

/**
 * The nearest event marker under the ray the raycaster is aimed along, or null (also while the
 * markers are hidden).
 */
export function pickMarker(set: MarkerSet | null, raycaster: THREE.Raycaster): MarkerHit | null {
  if (!set?.group.visible) return null;
  const hit = raycaster.intersectObjects([...set.entries.keys()], false)[0];
  if (!hit || !(hit.object instanceof THREE.InstancedMesh) || hit.instanceId === undefined) return null;
  const entry = set.entries.get(hit.object);
  const data = entry?.data[hit.instanceId];
  return entry && data ? { kind: entry.kind, data } : null;
}

/** Every instance (mesh and index) that draws event `ref`. */
function instancesOf(set: MarkerSet, ref: MarkerRef): { mesh: THREE.InstancedMesh; i: number; data: EventData }[] {
  const found: { mesh: THREE.InstancedMesh; i: number; data: EventData }[] = [];
  for (const [mesh, entry] of set.entries) {
    if (entry.kind !== ref.kind) continue;
    const i = entry.data.findIndex((d) => d.index === ref.index);
    const data = entry.data[i];
    if (data) found.push({ mesh, i, data });
  }
  return found;
}

/** Marks event `ref` (null: none) with a white outline; an event that has no marker (outside the matrix) selects nothing. */
export function selectMarker(set: MarkerSet, ref: MarkerRef | null) {
  if (set.outline) {
    set.group.remove(set.outline);
    set.outline.geometry.dispose();
    (set.outline.material as THREE.Material).dispose();
    set.outline = null;
  }
  set.selected = null;
  const found = ref ? instancesOf(set, ref)[0] : undefined;
  if (!ref || !found) return;
  const height = ref.kind === "warp" ? BEACON_HEIGHT : 0.4;
  const box = new THREE.BoxGeometry(0.95, height, 0.95);
  const outline = new THREE.LineSegments(new THREE.EdgesGeometry(box), new THREE.LineBasicMaterial({ color: "#ffffff", depthTest: false }));
  box.dispose();
  outline.renderOrder = 12;
  outline.frustumCulled = false;
  set.outline = outline;
  set.selected = ref;
  set.group.add(outline);
  placeOutline(set, found.data.x, found.data.z);
}

function placeOutline(set: MarkerSet, x: number, z: number) {
  if (!set.outline || !set.selected) return;
  const height = set.selected.kind === "warp" ? BEACON_HEIGHT : 0.4;
  set.outline.position.set(x + 0.5, (set.selected.kind === "warp" ? height / 2 : MARKER_Y) + cellYAtTile(set.matrix, x, z), z + 0.5);
}

/**
 * Shows event `ref` on tile (`x`, `z`) (absolute over the matrix), whatever its data says: a drag
 * in progress, or the committed position again. Its height follows the cell of that tile; its
 * area plane and outline follow.
 */
export function moveMarker(set: MarkerSet, ref: MarkerRef, tile: { x: number; z: number }) {
  const m4 = new THREE.Matrix4();
  for (const { mesh, i } of instancesOf(set, ref)) {
    mesh.getMatrixAt(i, m4);
    m4.setPosition(tile.x + 0.5, (meshBase.get(mesh) ?? MARKER_Y) + cellYAtTile(set.matrix, tile.x, tile.z), tile.z + 0.5);
    mesh.setMatrixAt(i, m4);
    mesh.instanceMatrix.needsUpdate = true;
    // Raycasts reject by the cached bounds of all instances, which must grow or shrink with the move.
    mesh.computeBoundingSphere();
  }
  if (ref.kind === "coord" && set.area) {
    const i = set.areaData.findIndex((c) => c.index === ref.index);
    const c = set.areaData[i];
    if (c) {
      set.area.setMatrixAt(i, areaMatrix(tile.x, AREA_Y + cellYAtTile(set.matrix, tile.x, tile.z), tile.z, c.width, c.length, m4));
      set.area.instanceMatrix.needsUpdate = true;
    }
  }
  if (sameRef(set.selected, ref)) placeOutline(set, tile.x, tile.z);
}
