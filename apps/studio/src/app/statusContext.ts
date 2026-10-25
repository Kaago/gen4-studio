import type { HeaderSummary, WarpDto } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { KIND_SINGULAR } from "../workbench/context";
import type { InspectorModel, SceneHover } from "../workbench/Inspector";
import { id4, termFor } from "../workbench/terms";
import { eventTitle } from "../world/events";
import { tileSummary } from "../world/hover";

/** The marker under the pointer: a door with where it leads ("Tür 2 → Haus des Rivalen"), any other event by its title. */
function markerSummary(hover: Extract<SceneHover, { kind: "eventHover" }>, depth: Depth): string {
  if (hover.eventKind !== "warp") return eventTitle(hover.eventKind, hover.data, depth);
  const dest = hover.destName || `${termFor("header", depth)} ${id4((hover.data as WarpDto).dest_header_id)}`;
  return `${KIND_SINGULAR.warp} ${hover.index + 1} → ${dest}`;
}

/**
 * The middle of the status line: the tile or marker under the pointer, else where the user is ("Zweiblattdorf, Zelle 3, 27";
 * Tüfteln adds the matrix: "Zweiblattdorf, Raster 0, Zelle 3, 27"). Empty when nothing is hovered and no header is selected.
 */
export function statusContext(pinned: InspectorModel, hover: SceneHover | null, header: HeaderSummary | undefined, depth: Depth): string {
  if (hover?.kind === "tile") return tileSummary(hover.map, hover.x, hover.y, hover.info);
  if (hover?.kind === "eventHover") return markerSummary(hover, depth);
  if (!header) return "";
  const parts = [header.location_name || header.internal_name];
  if (depth === "tuefteln") parts.push(`Raster ${header.matrix_id}`);
  if (pinned.kind === "matrixCell") parts.push(`Zelle ${pinned.x}, ${pinned.y}`);
  return parts.join(", ");
}

/** The middle of the status line in a data area: the selected member and its four-digit number ("Staravia, 0397"); empty without one. */
export function dataStatus(selection: { label: string; tech: string } | null): string {
  return selection === null ? "" : `${selection.label}, ${selection.tech}`;
}
