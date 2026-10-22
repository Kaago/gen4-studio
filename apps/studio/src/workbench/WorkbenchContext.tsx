import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { EventKind, HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "../api";
import type { Message } from "../Toolbar";
import type { BuildingRef } from "../world/buildingLayer";
import type { EditBus } from "../bus/editBus";
import type { EventLock, EventRef } from "../world/events";
import type { InspectorModel } from "./Inspector";
import type { NavTarget } from "./navigation";

/**
 * The data the panels show that changes rarely: the header list, the selection, the matrix, the
 * switches and the actions. App owns the state and builds one object that keeps its identity
 * until one of these fields changes (see `useWorkbenchModel`), so a panel that reads only this
 * context is not rendered again by the volatile state in `WorkbenchLive`.
 */
export interface WorkbenchData {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
  /** Follows a link: selects, pins the inspector and focuses the panel that shows the target. */
  goTo: (target: NavTarget) => void;
  /** Where the writers announce that data moved: the panels and the scene listen. A stable object. */
  bus: EditBus;
  matrix: MatrixDto | null;
  /** Area data the scene's models are textured with (follows the shown header, set together with `matrix`). */
  areaDataId: number;
  showPermissions: boolean;
  onShowPermissions: (show: boolean) => void;
  /** Event file of the selected header (null: none selected). */
  eventFileId: number | null;
  showEvents: boolean;
  onShowEvents: (show: boolean) => void;
  paintEnabled: boolean;
  /** Bumped by every undo and redo, at once (a ref, not state): a load in flight asks whether history moved while it ran. */
  history: { readonly current: number };
  onEdited: () => void;
  onError: (e: unknown) => void;
  /** Load progress / failure summary of the scene; null clears it (only if it is still the shown message). */
  onLoadStatus: (message: Message | null) => void;
  /** The scene reports what is under the pointer (`none` when it left every cell and marker). */
  onInspect: (model: InspectorModel) => void;
  memberCounts: MemberCounts;
  /** A header field was written: refresh list, scene and dirty count. */
  onHeaderChanged: () => void | Promise<void>;
  /** An area data field was written: refresh the dirty count and, if the shown models' tileset changed, the scene. */
  onAreaChanged: () => void | Promise<void>;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onMatrixEdited: (index: number) => void | Promise<void>;
  /** A map or matrix member was appended: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  /** The building selected in the scene (it is the pinned inspector model); null: none. */
  selectedBuilding: BuildingRef | null;
  /** A click in the scene chose a building (null: none). */
  onSelectBuilding: (ref: BuildingRef | null) => void;
  /** A drag moved a building and the move is written. */
  onBuildingMoved: (map: number) => void;
  /** The building editor wrote a field: refresh the scene and the dirty count. */
  onBuildingChanged: (map: number, retryModel: number | null) => void | Promise<void>;
  /** Appends a building on the last hovered tile (else in `fallbackMap`, when given) and selects it. */
  onAddBuilding: (fallbackMap: number | null) => void | Promise<void>;
  /** Asks first, then deletes the building. */
  onDeleteBuilding: (map: number, index: number) => void | Promise<void>;
  /** The event selected in the scene (it is the pinned inspector model); null: none. */
  selectedEvent: EventRef | null;
  /** A click in the scene chose an event marker (null: none). */
  onSelectEvent: (ref: EventRef | null) => void;
  /** A drag moved an event and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** The lock state of the shown event file (null: not locked, or none shown); the scene reports it. */
  eventLock: EventLock | null;
  onEventsLocked: (lock: EventLock | null) => void;
  /** The event editor wrote a field: refresh the scene and the dirty count. */
  onEventChanged: (eventFile: number) => void | Promise<void>;
  /** Adds an event on the last hovered tile of the selected header's event file and selects it. */
  onAddEvent: (kind: EventKind) => void | Promise<void>;
  /** Asks first, then deletes the event. */
  onDeleteEvent: (eventFile: number, kind: EventKind, index: number) => void | Promise<void>;
  onDismissMessage: () => void;
}

/** The cell of a matrix that the inspector shows as selected (a matrix tab highlight). */
export interface MatrixSelection {
  matrix: number;
  x: number;
  y: number;
}

/**
 * What changes with the pointer and the message bar, each in a context of its own so a panel
 * subscribes only to what it renders: hovering the scene must not render the header list or the
 * matrix tab, and a load progress line must not render anything but the status strip.
 */
export interface WorkbenchLive {
  /** What the inspector shows: the scene's hover while the pointer is over a cell, else the pinned model. */
  inspector: InspectorModel;
  /** The inspector's cell if it is a matrix cell, else null. Keeps its identity while the same cell shows. */
  matrixSelection: MatrixSelection | null;
  message: Message | null;
}

const WorkbenchContext = createContext<WorkbenchData | null>(null);
const InspectorContext = createContext<InspectorModel>({ kind: "none" });
const MatrixSelectionContext = createContext<MatrixSelection | null>(null);
const MessageContext = createContext<Message | null>(null);

export function WorkbenchProviders({ data, live, children }: { data: WorkbenchData; live: WorkbenchLive; children: ReactNode }) {
  return (
    <WorkbenchContext.Provider value={data}>
      <InspectorContext.Provider value={live.inspector}>
        <MatrixSelectionContext.Provider value={live.matrixSelection}>
          <MessageContext.Provider value={live.message}>{children}</MessageContext.Provider>
        </MatrixSelectionContext.Provider>
      </InspectorContext.Provider>
    </WorkbenchContext.Provider>
  );
}

/** The rarely changing data (see `WorkbenchData`). */
export function useWorkbench(): WorkbenchData {
  const data = useContext(WorkbenchContext);
  if (!data) throw new Error("panel rendered outside of the workbench");
  return data;
}

/** Changes with every hover; only the inspector reads it. */
export function useInspectorModel(): InspectorModel {
  return useContext(InspectorContext);
}

export function useMatrixSelection(): MatrixSelection | null {
  return useContext(MatrixSelectionContext);
}

/** Changes with the message and every load progress line; only the status strip reads it. */
export function useMessage(): Message | null {
  return useContext(MessageContext);
}
