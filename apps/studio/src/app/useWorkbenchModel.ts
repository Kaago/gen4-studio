import { useMemo } from "react";
import type { HeaderSummary } from "../api";
import type { Depth, Tool } from "../ui/frame/shellState";
import type { TileClick } from "../world/scene/types";
import type { EditBus } from "../bus/editBus";
import type { DataStore } from "../data/dataStore";
import type { ProblemsStore } from "../workbench/problems/problems";
import type { ScriptsApi } from "../workbench/script/useScripts";
import type { TextTarget } from "../workbench/text/textModel";
import type { MatrixSelection, WizardRequest, WorkbenchData, WorkbenchLive } from "../workbench/WorkbenchContext";
import type { Overview } from "./useOverview";
import type { InspectorState } from "./useInspectorState";
import type { ProjectSession } from "./useProjectSession";
import type { SceneSync } from "./useSceneSync";
import { useShallowStable, useStableCallback } from "./useStable";
import type { WorldActions } from "./useWorldActions";

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
    | "onSelectBuilding"
    | "onSelectEvent"
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
  pins: Pick<InspectorState, "pinned" | "inspector">;
  /** The matrix and warp overviews of the project (the world explorer's data). */
  overview: Pick<Overview, "matrices" | "links" | "loaded" | "failed">;
  /** Opens a wizard; must keep its identity (it is handed to memoised panels). */
  onOpenWizard: (request: WizardRequest) => void;
  /** The text panel's target and the way to set it (must keep its identity), owned by App. */
  text: { target: TextTarget | null };
  /** The project's scripts (the object keeps its identity until `revision` or another field changes) and the problems store. */
  scripts: ScriptsApi;
  problems: ProblemsStore;
  /** The data lists of the project (a stable object). */
  dataStore: DataStore;
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
 * rendered again by a hover or a message. `live` is the volatile part (the inspector's hover,
 * the cell it selects, the message), where a panel subscribes to the piece it renders.
 */
export function useWorkbenchModel({ bus, session, scene, world, pins, overview, onOpenWizard, text, scripts, problems, dataStore, view }: WorkbenchModelParams): { data: WorkbenchData; live: WorkbenchLive } {
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
    selected: scene.selected,
    onSelect,
    goTo,
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
    selectedBuilding,
    onSelectBuilding: world.onSelectBuilding,
    onBuildingMoved: world.onBuildingMoved,
    onBuildingChanged: world.onBuildingChanged,
    onAddBuilding: world.onAddBuilding,
    onDeleteBuilding: world.onDeleteBuilding,
    selectedEvent: world.selectedEvent,
    onSelectEvent: world.onSelectEvent,
    onEventMoved: world.onEventMoved,
    eventLock: world.eventLock,
    onEventsLocked: world.onEventsLocked,
    onEventChanged: world.onEventChanged,
    onAddEvent: world.onAddEvent,
    onDeleteEvent: world.onDeleteEvent,
    textTarget: text.target,
    scripts,
    problems,
    dataStore,
  });

  const { inspector } = pins;
  const cell = inspector.kind === "matrixCell" ? inspector : null;
  const [cellMatrix, cellX, cellY] = [cell?.matrix, cell?.x, cell?.y];
  const matrixSelection = useMemo<MatrixSelection | null>(
    () => (cellMatrix !== undefined && cellX !== undefined && cellY !== undefined ? { matrix: cellMatrix, x: cellX, y: cellY } : null),
    [cellMatrix, cellX, cellY],
  );
  const live = useShallowStable<WorkbenchLive>({ inspector, matrixSelection, message: session.message });
  return { data, live };
}
