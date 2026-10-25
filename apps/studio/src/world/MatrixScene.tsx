import { memo, useRef, useState } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import type { Message } from "../app/message";
import type { Tool } from "../ui/frame/shellState";
import type { InspectorModel } from "../workbench/Inspector";
import type { EditBus } from "../bus/editBus";
import { useBusQueue, useBusRevision } from "../bus/useBusEvent";
import type { BuildingRef } from "./buildingLayer";
import type { EventLock, EventRef } from "./events";
import { eventsNote } from "./scene/eventFetch";
import type { MapBuildings, ReportEvents, TileClick } from "./scene/types";
import { useBuildingRefresh, useEventRefresh, useMapRefresh } from "./scene/useRefreshQueues";
import { useSceneLoader } from "./scene/useSceneLoader";
import { useSceneRefs } from "./scene/useSceneRefs";
import { useSceneSetup } from "./scene/useSceneSetup";
import { useSceneTheme } from "./scene/useSceneTheme";
import styles from "./MatrixScene.module.css";
import { useBuildingSelection, useEventSelection, useEventVisibility, useFaultyEvents, usePermissionVisibility } from "./scene/useSceneProps";

interface Props {
  matrix: MatrixDto;
  /**
   * Area data of the shown header; selects the map texture set for every map of the matrix. It
   * belongs to `matrix` (App changes both in one update when another header is shown). Changed
   * alone, or on a `modelsReload` event, only the models reload, not the maps and markers.
   */
  areaDataId: number;
  /**
   * Where the scene hears that its data was edited elsewhere (undo, redo, the editors): maps,
   * buildings and event files to read again, and the models to reload. A stable object, so the
   * scene stays memoised. Every event is kept, in order, until the scene has handled it.
   */
  bus: EditBus;
  showPermissions: boolean;
  /** Event file of the selected header; its events are shown as markers (null: none). */
  eventFileId: number | null;
  showEvents: boolean;
  /** Header list, for the internal names of warp destinations. */
  headers: HeaderSummary[];
  /** Edits (painting, adding, dragging) are ignored while false (e.g. during a build). */
  paintEnabled: boolean;
  /** The active tool: what a click on a tile does (see `canvasClicks`). */
  tool: Tool;
  /** A click with one of the adding tools landed on this tile. */
  onTileClick: (tile: TileClick) => void;
  /** The buildings of the shown maps, reported whenever they were loaded or changed. */
  onBuildings: (buildings: MapBuildings[]) => void;
  /**
   * Bumped by every undo and redo, at once (not through a render). A matrix load that was in its
   * map phase meanwhile fetches again, so it never shows the state from before the undo.
   */
  history: { readonly current: number };
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
  /** The selected event (null: none). The scene outlines it and lets it be dragged. */
  selectedEvent: EventRef | null;
  /** A click on an event marker (or null for none). The owner decides what to select and passes it back. */
  onSelectEvent: (ref: EventRef | null) => void;
  /** An event was moved by dragging and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** The shown event file's lock state, reported whenever its events were read (null: not locked, or none shown). */
  onEventsLocked: (lock: EventLock | null) => void;
  /** The events an error finding of the checks hits; their markers show the error colour. Keeps its identity while it does not change. */
  faultyEvents: readonly EventRef[];
}

function MatrixScene({
  matrix,
  areaDataId,
  bus,
  showPermissions,
  eventFileId,
  showEvents,
  headers,
  paintEnabled,
  tool,
  onTileClick,
  onBuildings,
  history,
  onEdited,
  onError,
  onInspect,
  onStatus,
  selectedMap,
  selectedBuilding,
  onSelectBuilding,
  onBuildingMoved,
  selectedEvent,
  onSelectEvent,
  onEventMoved,
  onEventsLocked,
  faultyEvents,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [eventStatus, setEventStatus] = useState<string | null>(null);
  // Counts the scenes built (maps fetched, content in place): queued building edits are read again then.
  const [sceneBuilt, setSceneBuilt] = useState(0);
  // The bus subscription hooks must stay declared before useSceneSetup/useSceneLoader (effect order: events queue before the scene reacts).
  // The bus events as the queues the effects below drain (see `useBusQueue`), and the counter that reloads the models.
  const mapRefresh = useBusQueue(bus, "mapRefetch", (e) => e.map);
  const buildingRefresh = useBusQueue(bus, "buildingsRefetch", (e) => ({ map: e.map, retryModel: e.retryModel }));
  const eventRefresh = useBusQueue(bus, "eventsRefetch", (e) => e.file);
  const modelsToken = useBusRevision(bus, "modelsReload");
  const refs = useSceneRefs({
    showPermissions,
    showEvents,
    eventFileId,
    headers,
    areaDataId,
    callbacks: { paintEnabled, tool, onTileClick, onBuildings, onEdited, onError, onInspect, onStatus, onSelectBuilding, onBuildingMoved, onSelectEvent, onEventMoved, onEventsLocked },
  });
  /** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
  const reportEvents: ReportEvents = (fetched) => {
    setEventStatus(eventsNote(fetched));
    const events = fetched.events;
    refs.callbacks.current.onEventsLocked(events?.locked ? { file: events.event_file_id, reason: events.locked } : null);
  };

  // The hooks run their effects in this order, which is the order the effects always had.
  useSceneSetup(host, refs, reportEvents);
  useSceneTheme(refs);
  useSceneLoader({ matrix, areaDataId, modelsToken, history, refs, reportEvents, clearEventStatus: () => setEventStatus(null), onSceneBuilt: () => setSceneBuilt((n) => n + 1) });
  useMapRefresh(refs, mapRefresh);
  useBuildingSelection(refs, selectedMap, selectedBuilding);
  useBuildingRefresh(refs, buildingRefresh, sceneBuilt);
  useEventSelection(refs, selectedEvent);
  useFaultyEvents(refs, faultyEvents);
  useEventRefresh(refs, eventRefresh, reportEvents);
  usePermissionVisibility(refs, showPermissions);
  useEventVisibility(refs, showEvents);

  return (
    <div className={styles.scene}>
      <div ref={host} className={styles.canvas} />
      {eventStatus && <div className={styles.status}>{eventStatus}</div>}
    </div>
  );
}

// Hover moves only the inspector state in App; the scene itself must not re-render for it.
export default memo(MatrixScene);
