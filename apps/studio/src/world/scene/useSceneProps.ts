import { useEffect } from "react";
import type { MatrixDto } from "../../api";
import type { CellRect } from "../../workbench/sceneScope";
import { markFaulty, selectMarker } from "../markers";
import type { EventRef } from "../events";
import { frameCellTopDown, requestedCell } from "./camera";
import { showCellFrame } from "./cellFrame";
import { reportSelectedMarker } from "./content";
import { setHeightsShown } from "./heightLayer";
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
    reportSelectedMarker(h);
    h.render();
  }, [selectedEvent]);
}

/**
 * The selected cell of `matrix` (a click on the ground, a link, the Raster, ...) is framed. While the scene still shows another matrix,
 * its frame goes; the load of `matrix` draws the frame once the matrix is there.
 */
export function useCellSelection(refs: SceneRefs, matrix: MatrixDto, selectedCell: { x: number; y: number } | null) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h) return;
    h.selectedCell = selectedCell;
    if (h.matrix === matrix) showCellFrame(h);
    else h.setCellFrame(null);
    h.render();
  }, [selectedCell]);
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

/** Switches the camera between the slanted view and the top-down view; switching on frames `focus` of `matrix`, switching off brings back the slanted view as it was left. */
export function useTopDown(refs: SceneRefs, topDown: boolean, matrix: MatrixDto, focus: CellRect) {
  useEffect(() => {
    refs.handle.current?.setTopDown(topDown, focus, matrix);
  }, [topDown]);
}

/**
 * „Gelände anpassen“ asked to frame a cell (`request`): once the top view is on (`topDown`, which the same step switches), the scene is
 * built for `matrix` and that cell holds the paint `target` (after the matrix refresh of „Neue Karte malen“), the top-down view frames
 * the cell, also when it was on already, and `done` clears the request. Runs after every build (`built`), so a full load that framed the
 * area first is followed by the cell.
 */
export function useFrameRequest(
  refs: SceneRefs,
  request: { matrix: number; x: number; y: number } | null | undefined,
  matrix: MatrixDto,
  topDown: boolean,
  target: number | null,
  built: number,
  done: () => void,
) {
  useEffect(() => {
    const h = refs.handle.current;
    if (!h || !topDown) return;
    const cell = requestedCell(request, matrix, h.matrix, target);
    if (!cell) return;
    frameCellTopDown(h, cell, matrix);
    done();
  }, [request, matrix, topDown, target, built]);
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

/** Shows or hides the layer „Höhen“, built on its first switch-on (the canvas says so in `data-heights-shown`). */
export function useHeightVisibility(refs: SceneRefs, showHeights: boolean) {
  useEffect(() => {
    refs.showHeights.current = showHeights;
    const h = refs.handle.current;
    // Before the first matrix is loaded there is nothing to show; the load reads the switch itself.
    if (!h?.matrix) return;
    setHeightsShown(h, showHeights);
    h.render();
  }, [showHeights]);
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
