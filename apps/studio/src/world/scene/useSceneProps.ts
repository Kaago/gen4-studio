import { useEffect } from "react";
import { markFaulty, selectMarker } from "../markers";
import type { EventRef } from "../events";
import type { SceneRefs } from "./types";

/** The selected building (made by a click, the header list, ...) is outlined; the scene also drags it. */
export function useBuildingSelection(refs: SceneRefs, selectedMap: number | null, selectedBuilding: number | null) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    h.selection = selectedMap === null || selectedBuilding === null ? null : { map: selectedMap, index: selectedBuilding };
    h.buildings?.select(h.selection);
    h.render();
  }, [selectedMap, selectedBuilding]);
}

/** The selected event (made by a click, the header list, ...) is outlined; the scene also drags it. */
export function useEventSelection(refs: SceneRefs, selectedEvent: EventRef | null) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    h.selectedEvent = selectedEvent;
    if (h.markers) selectMarker(h.markers, selectedEvent && selectedEvent.file === h.events?.event_file_id ? selectedEvent : null);
    h.render();
  }, [selectedEvent]);
}

/** The events an error finding hits (of any file; the scene picks those of the file it shows) take the error colour. */
export function useFaultyEvents(refs: SceneRefs, faulty: readonly EventRef[]) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    h.faulty = faulty;
    if (h.markers && h.events) {
      const file = h.events.event_file_id;
      markFaulty(h.markers, faulty.filter((ref) => ref.file === file));
      h.render();
    }
  }, [faulty]);
}

/** Shows or hides the permission overlay. */
export function usePermissionVisibility(refs: SceneRefs, showPermissions: boolean) {
  useEffect(() => {
    refs.showPermissions.current = showPermissions;
    const h = refs.handle.current;
    if (h?.overlay) {
      h.overlay.visible = showPermissions;
      h.render();
    }
  }, [showPermissions]);
}

/** Shows or hides the event markers. */
export function useEventVisibility(refs: SceneRefs, showEvents: boolean) {
  useEffect(() => {
    refs.showEvents.current = showEvents;
    const h = refs.handle.current;
    if (h?.markers) {
      h.markers.group.visible = showEvents;
      h.render();
      // Hidden markers must stop being hovered, shown ones start to be.
      h.refreshHover?.();
    }
  }, [showEvents]);
}
