import type { AreaDataDto } from "../../api";
import type { FieldSpec } from "./fields";
import type { RecordFields } from "./headerCommit";

const u8 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u8" });
const u16 = (key: string, label: string): FieldSpec => ({ key, label, kind: "u16" });

const buildingsTileset: FieldSpec = { key: "buildings_tileset", label: "Gebäude-Tileset", kind: "pick", pick: "buildingTexture" };
const mapTileset: FieldSpec = { key: "map_tileset", label: "Karten-Tileset", kind: "pick", pick: "mapTexture" };

/** HGSS: whether the area is inside or outside (the game only uses these two values). */
export const AREA_TYPES = [
  { value: 0, label: "Innen (0)" },
  { value: 1, label: "Außen (1)" },
];

/**
 * The editable fields of an area data record. The DPPT field the codec calls `unknown` is left
 * out: like the header's unknown fields it is kept verbatim, not edited.
 */
export function areaFieldSpecs(family: AreaDataDto["family"]): FieldSpec[] {
  return family === "Hgss"
    ? [
        mapTileset,
        buildingsTileset,
        u16("dynamic_texture_type", "Dynamische Textur"),
        { key: "area_type", label: "Gebietstyp", kind: "select", options: AREA_TYPES },
        u8("light_type", "Lichttyp"),
      ]
    : [mapTileset, buildingsTileset, u16("light_type", "Lichttyp")];
}

export function areaValue(area: AreaDataDto, key: string): number {
  return Number((area as unknown as Record<string, number | string>)[key]);
}

/** A copy of `area` with `key` set. */
export function withAreaField(area: AreaDataDto, key: string, value: number): AreaDataDto {
  return { ...area, [key]: value } as AreaDataDto;
}

export const AREA_FIELDS: RecordFields<AreaDataDto> = { value: areaValue, with: withAreaField };

/** What the scene's models depend on; a change of it re-textures them. */
export function tilesetKey(area: AreaDataDto): string {
  return `${area.map_tileset}:${area.buildings_tileset}`;
}
