import { errorText, setPermission } from "../../api";
import { MAP_SIZE } from "../layout";
import { BLOCKED, isClick } from "../paint";
import { recolourMap } from "./content";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";

/**
 * Painting and selecting: a click (pointer barely moved) selects the event marker or building
 * under the cursor, else sets collision on the tile under it; dragging still orbits. Starts
 * listening on the canvas; the returned function stops it.
 */
export function attachCanvasClicks(h: SceneHost, picker: Picker, refs: SceneRefs): () => void {
  const canvas = h.canvas;
  let down: { x: number; y: number } | null = null;
  const onDown = (e: PointerEvent) => {
    down = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
  };
  const onUp = (e: PointerEvent) => {
    const start = down;
    down = null;
    if (!start || e.button !== 0 || !isClick(start, { x: e.clientX, y: e.clientY })) return;
    const cb = refs.callbacks.current;
    // A marker under the click is selected instead of anything below it.
    const marker = picker.markerAt(e.clientX, e.clientY);
    if (marker && h.events) {
      cb.onSelectEvent({ file: h.events.event_file_id, kind: marker.kind, index: marker.data.index });
      return;
    }
    // A building under the click is selected instead of painting the tile below it.
    if (h.buildings) {
      picker.aim(e.clientX, e.clientY);
      const hit = h.buildings.pick(picker.raycaster);
      if (hit) {
        cb.onSelectBuilding(hit.ref);
        return;
      }
    }
    if (!cb.paintEnabled || !h.overlay?.visible) return;
    const picked = picker.pickTile(e.clientX, e.clientY);
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
        refs.callbacks.current.onEdited();
      },
      (err) => refs.callbacks.current.onError(errorText(err)),
    );
  };
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointerup", onUp);
  return () => {
    canvas.removeEventListener("pointerdown", onDown);
    canvas.removeEventListener("pointerup", onUp);
  };
}
