import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { AssetId, DraftDto, EventKind, HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, ProjectInfo, WarpLink } from "../api";
import type { Message } from "../app/message";
import type { Created } from "../app/useWizards";
import type { EventPlacement } from "../app/useWorldActions";
import type { Depth, Tool } from "../ui/frame/shellState";
import type { MapBuildings, TileClick } from "../world/scene/types";
import type { BuildingRef } from "../world/buildingLayer";
import type { EditBus } from "../bus/editBus";
import type { DataStore } from "../data/dataStore";
import type { EventLock, EventRef } from "../world/events";
import { headerIdentity } from "./headerIdentity";
import type { HeaderIdentity } from "./headerIdentity";
import type { InspectorModel } from "./Inspector";
import type { NavTarget } from "./navigation";
import type { ChecksApi } from "../checks/useChecks";
import type { ProblemsStore } from "./problems/problems";
import type { ScriptsApi } from "./script/useScripts";
import type { TextTarget } from "./text/textModel";
import type { DialogTarget } from "./dialog/dialogModel";
import { EMPTY_WORLD_META } from "./useWorldModel";
import type { WorldModel } from "./useWorldModel";

/**
 * What a panel asks the app to open: the wizard for a new area or a new room, with what it can already fill in, the wizard that
 * clears the world down to the start town ("Welt leeren"), or one of the recipes of Erzählen (a figure, two doors, a trainer).
 */
export type WizardRequest =
  | { kind: "area"; prefill?: { matrix?: number } }
  | { kind: "room"; prefill?: { outsideHeader?: number; template?: number } }
  | { kind: "clearWorld" }
  | { kind: "publish" }
  // The recipes of 3d.5: where the figure or the trainer stands (a tile of the header's matrix), and door A of „Tür verbinden“ (a door
  // there or a tile for a new one).
  | { kind: "new_figure"; prefill?: { header?: number; tile?: { x: number; z: number } } }
  | { kind: "connect_doors"; prefill?: { a?: { header: number; warp?: number; tile?: { x: number; z: number } } } }
  | { kind: "new_trainer_here"; prefill?: { header?: number; tile?: { x: number; z: number } } };

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
  /** The project got new members outside a wizard (a room split off): it is read again as after a wizard and `made.header` shown with the announcement. */
  onCreated: (made: Created) => Promise<void>;
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
  /** Follows a link: selects, pins the inspector and focuses the panel that shows the target. */
  goTo: (target: NavTarget) => void;
  /** Shows the scene shown before / after this one (the history of the scenes shown; „Zurück“ and „Vor“ of „In dieser Szene“). */
  sceneBack: () => void;
  sceneForward: () => void;
  canBack: boolean;
  canForward: boolean;
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
  /** Pins a selection as a click in the scene does (a row of „In dieser Szene“ chose it): the inspector, the halo and the breadcrumb follow. */
  onPin: (model: InspectorModel) => void;
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
  /** A click on the ground of the scene chose the map in cell (`x`, `y`) of matrix `matrix`; it is pinned. */
  onSelectCell: (matrix: number, cell: { x: number; y: number }) => void;
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
  /** What an open wizard plans (null: nothing): the scene and the world map draw it dashed. */
  draft: DraftDto | null;
  /** Sets the draft: the dialog „Karte bei … anlegen?“ plans its new cell (a stable function). */
  onDraft: (draft: DraftDto | null) => void;
  /** Opens the dialog editor of a figure or a sign (the field „Dialog“ of the inspector in Erzählen); a stable function. */
  onOpenDialog: (target: DialogTarget) => void;
}

/** The cell of a matrix that the inspector shows as selected (a matrix tab highlight). */
export interface MatrixSelection {
  matrix: number;
  x: number;
  y: number;
}

/**
 * The world model of the open project (`useWorldModel`), built once for the workbench: the explorer shows it, and the inspector
 * head, the breadcrumb and the door under the pointer name headers with `identify`, so they say what the world tree says.
 */
export interface WorkbenchWorld extends WorldModel {
  /** What a header is and how it is called: from the world once it is read, from the places until then. Keeps its identity until either changes. */
  identify: (header: HeaderSummary) => HeaderIdentity;
}

/** The world before a project is read (and for panels rendered without one). */
export const NO_WORLD: WorkbenchWorld = {
  world: null,
  names: null,
  meta: EMPTY_WORLD_META,
  broken: null,
  setMeta: async () => {},
  reread: () => {},
  identify: (header) => headerIdentity([], header),
};

/**
 * What changes with the selection, the world and the session message, each in a context of its own so a
 * panel subscribes only to what it renders: selecting must not render the header list or the matrix tab
 * (unless the selection is a matrix cell). The scene's hover is not here at all: it goes to the status
 * line. The message is not read by any panel any more (the frame shows it as a toast); it stays here as
 * the one place that says what the session currently shows.
 */
export interface WorkbenchLive {
  /** What the inspector shows: the selection (the pin); the scene's hover goes to the status line. */
  inspector: InspectorModel;
  /** The inspector's cell if it is a matrix cell, else null. Keeps its identity while the same cell shows. */
  matrixSelection: MatrixSelection | null;
  /** The session message the frame mirrors into the toasts. */
  message: Message | null;
  /** Changes when the world is read again (a project opened, matrix data or the header list moved, the project file written). */
  world: WorkbenchWorld;
}

const WorkbenchContext = createContext<WorkbenchData | null>(null);
const InspectorContext = createContext<InspectorModel>({ kind: "none" });
const MatrixSelectionContext = createContext<MatrixSelection | null>(null);
const WorldContext = createContext<WorkbenchWorld>(NO_WORLD);

export function WorkbenchProviders({ data, live, children }: { data: WorkbenchData; live: WorkbenchLive; children: ReactNode }) {
  return (
    <WorkbenchContext.Provider value={data}>
      <InspectorContext.Provider value={live.inspector}>
        <MatrixSelectionContext.Provider value={live.matrixSelection}>
          <WorldContext.Provider value={live.world}>{children}</WorldContext.Provider>
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

/** The world model and the names of the headers (see `WorkbenchWorld`). */
export function useWorld(): WorkbenchWorld {
  return useContext(WorldContext);
}
