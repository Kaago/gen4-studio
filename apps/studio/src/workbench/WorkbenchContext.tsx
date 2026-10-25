import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { AssetId, EventKind, HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, ProjectInfo, WarpLink } from "../api";
import type { Message } from "../app/message";
import type { EventPlacement } from "../app/useWorldActions";
import type { Depth, Tool } from "../ui/frame/shellState";
import type { MapBuildings, TileClick } from "../world/scene/types";
import type { BuildingRef } from "../world/buildingLayer";
import type { EditBus } from "../bus/editBus";
import type { DataStore } from "../data/dataStore";
import type { EventLock, EventRef } from "../world/events";
import type { InspectorModel } from "./Inspector";
import type { NavTarget } from "./navigation";
import type { ChecksApi } from "../checks/useChecks";
import type { ProblemsStore } from "./problems/problems";
import type { ScriptsApi } from "./script/useScripts";
import type { TextTarget } from "./text/textModel";

/** What a panel asks the app to open: the wizard for a new area or a new room, with what it can already fill in. */
export type WizardRequest = { kind: "area"; prefill?: { matrix?: number } } | { kind: "room"; prefill?: { outsideHeader?: number; template?: number } };

/**
 * The data the panels show that changes rarely: the header list, the selection, the matrix, the
 * switches and the actions. App owns the state and builds one object that keeps its identity
 * until one of these fields changes (see `useWorkbenchModel`), so a panel that reads only this
 * context is not rendered again by the volatile state in `WorkbenchLive`.
 */
export interface WorkbenchData {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  /** One summary per matrix and every warp of the project (the overview behind the world explorer). */
  matrices: MatrixSummary[];
  warpLinks: WarpLink[];
  /** False until the first matrix listing of the project arrived: `matrices` is empty then, which says nothing about the project. */
  matricesLoaded: boolean;
  /** The matrix listing failed and nothing is loaded (see `Overview.failed`). */
  matricesFailed: boolean;
  /** Opens the wizard for a new area or room. */
  onOpenWizard: (request: WizardRequest) => void;
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
  /** Edits in the scene (painting, adding, dragging) are allowed; false while a build runs. */
  paintEnabled: boolean;
  /** The depth of the whole app: Tüfteln adds the technical section, the Dez | Hex switches and the identifiers in the explorer. It changes rarely. */
  depth: Depth;
  /** The active tool of the palette. */
  tool: Tool;
  /** Switches the tool; switching to painting also shows the terrain layer. */
  onTool: (tool: Tool) => void;
  /** A click in the scene with one of the adding tools landed on this tile: the app adds what the tool makes there and returns to "Auswählen". */
  onTileClick: (tile: TileClick) => void;
  /** The buildings of the shown maps as the scene reports them (maps without buildings are left out). */
  buildings: MapBuildings[];
  /** The scene reports the buildings of the shown maps. */
  onBuildings: (buildings: MapBuildings[]) => void;
  /** Bumped by every undo and redo, at once (a ref, not state): a load in flight asks whether history moved while it ran. */
  history: { readonly current: number };
  onEdited: () => void;
  onError: (e: unknown) => void;
  /** Load progress / failure summary of the scene; null clears it (only if it is still the shown message). */
  onLoadStatus: (message: Message | null) => void;
  /** The scene reports what is under the pointer (`none` when it left every cell and marker). */
  onInspect: (model: InspectorModel) => void;
  memberCounts: MemberCounts;
  /** The assets with unsaved changes; it changes only when a write or a save moved them. */
  dirtyAssets: readonly AssetId[];
  /** A header field was written: refresh list, scene and dirty count. */
  onHeaderChanged: () => void | Promise<void>;
  /** An area data field was written: refresh the dirty count and, if the shown models' tileset changed, the scene. */
  onAreaChanged: () => void | Promise<void>;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onMatrixEdited: (index: number) => void | Promise<void>;
  /** A map or matrix member was appended: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  /** Parts were imported into a map: its pictures are dropped, the scene reloads the models and the dirty count catches up. */
  onMapImported: (map: number) => void | Promise<void>;
  /** The building selected in the scene (it is the pinned inspector model); null: none. */
  selectedBuilding: BuildingRef | null;
  /** A click in the scene chose a building (null: none). */
  onSelectBuilding: (ref: BuildingRef | null) => void;
  /** A drag moved a building and the move is written. */
  onBuildingMoved: (map: number) => void;
  /** The building editor wrote a field: refresh the scene and the dirty count. */
  onBuildingChanged: (map: number, retryModel: number | null) => void | Promise<void>;
  /** Appends a building on the tile `at` (a click), else on the last hovered tile (else in `fallbackMap`, when given) and selects it. */
  onAddBuilding: (fallbackMap: number | null, at?: { map: number; x: number; y: number }) => void | Promise<void>;
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
  /**
   * Adds an event on the tile `at` (a click), else on the last hovered tile of the selected header's event file and selects it; `placement`
   * sets fields of the new event in the same undo step.
   */
  onAddEvent: (kind: EventKind, at?: { x: number; z: number }, placement?: EventPlacement) => void | Promise<void>;
  /** Asks first, then deletes the event. */
  onDeleteEvent: (eventFile: number, kind: EventKind, index: number) => void | Promise<void>;
  /** The archive and message a link to the text panel asked for; a new object with every request, so the same one can be asked for again. Null: none yet. */
  textTarget: TextTarget | null;
  /** The project's scripts: status, open documents and their commands. Changes with its `revision`. */
  scripts: ScriptsApi;
  /** The problems of the project; a stable object that notifies its own subscribers. */
  problems: ProblemsStore;
  /** The checks of the project (a stable object): fixes, ignoring, „Erneut prüfen“; their state notifies its own subscribers. */
  checks: ChecksApi;
  /** The data lists of the project (a stable object): the world names trainer figures from its trainer rows. */
  dataStore: DataStore;
}

/** The cell of a matrix that the inspector shows as selected (a matrix tab highlight). */
export interface MatrixSelection {
  matrix: number;
  x: number;
  y: number;
}

/**
 * What changes with the selection and the session message, each in a context of its own so a panel
 * subscribes only to what it renders: selecting must not render the header list or the matrix tab
 * (unless the selection is a matrix cell). The scene's hover is not here at all: it goes to the status line. The message is not read by any panel any more (the frame shows it as a toast); it stays
 * here as the one place that says what the session currently shows.
 */
export interface WorkbenchLive {
  /** What the inspector shows: the selection (the pin); the scene's hover goes to the status line. */
  inspector: InspectorModel;
  /** The inspector's cell if it is a matrix cell, else null. Keeps its identity while the same cell shows. */
  matrixSelection: MatrixSelection | null;
  /** The session message the frame mirrors into the toasts. */
  message: Message | null;
}

const WorkbenchContext = createContext<WorkbenchData | null>(null);
const InspectorContext = createContext<InspectorModel>({ kind: "none" });
const MatrixSelectionContext = createContext<MatrixSelection | null>(null);

export function WorkbenchProviders({ data, live, children }: { data: WorkbenchData; live: WorkbenchLive; children: ReactNode }) {
  return (
    <WorkbenchContext.Provider value={data}>
      <InspectorContext.Provider value={live.inspector}>
        <MatrixSelectionContext.Provider value={live.matrixSelection}>
          {children}
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

/** Changes with the selection; only the inspector reads it. */
export function useInspectorModel(): InspectorModel {
  return useContext(InspectorContext);
}

export function useMatrixSelection(): MatrixSelection | null {
  return useContext(MatrixSelectionContext);
}
