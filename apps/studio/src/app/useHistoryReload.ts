import { getMatrix, undo } from "../api";
import type { AssetId } from "../api";
import { reloadPlan } from "../reload";
import type { ReloadAction } from "../reload";
import { pinAfterHistory } from "../workbench/pin";
import type { InspectorState } from "./useInspectorState";
import type { ProjectSession } from "./useProjectSession";
import type { SceneSync } from "./useSceneSync";
import type { WorldActions } from "./useWorldActions";

interface HistoryReloadParams {
  session: ProjectSession;
  scene: SceneSync;
  world: WorldActions;
  pins: InspectorState;
}

/**
 * Undo and redo, and everything that has to be reloaded after the backend restored an asset (see
 * `reloadPlan`) or the user resolved a stale file: the scene, the editors and the pinned selection.
 */
export function useHistoryReload({ session, scene, world, pins }: HistoryReloadParams) {
  const { setPinned } = pins;
  const { fail, refreshDirty } = session;

  /** Reloads the shown matrix (a new object makes the scene refetch all its maps), the headers and the dirty count. */
  async function refreshAfterStale(matrixIndex: number | null) {
    try {
      if (matrixIndex !== null) scene.setMatrix(await getMatrix(matrixIndex));
      scene.bumpHeaders();
      scene.bumpMatrix();
      scene.bumpArea();
      world.bumpBuildings();
      world.bumpEvents();
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
  session.staleRefresh.current = () => refreshAfterStale(scene.matrixRef.current?.index ?? null);

  /** Carries out one action of a `reloadPlan`. */
  async function reloadAction(a: ReloadAction) {
    switch (a.kind) {
      case "unpin":
        // A pinned building or event is bound by index, which undo and redo can shift (undoing a
        // delete re-inserts one in front), so it would silently become another one: let go of it.
        setPinned((p) => pinAfterHistory(p, scene.selectedRef.current));
        break;
      case "buildings":
        world.bumpBuildings();
        break;
      case "map":
        scene.requestMapRefresh(a.map);
        break;
      case "matrixPanel":
        scene.bumpMatrix();
        break;
      case "matrix":
        await scene.refreshSceneMatrix(scene.matrixRef.current?.index);
        break;
      case "eventsPanel":
        world.bumpEvents();
        break;
      case "events":
        world.requestEventRefresh(a.file);
        break;
      case "areaPanel":
        scene.bumpArea();
        break;
      case "models":
        await scene.noteSceneTileset(true);
        break;
      case "headers":
        scene.bumpHeaders();
        await scene.syncHeaders();
        break;
    }
  }

  /** Reloads what the restored asset affects (see `reloadPlan`), nothing else. */
  async function reloadAfterHistory(asset: AssetId) {
    const shown = scene.matrixRef.current;
    const plan = reloadPlan(asset, {
      matrix: shown?.index ?? null,
      maps: shown?.maps ?? [],
      area: scene.targetArea(),
      eventFile: scene.headersRef.current.find((h) => h.id === scene.selectedRef.current)?.event_file_id ?? null,
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

  async function step(action: typeof undo) {
    if (!session.info || session.busy) return;
    try {
      const asset = await action();
      if (asset) {
        scene.noteHistoryMoved();
        await reloadAfterHistory(asset);
      }
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  return { step };
}
