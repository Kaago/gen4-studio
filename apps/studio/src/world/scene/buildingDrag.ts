import * as THREE from "three";
import { errorText, updateBuilding } from "../../api";
import type { BuildingDto } from "../../api";
import type { BuildingRef } from "../buildingLayer";
import { sameBuilding, settleBuilding } from "../buildings";
import { DragController } from "../dragController";
import type { DragSurface } from "../dragController";
import { MAP_SIZE } from "../layout";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";

/** What a building drag holds on to: the building as it was at the press and the centre of its map cell (its positions are relative to it). */
export interface BuildingDrag {
  ref: BuildingRef;
  building: BuildingDto;
  centre: { x: number; z: number };
}

/**
 * Dragging the selected building moves it on the ground plane (buildings snap to half tiles). A
 * press only starts a drag on the selected building itself, when painting is enabled and no event
 * marker is over it; one `update_building` is written when the pointer is released.
 */
export function createBuildingDrag(h: SceneHost, picker: Picker, surface: DragSurface, refs: SceneRefs): DragController<BuildingDrag, THREE.Plane> {
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
        refs.callbacks.current.onBuildingMoved(ref.map);
      },
      (err) => {
        const data = h.loaded.get(ref.map);
        const current = data?.map.buildings[ref.index];
        if (current && sameBuilding(current, building)) {
          h.buildings?.show(ref, building);
          h.render();
        }
        refs.callbacks.current.onError(errorText(err));
      },
    );
    h.refreshHover?.();
  };
  return new DragController<BuildingDrag, THREE.Plane>(surface, 0.5, {
    begin: (e) => {
      const selected = h.selection;
      if (!selected || !h.buildings || !refs.callbacks.current.paintEnabled) return null;
      // A marker over the building is what a press there means (a click selects it).
      if (picker.markerAt(e.clientX, e.clientY)) return null;
      picker.aim(e.clientX, e.clientY);
      const hit = h.buildings.pick(picker.raycaster);
      if (!hit || hit.ref.map !== selected.map || hit.ref.index !== selected.index) return null;
      const centre = { x: hit.cell.x + MAP_SIZE / 2, z: hit.cell.z + MAP_SIZE / 2 };
      return {
        subject: { ref: hit.ref, building: hit.building, centre },
        plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -hit.building.y),
        origin: { x: centre.x + hit.building.x, z: centre.z + hit.building.z },
      };
    },
    pointOn: picker.planePoint,
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
}
