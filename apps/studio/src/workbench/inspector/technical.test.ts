import { describe, expect, it } from "vitest";
import type { AreaDataDto, BuildingDto, MapHeader } from "../../api";
import type { InspectorModel } from "../Inspector";
import { areaFieldSpecs } from "./areaFields";
import { eventFieldSpecs } from "./eventFields";
import { headerFieldSpecs } from "./fields";
import { assetLine, filePath, technicalRows } from "./technical";

const pt = {
  Pt: {
    area_data_id: 3,
    matrix_id: 255,
    script_file_id: 1,
    level_script_id: 2,
    text_archive_id: 412,
    music_day_id: 1000,
    music_night_id: 1001,
    wild_pokemon: 0,
    event_file_id: 411,
    location_name: 7,
    area_icon: 1,
    weather_id: 0,
    camera_angle_id: 4,
    location_specifier: 2,
    battle_background: 9,
    flags: 16,
  },
} as unknown as MapHeader;

describe("technicalRows", () => {
  it("lists_every_field_of_a_platinum_header_with_decimal_and_uppercase_hex", () => {
    const rows = technicalRows({ kind: "header", header: pt });
    expect(rows.map((r) => r.key)).toEqual(headerFieldSpecs("Pt").map((s) => s.key));
    expect(rows.find((r) => r.key === "matrix_id")).toEqual({ key: "matrix_id", dec: 255, hex: "0xFF" });
    expect(rows.find((r) => r.key === "event_file_id")).toEqual({ key: "event_file_id", dec: 411, hex: "0x19B" });
    expect(rows.find((r) => r.key === "weather_id")).toEqual({ key: "weather_id", dec: 0, hex: "0x0" });
  });

  it("writes_a_switch_field_as_0_or_1", () => {
    const hgss = { HGSS: { kanto_flag: true } } as unknown as MapHeader;
    const rows = technicalRows({ kind: "header", header: hgss });
    expect(rows.find((r) => r.key === "kanto_flag")).toEqual({ key: "kanto_flag", dec: 1, hex: "0x1" });
  });

  it("lists_every_field_of_an_area_data_record", () => {
    const area: AreaDataDto = { family: "Hgss", buildings_tileset: 17, map_tileset: 2, dynamic_texture_type: 65535, area_type: 1, light_type: 3 };
    const rows = technicalRows({ kind: "area", area });
    expect(rows.map((r) => r.key)).toEqual(areaFieldSpecs("Hgss").map((s) => s.key));
    expect(rows.find((r) => r.key === "dynamic_texture_type")).toEqual({ key: "dynamic_texture_type", dec: 65535, hex: "0xFFFF" });
  });

  it("lists_the_fields_of_an_event_and_the_ones_the_form_does_not_edit", () => {
    const warp = { index: 0, x: 8, z: 4, dest_header_id: 414, dest_warp_id: 1, height: 0 };
    const rows = technicalRows({ kind: "event", eventKind: "warp", event: warp, family: "Pt" });
    expect(rows.map((r) => r.key)).toEqual([...eventFieldSpecs("warp", "Pt").map((s) => s.key), "dest_header_id"]);
    expect(rows.find((r) => r.key === "dest_header_id")).toEqual({ key: "dest_header_id", dec: 414, hex: "0x19E" });
    const object = { index: 0, x: 3, y: 2, z: 5, local_id: 1, graphics_id: 90, movement_type: 2, dir: 1, script: 2000 };
    const keys = technicalRows({ kind: "event", eventKind: "object", event: object as never, family: "Pt" }).map((r) => r.key);
    expect(keys).toContain("y");
    expect(keys).toContain("script");
  });

  it("gives_a_negative_or_fractional_value_no_hex", () => {
    const building: BuildingDto = { model_id: 12, x: 1.5, y: 0, z: -2, sx: 1, sy: 1, sz: 1, rotation: ["0", "0", "0"] };
    const rows = technicalRows({ kind: "building", building });
    expect(rows.map((r) => r.key)).toEqual(["model_id", "x", "y", "z", "sx", "sy", "sz"]);
    expect(rows[0]).toEqual({ key: "model_id", dec: 12, hex: "0xC" });
    expect(rows[1]).toEqual({ key: "x", dec: 1.5, hex: null });
    expect(rows[3]).toEqual({ key: "z", dec: -2, hex: null });
  });

  it("leaves_out_what_a_matrix_cell_does_not_have", () => {
    expect(technicalRows({ kind: "matrixCell", map: 187, header: null, altitude: 4 })).toEqual([
      { key: "map", dec: 187, hex: "0xBB" },
      { key: "altitude", dec: 4, hex: "0x4" },
    ]);
    expect(technicalRows({ kind: "matrixCell", map: null, header: 0, altitude: null })).toEqual([{ key: "header", dec: 0, hex: "0x0" }]);
  });
});

describe("assetLine and filePath", () => {
  const event: InspectorModel = { kind: "event", eventFile: 411, eventKind: "object", index: 3 };
  const building: InspectorModel = { kind: "building", map: 187, index: 2 };
  const cell: InspectorModel = { kind: "matrixCell", matrix: 131, x: 7, y: 25 };

  it("names_the_asset_of_each_editable_model", () => {
    expect(assetLine(event)).toBe("Eventdatei 0411, Index 3");
    expect(assetLine(building)).toBe("Karte 0187, Index 2");
    expect(assetLine(cell)).toBe("Raster 0131, Zelle 7, 25");
    expect(assetLine({ kind: "area", id: 5 })).toBe("Area-Data 0005");
    expect(assetLine({ kind: "header", id: 1 })).toBe("Header 0001, Eintrag der Header-Tabelle");
  });

  it("gives_the_unpacked_file_of_the_asset_or_null_when_it_has_none", () => {
    expect(filePath(event)).toBe("unpacked/eventFiles/0411");
    expect(filePath(building)).toBe("unpacked/maps/0187");
    expect(filePath(cell)).toBe("unpacked/matrices/0131");
    expect(filePath({ kind: "area", id: 5 })).toBe("unpacked/areaData/0005");
    // The header table is one file for all headers: a header has none of its own.
    expect(filePath({ kind: "header", id: 1 })).toBeNull();
    expect(filePath({ kind: "none" })).toBeNull();
  });

  it("says_nothing_for_a_model_without_an_editor", () => {
    expect(assetLine({ kind: "none" })).toBe("");
  });
});
