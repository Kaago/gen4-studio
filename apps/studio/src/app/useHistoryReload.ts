import { getMatrix, undo } from "../api";
import type { AssetId } from "../api";
import { historyPlan } from "../reload";
import type { ReloadAction } from "../reload";
import { thumbnails } from "../world/thumbnailStore";
import { pinAfterHistory } from "../workbench/pin";
import type { EditBus } from "../bus/editBus";
import { dataChangedOf } from "../data/dataBus";
import type { InspectorState } from "./useInspectorState";
import type { ProjectSession } from "./useProjectSession";
import type { SceneSync } from "./useSceneSync";

interface HistoryReloadParams {
  bus: EditBus;
  session: ProjectSession;
  scene: SceneSync;
  pins: InspectorState;
}

/**
 * Undo and redo, and everything that has to be reloaded after the backend restored assets (see
 * `historyPlan`) or the user resolved a stale file: the scene, the editors and the pinned selection.
 */
export function useHistoryReload({
  bus,
  session,
  scene,
  pins,
}: HistoryReloadParams) {
  const { setPinned } = pins;
  const { fail, refreshDirty } = session;

  /** Reloads the shown matrix (a new object makes the scene refetch all its maps), the headers and the dirty count. */
  async function refreshAfterStale(matrixIndex: number | null) {
    try {
      if (matrixIndex !== null) scene.setMatrix(await getMatrix(matrixIndex));
      bus.emit({ topic: "headerTable" });
      bus.emit({ topic: "matrixData" });
      bus.emit({ topic: "areaData" });
      bus.emit({ topic: "buildingData" });
      bus.emit({ topic: "eventData" });
      bus.emit({ topic: "textData", archive: null });
      for (const kind of [
        "species",
        "learnset",
        "evolution",
        "move",
        "item",
        "encounter",
        "trainer",
      ] as const)
        bus.emit({ topic: "dataChanged", kind, index: null });
      // The reload may have re-ordered the buildings or events: a pinned one is bound by index, so let go of it.
      setPinned((p) => pinAfterHistory(p, scene.selectedRef.current));
      await scene.syncHeaders();
      await scene.noteSceneTileset(true);
    } finally {
      // The dirty count must follow the reload/accept even when refetching the matrix fails.
      await refreshDirty();
    }
  }
  // A stale resolution (see `fail`) reloads whatever matrix is shown when it is chosen.
  session.staleRefresh.current = () =>
    refreshAfterStale(scene.matrixRef.current?.index ?? null);

  /** Carries out one action of a `reloadPlan`. */
  async function reloadAction(a: ReloadAction) {
    switch (a.kind) {
      case "unpin":
        // A pinned building or event is bound by index, which undo and redo can shift (undoing a
        // delete re-inserts one in front), so it would silently become another one: let go of it.
        setPinned((p) => pinAfterHistory(p, scene.selectedRef.current));
        break;
      case "buildings":
        bus.emit({ topic: "buildingData" });
        break;
      case "map":
        bus.emit({ topic: "mapRefetch", map: a.map });
        break;
      case "thumbnail":
        thumbnails.invalidate(a.map);
        break;
      case "mapModel":
        bus.emit({ topic: "modelsReload" });
        break;
      case "matrixPanel":
        bus.emit({ topic: "matrixData" });
        break;
      case "matrix":
        await scene.refreshSceneMatrix(scene.matrixRef.current?.index);
        break;
      case "eventsPanel":
        bus.emit({ topic: "eventData" });
        break;
      case "events":
        bus.emit({ topic: "eventsRefetch", file: a.file });
        break;
      case "areaPanel":
        bus.emit({ topic: "areaData" });
        break;
      case "models":
        await scene.noteSceneTileset(true);
        break;
      case "textPanel":
        bus.emit({ topic: "textData", archive: a.archive });
        break;
      case "headers":
        bus.emit({ topic: "headerTable" });
        // An encounter file's users are the headers that name it: its open document reads them anew.
        bus.emit({ topic: "dataChanged", kind: "encounter", index: null });
        await scene.syncHeaders();
        break;
      case "dataPanel": {
        // The data areas read their lists and editors again; nothing in the world shows data members.
        const event = dataChangedOf(a.asset);
        if (event) bus.emit(event);
        break;
      }
    }
  }

  /** Reloads what the restored assets affect (see `historyPlan`), nothing else. */
  async function reloadAfterHistory(assets: AssetId[]) {
    const shown = scene.matrixRef.current;
    const plan = historyPlan(assets, {
      matrix: shown?.index ?? null,
      maps: shown?.maps ?? [],
      area: scene.targetArea(),
      eventFile:
        scene.headersRef.current.find((h) => h.id === scene.selectedRef.current)
          ?.event_file_id ?? null,
    });
    // One failing reload must not keep the others from running; the first failure is reported.
    let failure: unknown = null;
    for (const action of plan) {
      try {
        await reloadAction(action);
      } catch (e) {
        failure ??= e;
      }
    }
    if (failure !== null) throw failure;
  }

  /** One undo or redo with its reloads. Resolves to false when it did not go through (no project, busy, or an error that was reported). */
  async function step(action: typeof undo): Promise<boolean> {
    if (!session.info || session.busy) return false;
    let done = true;
    try {
      const assets = await action();
      if (assets.length > 0) {
        scene.noteHistoryMoved();
        await reloadAfterHistory(assets);
      }
    } catch (e) {
      done = false;
      fail(e);
    }
    await refreshDirty();
    return done;
  }

  return { step };
}
