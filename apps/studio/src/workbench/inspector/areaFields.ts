import type { AreaDataDto } from "../../api";
import { t } from "../../i18n";
import type { FieldSpec } from "./fields";
import type { RecordFields } from "./headerCommit";

const u8 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u8" });
const u16 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u16" });

// Built when `areaFieldSpecs` is called, so the labels are in the language that is shown then.
const buildingsTileset = (): FieldSpec => ({ key: "buildings_tileset", label: t("workbench:areaFields.buildingsTileset"), kind: "pick", pick: "buildingTexture" });
const mapTileset = (): FieldSpec => ({ key: "map_tileset", label: t("workbench:areaFields.mapTileset"), kind: "pick", pick: "mapTexture" });

/** HGSS: whether the area is inside or outside (the game only uses these two values). */
export const areaTypes = (): { value: number; label: string }[] => [
  { value: 0, label: t("workbench:areaFields.inside") },
  { value: 1, label: t("workbench:areaFields.outside") },
];

/**
 * The editable fields of an area data record. The DPPT field the codec calls `unknown` is left
 * out: like the header's unknown fields it is kept verbatim, not edited.
 */
export function areaFieldSpecs(family: AreaDataDto["family"]): FieldSpec[] {
  return family === "Hgss"
    ? [
        mapTileset(),
        buildingsTileset(),
        u16("dynamic_texture_type", t("workbench:areaFields.dynamicTexture")),
        { key: "area_type", label: t("workbench:areaFields.areaType"), kind: "select", options: areaTypes() },
        u8("light_type", t("workbench:areaFields.lightType")),
      ]
    : [mapTileset(), buildingsTileset(), u16("light_type", t("workbench:areaFields.lightType"))];
}

export function areaValue(area: AreaDataDto, key: string): number {
  return Number((area as unknown as Record<string, number | string>)[key]);
}

/** A copy of `area` with `key` set. */
export function withAreaField(area: AreaDataDto, key: string, value: number): AreaDataDto {
  return { ...area, [key]: value } as AreaDataDto;
}

export const AREA_FIELDS: RecordFields<AreaDataDto> = { value: areaValue, with: withAreaField };

/**
 * What the scene's models depend on; a change of it reloads them. Besides the two tilesets, the
 * HGSS area type does: it switches the building models between the interior and exterior archives.
 */
export function tilesetKey(area: AreaDataDto): string {
  const key = `${area.map_tileset}:${area.buildings_tileset}`;
  return area.family === "Hgss" ? `${key}:${area.area_type}` : key;
}
