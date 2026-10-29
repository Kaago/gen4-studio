import { countKey, targetIndex } from "../api";
import type { OriginalStatus, ResetTarget } from "../api";
import type { MenuItemSpec } from "../ui/primitives";

/** The entry's label where one target stands alone; where two stand side by side the caller names which one. */
export const RESET_LABEL = "Auf Original zurücksetzen…";

/**
 * Why resetting `target` is not possible, as a menu entry's quiet detail or a locked button's reason: the base ROM is read, missing, or does
 * not have the entry. Null when it can be used.
 */
export function whyNot(target: ResetTarget, status: OriginalStatus | null): string | null {
  if (status === null) return "Wird gelesen …";
  if (status.state === "unavailable") return "Basis-ROM fehlt";
  // A kind the base has no archive for counts 0. A message itself is checked by the plan.
  if (targetIndex(target) >= (status.counts[countKey(target)] ?? 0)) return "Kein Original";
  return null;
}

/**
 * The menu entry „Auf Original zurücksetzen…“ for `target` (the actions menu of a head, a row's context menu). Disabled, with the reason
 * as its detail, while the status is read, without a found base ROM and for an entry the base does not have (a new one).
 */
export function resetEntry(target: ResetTarget, status: OriginalStatus | null, open: (t: ResetTarget) => void, label: string = RESET_LABEL): MenuItemSpec {
  const why = whyNot(target, status);
  return { label, icon: "undo", onSelect: () => open(target), disabled: why !== null, ...(why !== null && { detail: why }) };
}
