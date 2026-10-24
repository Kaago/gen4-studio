import type { HeaderSummary } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { tileSummary } from "../world/hover";

/**
 * The middle of the status line: the tile under the pointer, else where the user is ("Zweiblattdorf, Raster 0, Zelle 3, 27").
 * Empty when no header is selected.
 */
export function statusContext(shown: InspectorModel, header: HeaderSummary | undefined): string {
  if (shown.kind === "tile") return tileSummary(shown.map, shown.x, shown.y, shown.info);
  if (!header) return "";
  const parts = [header.location_name || header.internal_name, `Raster ${header.matrix_id}`];
  if (shown.kind === "matrixCell") parts.push(`Zelle ${shown.x}, ${shown.y}`);
  return parts.join(", ");
}

/** The middle of the status line in a data area: the selected member and its four-digit number ("Staravia, 0397"); empty without one. */
export function dataStatus(selection: { label: string; tech: string } | null): string {
  return selection === null ? "" : `${selection.label}, ${selection.tech}`;
}
