import type { HeaderSummary } from "../api";
import type { HeaderIdentity } from "../workbench/headerIdentity";
import { UNNAMED_PLACE } from "../workbench/places";
import { placeNameOf } from "../workbench/worldModel";

/**
 * The start town as the world names it in Bauen, for Technik's hint below the preview picture („… das Kartenbild von Zweiblattdorf.“):
 * an area's name, or the place of a header outside the world maps. A room gives no name, and neither does a header without a place name:
 * the world calls such an area „Ohne Ortsnamen“, which is no town to name (the hint then says „der Startstadt“).
 */
export function startTownName(header: HeaderSummary, identity: HeaderIdentity): string | null {
  if (identity.kind === "room" || placeNameOf(header) === "" || identity.name === UNNAMED_PLACE) return null;
  return identity.name;
}
