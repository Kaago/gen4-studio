import * as THREE from "three";
import { updateEvent } from "../../api";
import { DragController } from "../dragController";
import type { DragSurface } from "../dragController";
import { clampToMatrix } from "../events";
import type { EventRef } from "../events";
import { cellYAtTile } from "../layout";
import { MARKER_Y, moveMarker, pickMarker } from "../markers";
import type { MarkerSet } from "../markers";
import { refreshEvents } from "./eventFetch";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { ReportEvents, SceneRefs } from "./types";

/** What an event drag holds on to: the marker set it started in, the event as it was at the press. */
export interface EventDrag {
  set: MarkerSet;
  file: number;
  kind: EventRef["kind"];
  data: { index: number; x: number; z: number };
}

/**
 * Dragging the selected event marker moves it on the plane of its cell's height in whole tiles (`origin` is the
 * event's tile, so the snapped positions are tiles too). A locked event file cannot be dragged. A
 * press only starts a drag on the selected marker itself; one `update_event` is written when the
 * pointer is released.
 */
export function createEventDrag(h: SceneHost, picker: Picker, surface: DragSurface, refs: SceneRefs, report: ReportEvents): DragController<EventDrag, THREE.Plane> {
  /** Shows the event on its recorded tile again (a drag that did not take, or was refused). */
  const restoreEvent = ({ set, kind, data }: EventDrag) => {
    // The markers may have been rebuilt meanwhile: they show what the backend holds.
    if (h.markers !== set) return;
    moveMarker(set, { kind, index: data.index }, { x: data.x, z: data.z });
    h.render();
  };
  return new DragController<EventDrag, THREE.Plane>(surface, 1, {
    begin: (e) => {
      const selected = h.selectedEvent;
      const events = h.events;
      if (!selected || !events || events.locked !== null || !h.markers || selected.file !== events.event_file_id || !refs.callbacks.current.paintEnabled) return null;
      picker.aim(e.clientX, e.clientY);
      const hit = pickMarker(h.markers, picker.raycaster);
      if (!hit || hit.kind !== selected.kind || hit.data.index !== selected.index) return null;
      return {
        subject: { set: h.markers, file: events.event_file_id, kind: hit.kind, data: { index: hit.data.index, x: hit.data.x, z: hit.data.z } },
        plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -(MARKER_Y + (h.matrix ? cellYAtTile(h.matrix, hit.data.x, hit.data.z) : 0))),
        origin: { x: hit.data.x, z: hit.data.z },
      };
    },
    pointOn: picker.planePoint,
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
          refs.callbacks.current.onEventMoved(d.file);
          await refreshEvents(h, d.file, refs.showEvents.current, () => refs.eventFile.current === d.file, report);
        },
        (err) => {
          restoreEvent(d);
          refs.callbacks.current.onError(err);
        },
      );
      h.refreshHover?.();
    },
    cancel: (d) => {
      restoreEvent(d);
      h.refreshHover?.();
    },
  });
}
