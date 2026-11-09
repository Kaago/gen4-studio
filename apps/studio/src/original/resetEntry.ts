import { countKey, targetIndex } from "../api";
import type { OriginalStatus, ResetTarget } from "../api";
import { t } from "../i18n";
import deOriginal from "../i18n/locales/de/original.json";
import type { MenuItemSpec } from "../ui/primitives";

/** The entry's label where one target stands alone; where two stand side by side the caller names which one. */
export function resetLabel(): string {
  return t("original:resetEntry.label");
}

/**
 * The German label, as a constant for the places of other folders that still read `RESET_LABEL` (Technik › Änderungen: `ChangesSurface`,
 * `EntityDiff`; the script panel). Taken from the German catalog, so it stays German until they call `resetLabel()`.
 * @deprecated Call `resetLabel()`.
 */
export const RESET_LABEL: string = deOriginal.resetEntry.label;

/**
 * Why resetting `target` is not possible, as a menu entry's quiet detail or a locked button's reason: the base ROM is read, missing, or does
 * not have the entry. Null when it can be used.
 */
export function whyNot(target: ResetTarget, status: OriginalStatus | null): string | null {
  if (status === null) return t("original:resetEntry.reading");
  if (status.state === "unavailable") return t("original:resetEntry.baseMissing");
  // A kind the base has no archive for counts 0. A message itself is checked by the plan.
  if (targetIndex(target) >= (status.counts[countKey(target)] ?? 0)) return t("original:resetEntry.noOriginal");
  return null;
}

/**
 * The menu entry „Auf Original zurücksetzen…“ for `target` (the actions menu of a head, a row's context menu). Disabled, with the reason
 * as its detail, while the status is read, without a found base ROM and for an entry the base does not have (a new one).
 */
export function resetEntry(target: ResetTarget, status: OriginalStatus | null, open: (t: ResetTarget) => void, label: string = resetLabel()): MenuItemSpec {
  const why = whyNot(target, status);
  return { label, icon: "undo", onSelect: () => open(target), disabled: why !== null, ...(why !== null && { detail: why }) };
}
