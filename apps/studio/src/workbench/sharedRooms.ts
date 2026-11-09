import { t } from "../i18n";
import de from "../i18n/locales/de/workbench.json";

/** „Geteilt mit 17 Räumen“: the badge of a room whose maps `shared` other rooms use too. */
export function sharedText(shared: number): string {
  return t("workbench:sharedRooms.shared", { count: shared });
}

/** What the badge „Geteilt“ explains on hover, in the language shown. */
export const sharedHint = (): string => t("workbench:sharedRooms.hint");

/**
 * The German hint, for the gallery (which stays German) and for callers not yet on `sharedHint()`; read from the German catalog, so the
 * words live in one place. Remove once every caller uses `sharedHint()`.
 */
export const SHARED_HINT: string = de.sharedRooms.hint;
