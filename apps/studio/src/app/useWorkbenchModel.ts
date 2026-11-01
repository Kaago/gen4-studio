import { useCallback, useMemo } from "react";
import type { DraftDto, HeaderSummary } from "../api";
import { useBusEvent } from "../bus/useBusEvent";
import type { Depth, Tool } from "../ui/frame/shellState";
import type { TileClick } from "../world/scene/types";
import type { EditBus } from "../bus/editBus";
import type { DataStore } from "../data/dataStore";
import type { ChecksApi } from "../checks/useChecks";
import type { ProblemsStore } from "../workbench/problems/problems";
import type { ScriptsApi } from "../workbench/script/useScripts";
import type { TextTarget } from "../workbench/text/textModel";
import type { DialogTarget } from "../workbench/dialog/dialogModel";
import { headerIdentity } from "../workbench/headerIdentity";
import { buildPlaces } from "../workbench/places";
import { useWorldModel } from "../workbench/useWorldModel";
import type { MatrixSelection, WizardRequest, WorkbenchData, WorkbenchLive, WorkbenchWorld } from "../workbench/WorkbenchContext";
import type { Created } from "./useWizards";
import type { Overview } from "./useOverview";
import type { InspectorState } from "./useInspectorState";
import type { ProjectSession } from "./useProjectSession";
import type { SceneSync } from "./useSceneSync";
import type { useSceneHistory } from "./useSceneHistory";
import { useShallowStable, useStableCallback } from "./useStable";
import type { WorldActions } from "./useWorldActions";

/** The dialog opener without App (tests of the model): nothing opens. */
const NO_DIALOG: (target: DialogTarget) => void = () => {};

interface WorkbenchModelParams {
  bus: EditBus;
  session: Pick<ProjectSession, "info" | "message" | "busy" | "counts" | "dirtyAssets" | "onEdited" | "fail" | "onLoadStatus" | "onCountsChanged">;
  scene: Pick<SceneSync, "headers" | "selected" | "matrix" | "sceneArea" | "history" | "onSelect" | "goTo" | "onHeaderChanged" | "onAreaChanged" | "onMatrixEdited">;
  world: Pick<
    WorldActions,
    | "eventLock"
    | "selectedEvent"
    | "onInspect"
    | "onEventsLocked"
    | "onPin"
    | "onSelectBuilding"
    | "onSelectEvent"
    | "onSelectCell"
    | "onEventMoved"
    | "onEventChanged"
    | "onBuildingMoved"
    | "onBuildingChanged"
    | "onAddBuilding"
    | "onDeleteBuilding"
    | "onAddEvent"
    | "onDeleteEvent"
    | "sceneBuildings"
    | "onBuildings"
  >;
  pins: Pick<InspectorState, "pinned">;
  /** The history of the scenes shown (App owns it: the keys act on it too). */
  sceneHistory: ReturnType<typeof useSceneHistory>;
  /** The matrix and warp overviews of the project (the world explorer's data). */
  overview: Pick<Overview, "matrices" | "links" | "loaded" | "failed">;
  /** Parts were imported into a map (must keep its identity). */
  onMapImported: (map: number) => void | Promise<void>;
  /** Opens a wizard; must keep its identity (it is handed to memoised panels). */
  onOpenWizard: (request: WizardRequest) => void;
  /** Reads the project again as after a wizard and shows what was made; must keep its identity. */
  onCreated: (made: Created) => Promise<void>;
  /** The text panel's target and the way to set it (must keep its identity), owned by App. */
  text: { target: TextTarget | null };
  /** The project's scripts (the object keeps its identity until `revision` or another field changes) and the problems store. */
  scripts: ScriptsApi;
  problems: ProblemsStore;
  /** The checks of the project (a stable object). */
  checks: ChecksApi;
  /** The data lists of the project (a stable object). */
  dataStore: DataStore;
  /** What an open wizard plans (null: nothing), owned by App. */
  draft: DraftDto | null;
  /** Sets it (the dialog „Karte bei … anlegen?“ of the world map and the raster plans too); must keep its identity. */
  onDraft: (draft: DraftDto | null) => void;
  /** Opens the dialog editor of a figure (App's state); must keep its identity. None: the field „Dialog“ opens nothing (tests). */
  onOpenDialog?: (target: DialogTarget) => void;
  /** The scene's switches and the tool, owned by App. */
  view: {
    showPermissions: boolean;
    setShowPermissions: (show: boolean) => void;
    showEvents: boolean;
    setShowEvents: (show: boolean) => void;
    /** The depth of the app (the shell's). */
    depth: Depth;
    /** The active tool, the way to switch it, and the answer to a click with an adding tool; the two callbacks must keep their identity. */
    tool: Tool;
    onTool: (tool: Tool) => void;
    onTileClick: (tile: TileClick) => void;
  };
}

/**
 * What App hands the workbench, in two parts. `data` is everything that changes rarely; its
 * identity changes only when one of its fields does, so the panels reading only it are not
 * rendered again by a message. `live` is the volatile part (what the inspector shows, the cell
 * it selects, the message, the world model), where a panel subscribes to the piece it renders.
 */
