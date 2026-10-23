import type { AssetId, AssetKind } from "../../api";

/** Whether the asset `kind` number `index` has unsaved changes (the inspector badges the record it shows). */
export function isDirty(dirty: readonly AssetId[], kind: AssetKind, index: number): boolean {
  return dirty.some((a) => a.kind === kind && a.index === index);
}
