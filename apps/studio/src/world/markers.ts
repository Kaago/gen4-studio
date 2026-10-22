import * as THREE from "three";
import type { EventsDto, MatrixDto } from "../api";
import { MARKER_COLORS, markerPosition } from "./events";
import type { EventData, EventKind } from "./events";
import { disposeTree } from "./sceneUtil";

/** Centre height of the flat event markers; the tall warp beacon stands on the ground. */
const MARKER_Y = 0.3;
const BEACON_HEIGHT = 1.2;

/** The event markers of the shown header: one instanced mesh per class, plus a tall one for warps. */
export interface MarkerSet {
  group: THREE.Group;
  /** Event class and data per instance, for every mesh in `group`. */
  entries: Map<THREE.InstancedMesh, { kind: EventKind; data: EventData[] }>;
}

/** An event marker under a ray. */
export interface MarkerHit {
  kind: EventKind;
  data: EventData;
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