export function useWorkbenchModel({ bus, session, scene, world, pins, sceneHistory, overview, onOpenWizard, onCreated, onMapImported, text, scripts, problems, checks, dataStore, draft, onDraft, view, onOpenDialog = NO_DIALOG }: WorkbenchModelParams): { data: WorkbenchData; live: WorkbenchLive } {
  // `scene.onSelect` and `scene.goTo` are plain closures over the current render (not stable), so they are wrapped here for the memoised panels.
  const onSelect = useStableCallback((h: HeaderSummary) => void scene.onSelect(h));
  const goTo = useStableCallback(scene.goTo);

  const eventFileId = scene.headers.find((h) => h.id === scene.selected)?.event_file_id ?? null;
  const { pinned } = pins;
  const buildingMap = pinned.kind === "building" ? pinned.map : null;
  const buildingIndex = pinned.kind === "building" ? pinned.index : null;
  const selectedBuilding = useMemo(() => (buildingMap !== null && buildingIndex !== null ? { map: buildingMap, index: buildingIndex } : null), [buildingMap, buildingIndex]);

  const data = useShallowStable<WorkbenchData>({
    info: session.info,
    headers: scene.headers,
    matrices: overview.matrices,
    warpLinks: overview.links,
    matricesLoaded: overview.loaded,
    matricesFailed: overview.failed,
    onOpenWizard,
    onCreated,
    selected: scene.selected,
    onSelect,
    goTo,
    sceneBack: sceneHistory.back,
    sceneForward: sceneHistory.forward,
    canBack: sceneHistory.canBack,
    canForward: sceneHistory.canForward,
    bus,
    matrix: scene.matrix,
    areaDataId: scene.sceneArea,
    showPermissions: view.showPermissions,
    onShowPermissions: view.setShowPermissions,
    eventFileId,
    showEvents: view.showEvents,
    onShowEvents: view.setShowEvents,
    paintEnabled: !session.busy,
    depth: view.depth,
    tool: view.tool,
    onTool: view.onTool,
    onTileClick: view.onTileClick,
    buildings: world.sceneBuildings,
    onBuildings: world.onBuildings,
    history: scene.history,
    onEdited: session.onEdited,
    onError: session.fail,
    onLoadStatus: session.onLoadStatus,
    onInspect: world.onInspect,
    memberCounts: session.counts,
    dirtyAssets: session.dirtyAssets,
    onHeaderChanged: scene.onHeaderChanged,
    onAreaChanged: scene.onAreaChanged,
    onMatrixEdited: scene.onMatrixEdited,
    onCountsChanged: session.onCountsChanged,
    onMapImported,
    selectedBuilding,
    onPin: world.onPin,
    onSelectBuilding: world.onSelectBuilding,
    onBuildingMoved: world.onBuildingMoved,
    onBuildingChanged: world.onBuildingChanged,
    onAddBuilding: world.onAddBuilding,
    onDeleteBuilding: world.onDeleteBuilding,
    selectedEvent: world.selectedEvent,
    onSelectEvent: world.onSelectEvent,
    onSelectCell: world.onSelectCell,
    onEventMoved: world.onEventMoved,
    eventLock: world.eventLock,
    onEventsLocked: world.onEventsLocked,
    onEventChanged: world.onEventChanged,
    onAddEvent: world.onAddEvent,
    onDeleteEvent: world.onDeleteEvent,
    textTarget: text.target,
    scripts,
    problems,
    checks,
    dataStore,
    draft,
    onDraft,
    onOpenDialog,
  });

  // The inspector shows the selection; the hover goes to the status line and is not part of the model.
  const inspector = pinned;
  const cell = inspector.kind === "matrixCell" ? inspector : null;
  const [cellMatrix, cellX, cellY] = [cell?.matrix, cell?.x, cell?.y];
  const matrixSelection = useMemo<MatrixSelection | null>(
    () => (cellMatrix !== undefined && cellX !== undefined && cellY !== undefined ? { matrix: cellMatrix, x: cellX, y: cellY } : null),
    [cellMatrix, cellX, cellY],
  );

  // The world model, built once for the workbench: the cells of the world maps are read again whenever matrix data moved (which
  // renders App only when they changed), the project file once per project. Until it is read, the places name the headers.
  const worldModel = useWorldModel({
    info: session.info,
    headers: scene.headers,
    matrices: overview.matrices,
    links: overview.links,
    loaded: overview.loaded,
    revision: 0,
    onError: session.fail,
  });
  useBusEvent(bus, "matrixData", worldModel.reread);
  const places = useMemo(() => buildPlaces(scene.headers, overview.matrices), [scene.headers, overview.matrices]);
  const { names } = worldModel;
  const identify = useCallback((header: HeaderSummary) => headerIdentity(places, header, names), [places, names]);
  const shownWorld = useMemo<WorkbenchWorld>(() => ({ ...worldModel, identify }), [worldModel, identify]);

  const live = useShallowStable<WorkbenchLive>({ inspector, matrixSelection, message: session.message, world: shownWorld });
  return { data, live };
}
