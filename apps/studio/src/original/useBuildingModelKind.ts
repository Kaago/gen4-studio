import { useEffect, useState } from "react";
import { getAreaData } from "../api";
import type { AreaDataDto } from "../api";

/** The reset target kinds of a building model: the exterior archive, or HeartGold's and SoulSilver's interior one. */
export type BuildingModelKind = "building_model" | "interior_building_model";

/**
 * The archive the scene takes a building's model from, as the backend chooses it (`building_inputs`): HeartGold and SoulSilver take the
 * models of an interior area (area type 0) from the interior archive, everything else from the exterior one.
 */
export function buildingModelKind(area: AreaDataDto): BuildingModelKind {
  return area.family === "Hgss" && area.area_type === 0 ? "interior_building_model" : "building_model";
}

/**
 * The building model archive of area data `areaDataId` (unsaved edits included, as the scene reads it), read again when `revision`
 * changes; null only while it is read. Without area data, and when it cannot be read, the exterior archive, as the backend takes it then.
 */
export function useBuildingModelKind(areaDataId: number | null, revision: number): BuildingModelKind | null {
  const [read, setRead] = useState<{ area: number; kind: BuildingModelKind } | null>(null);
  useEffect(() => {
    if (areaDataId === null) return;
    let current = true;
    getAreaData(areaDataId).then(
      (area) => current && setRead({ area: areaDataId, kind: buildingModelKind(area) }),
      () => current && setRead({ area: areaDataId, kind: "building_model" }),
    );
    return () => {
      current = false;
    };
  }, [areaDataId, revision]);
  if (areaDataId === null) return "building_model";
  return read !== null && read.area === areaDataId ? read.kind : null;
}
