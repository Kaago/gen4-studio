import { errorText, setPermission } from "../../api";
import { MAP_SIZE } from "../layout";
import { BLOCKED, isClick } from "../paint";
import { recolourMap } from "./content";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";

/**
 * What a click (pointer barely moved) does, by tool: `select` selects the event marker, else the building under the cursor,
 * else the matrix cell of the loaded map under it, and on empty ground does nothing (it never paints); `paint` selects a
 * marker or building the same way (never a cell) and else sets collision on the tile under the cursor; the adding tools
 * (figure, door, trigger, sign, building) report the tile under it and select nothing.
 * Dragging still orbits. Starts listening on the canvas; the returned function stops it.
 */
export function attachCanvasClicks(h: SceneHost, picker: Picker, refs: SceneRefs): () => void {
  const canvas = h.canvas;
  let down: { x: number; y: number } | null = null;
  const onDown = (e: PointerEvent) => {
    down = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
  };
  /** Selects the marker, else the building under the click; true when it did. */
  const select = (e: PointerEvent): boolean => {
    const cb = refs.callbacks.current;
    // A marker under the click is selected instead of anything below it.
    const marker = picker.markerAt(e.clientX, e.clientY);
    if (marker && h.events) {
      cb.onSelectEvent({ file: h.events.event_file_id, kind: marker.kind, index: marker.data.index });
      return true;
    }
    // A building under the click is selected instead of painting the tile below it.
    if (h.buildings) {
      picker.aim(e.clientX, e.clientY);
      const hit = h.buildings.pick(picker.raycaster);
      if (hit) {
        cb.onSelectBuilding(hit.ref);
        return true;
      }
    }
    return false;
  };
  const paint = (e: PointerEvent) => {
    if (!refs.callbacks.current.paintEnabled || !h.overlay?.visible) return;
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
  const onUp = (e: PointerEvent) => {
    const start = down;
    down = null;
    if (!start || e.button !== 0 || !isClick(start, { x: e.clientX, y: e.clientY })) return;
    const cb = refs.callbacks.current;
    if (cb.tool === "select") {
      if (select(e)) return;
      const picked = picker.pickTile(e.clientX, e.clientY);
      if (picked) cb.onSelectCell({ col: picked.col, row: picked.row });
    } else if (cb.tool === "paint") {
      if (!select(e)) paint(e);
    } else if (cb.paintEnabled) {
      const picked = picker.pickTile(e.clientX, e.clientY);
      if (picked) cb.onTileClick({ mapId: picked.mapId, x: picked.x, y: picked.y, absX: picked.col * MAP_SIZE + picked.x, absZ: picked.row * MAP_SIZE + picked.y });
    }
  };
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointerup", onUp);
  return () => {
    canvas.removeEventListener("pointerdown", onDown);
    canvas.removeEventListener("pointerup", onUp);
  };
}
