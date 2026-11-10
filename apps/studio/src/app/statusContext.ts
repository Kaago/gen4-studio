import type { HeaderSummary, WarpDto } from "../api";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { KIND_SINGULAR } from "../workbench/context";
import type { InspectorModel, SceneHover } from "../workbench/Inspector";
import { id4, termFor } from "../workbench/terms";
import { eventTitle } from "../world/events";
import { tileSummary } from "../world/hover";

/**
 * The marker under the pointer: a door with where it leads, named as the world tree names it („Tür 1 → Haus des Rivalen“, with the
 * number the inspector's title gives the warp), any other event by its title.
 */
function markerSummary(hover: Extract<SceneHover, { kind: "eventHover" }>, depth: Depth): string {
  if (hover.eventKind !== "warp") return eventTitle(hover.eventKind, hover.data, depth);
  const dest = hover.destName || `${termFor("header", depth)} ${id4((hover.data as WarpDto).dest_header_id)}`;
  return `${KIND_SINGULAR.warp} ${hover.index} → ${dest}`;
}

/**
 * The middle of the status line: the tile or marker under the pointer, else where the user is ("Zweiblattdorf, Karte 0412 (3, 27)", an empty
 * cell „Zweiblattdorf, leere Zelle (3, 27)“; Tüfteln adds the matrix: „Zweiblattdorf, Raster 0, Zelle 3, 27“). Empty when nothing is hovered and no header is selected.
 */
export function statusContext(pinned: InspectorModel, hover: SceneHover | null, header: HeaderSummary | undefined, depth: Depth, cellMap?: number | null): string {
  if (hover?.kind === "tile") return tileSummary(hover.map, hover.x, hover.y, hover.info, depth, hover.units);
  if (hover?.kind === "eventHover") return markerSummary(hover, depth);
  if (!header) return "";
  const parts = [header.location_name || header.internal_name];
  if (depthAtLeast(depth, "tuefteln")) parts.push(t("app:statusContext.matrix", { id: header.matrix_id }));
  if (pinned.kind === "matrixCell") {
    const at = `${pinned.x}, ${pinned.y}`;
    // Bauen names the cell by its map; `cellMap` is null for an empty cell and missing when the matrix is not at hand.
    parts.push(
      depthAtLeast(depth, "tuefteln")
        ? t("app:statusContext.cell", { at })
        : cellMap === null
          ? t("app:statusContext.emptyCell", { at })
          : cellMap === undefined
            ? t("app:statusContext.someMap", { at })
            : t("app:statusContext.map", { id: id4(cellMap), at }),
    );
  }
  return parts.join(", ");
}

/** The middle of the status line in a data area: the selected member and its four-digit number ("Staravia, 0397"); empty without one. */
export function dataStatus(selection: { label: string; tech: string } | null): string {
  return selection === null ? "" : `${selection.label}, ${selection.tech}`;
}
