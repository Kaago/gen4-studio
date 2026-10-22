import { memo, useRef, useState } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import type { Message } from "../Toolbar";
import type { InspectorModel } from "../workbench/Inspector";
import type { RefreshQueue } from "../reload";
import type { BuildingRef } from "./buildingLayer";
import type { BuildingRefresh } from "./buildings";
import type { EventLock, EventRef, EventRefresh } from "./events";
import { eventsNote } from "./scene/eventFetch";
import type { ReportEvents } from "./scene/types";
import { useBuildingRefresh, useEventRefresh, useMapRefresh } from "./scene/useRefreshQueues";
import { useSceneLoader } from "./scene/useSceneLoader";
import { useSceneRefs } from "./scene/useSceneRefs";
import { useSceneSetup } from "./scene/useSceneSetup";
import { useBuildingSelection, useEventSelection, useEventVisibility, usePermissionVisibility } from "./scene/useSceneSync";

interface Props {
  matrix: MatrixDto;
  /**
   * Area data of the shown header; selects the map texture set for every map of the matrix. It
   * belongs to `matrix` (App changes both in one update when another header is shown). Changed
   * alone, or with a changed `modelsToken`, only the models reload, not the maps and markers.
   */
  areaDataId: number;
  /** Increment to reload the models alone (the shown area data's tilesets were edited). */
  modelsToken: number;
  showPermissions: boolean;
  /** Event file of the selected header; its events are shown as markers (null: none). */
  eventFileId: number | null;
  showEvents: boolean;
  /** Header list, for the internal names of warp destinations. */
  headers: HeaderSummary[];
  /** Painting is ignored while false (e.g. during a build). */
  paintEnabled: boolean;
  /**
   * Bumped by every undo and redo, at once (not through a render). A matrix load that was in its
   * map phase meanwhile fetches again, so it never shows the state from before the undo.
   */
  history: { readonly current: number };
  /** A new request makes the scene refetch one map (overlay colours and buildings of all its cells): undo or redo restored it. */
  mapRefresh: RefreshQueue<number> | null;
  /** Called after a tile was changed in the backend. */
  onEdited: () => void;
  onError: (message: string) => void;
  /** Reports what the inspector should show (filled in by tile hover and event markers). */
  onInspect: (model: InspectorModel) => void;
  /** Load progress and the summary of what failed to load (null: nothing to show, clears the progress). */
  onStatus: (message: Message | null) => void;
  /** The selected building (both null: none). The scene outlines it and lets it be dragged. */
  selectedMap: number | null;
  selectedBuilding: number | null;
  /** A click on a building (or null for none). The owner decides what to select and passes it back. */
  onSelectBuilding: (ref: BuildingRef | null) => void;
  /** A building was moved by dragging and the move is written. */
  onBuildingMoved: (map: number) => void;
  /** A new request makes the scene read one map's buildings again (they were edited elsewhere). */
  buildingRefresh: RefreshQueue<BuildingRefresh> | null;
  /** The selected event (null: none). The scene outlines it and lets it be dragged. */
  selectedEvent: EventRef | null;
  /** A click on an event marker (or null for none). The owner decides what to select and passes it back. */
  onSelectEvent: (ref: EventRef | null) => void;
  /** An event was moved by dragging and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** A new request makes the scene read one event file's events again (they were edited elsewhere). */
  eventRefresh: EventRefresh | null;
  /** The shown event file's lock state, reported whenever its events were read (null: not locked, or none shown). */
  onEventsLocked: (lock: EventLock | null) => void;
}

function MatrixScene({
  matrix,
  areaDataId,
  modelsToken,
  showPermissions,
  eventFileId,
  showEvents,
  headers,
  paintEnabled,
  history,
  mapRefresh,
  onEdited,
  onError,
  onInspect,
  onStatus,
  selectedMap,
  selectedBuilding,
  onSelectBuilding,
  onBuildingMoved,
  buildingRefresh,
  selectedEvent,
  onSelectEvent,
  onEventMoved,
  eventRefresh,
  onEventsLocked,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [eventStatus, setEventStatus] = useState<string | null>(null);
  // Counts the scenes built (maps fetched, content in place): queued building edits are read again then.
  const [sceneBuilt, setSceneBuilt] = useState(0);
  const refs = useSceneRefs({
    showPermissions,
    showEvents,
    eventFileId,
    headers,
    areaDataId,
    callbacks: { paintEnabled, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved, onEventsLocked },
  });
  /** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
  const reportEvents: ReportEvents = (fetched) => {
    setEventStatus(eventsNote(fetched));
    const events = fetched.events;
    refs.callbacks.current.onEventsLocked(events?.locked ? { file: events.event_file_id, reason: events.locked } : null);
  };

  // The hooks run their effects in this order, which is the order the effects always had.
  useSceneSetup(host, refs, reportEvents);
  useSceneLoader({ matrix, areaDataId, modelsToken, history, refs, reportEvents, clearEventStatus: () => setEventStatus(null), onSceneBuilt: () => setSceneBuilt((n) => n + 1) });
  useMapRefresh(refs, mapRefresh);
  useBuildingSelection(refs, selectedMap, selectedBuilding);
  useBuildingRefresh(refs, buildingRefresh, sceneBuilt);
  useEventSelection(refs, selectedEvent);
  useEventRefresh(refs, eventRefresh, reportEvents);
  usePermissionVisibility(refs, showPermissions);
  useEventVisibility(refs, showEvents);

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      <div ref={host} style={{ position: "absolute", inset: 0 }} />
      {eventStatus && (
        <div style={{ position: "absolute", left: 8, bottom: 8, color: "#e5e7eb", font: "12px system-ui", maxWidth: "70%" }}>
          {eventStatus}
        </div>
      )}
    </div>
  );
}

// Hover moves only the inspector state in App; the scene itself must not re-render for it.
export default memo(MatrixScene);
