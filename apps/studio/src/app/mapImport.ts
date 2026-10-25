import type { EditBus } from "../bus/editBus";

export interface MapImportDeps {
  bus: Pick<EditBus, "emit">;
  /** Drops the map's cached pictures (the thumbnail store). */
  invalidate: (map: number) => void;
  /** Counts the unsaved assets again. */
  refreshDirty: () => Promise<void>;
}

/**
 * After a parts import into `map`: its pictures go, the scene refetches the map (overlay colours
 * and buildings, for an imported collision) and reloads only that map's model, then the dirty
 * count catches up. The same refreshes an undo or redo of the import gets (see `reloadPlan`).
 */
export async function afterMapImport(map: number, { bus, invalidate, refreshDirty }: MapImportDeps): Promise<void> {
  invalidate(map);
  bus.emit({ topic: "mapRefetch", map });
  bus.emit({ topic: "mapModelReload", map });
  await refreshDirty();
}
