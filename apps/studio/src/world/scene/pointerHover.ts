import { MAP_SIZE } from "../layout";
import type { WarpDto } from "../../api";
import { describeTile } from "../hover";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";

/** The hover part of the scene: what is under the pointer is reported through `onInspect` (the status line names it). */
export interface PointerHover {
  /** Starts listening for the pointer on the canvas; the returned function stops it (and drops a queued frame). */
  attach(): () => void;
  /** Re-resolves the thing under the pointer on the next frame, even when it looks the same (it may have changed). */
  refresh(): void;
}

/**
 * Hover: the latest pointer position is resolved at most once per animation frame. A marker under
 * the pointer wins over the tile below it. While `busy()` (something is dragged) the hover keeps
 * what it reported; the end of the drag looks again (through `refresh`).
 */
export function createHover(h: SceneHost, picker: Picker, refs: SceneRefs, busy: () => boolean): PointerHover {
  let pointer: { x: number; y: number } | null = null;
  let frame = 0;
  let shown = "";
  const showHover = () => {
    frame = 0;
    // While something is dragged the reported hover stays; the end of the drag looks again.
    if (busy()) return;
    // A marker under the pointer wins over the tile below it.
    const marker = pointer ? picker.markerAt(pointer.x, pointer.y) : null;
    if (marker) {
      const key = `event:${marker.kind}:${marker.data.index}`;
      if (key === shown) return;
      shown = key;
      const destName = marker.kind === "warp" ? refs.destName.current((marker.data as WarpDto).dest_header_id) : undefined;
      refs.callbacks.current.onInspect({ kind: "eventHover", eventFile: h.events?.event_file_id ?? 0, eventKind: marker.kind, index: marker.data.index, data: marker.data, destName });
      return;
    }
    const picked = pointer ? picker.pickTile(pointer.x, pointer.y) : null;
    if (!picked) {
      if (shown !== "") refs.callbacks.current.onInspect({ kind: "none" });
      shown = "";
      return;
    }
    const cell = picked.data.map.permissions[picked.y * MAP_SIZE + picked.x] ?? [0, 0];
    // The collision byte is part of the key so a painted tile refreshes while the pointer rests on it.
    const key = `${picked.mapId}:${picked.col}:${picked.row}:${picked.x}:${picked.y}:${cell[0]}:${cell[1]}`;
    if (key === shown) return;
    shown = key;
    refs.callbacks.current.onInspect({
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
  return {
    refresh: () => {
      shown = "";
      scheduleHover();
    },
    attach: () => {
      const canvas = h.canvas;
      canvas.addEventListener("pointermove", onMove);
      canvas.addEventListener("pointerleave", onLeave);
      return () => {
        canvas.removeEventListener("pointermove", onMove);
        canvas.removeEventListener("pointerleave", onLeave);
        cancelAnimationFrame(frame);
      };
    },
  };
}
