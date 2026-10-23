import { describe, expect, it } from "vitest";
import type { MapHeader, MapHeaderHGSS, MapHeaderPt } from "../../api";
import { fieldBits, fieldValue, headerFieldSpecs, pickMax, withField } from "./fields";

const pt: MapHeaderPt = {
  area_data_id: 1,
  unknown1: 0,
  matrix_id: 2,
  script_file_id: 3,
  level_script_id: 4,
  text_archive_id: 5,
  music_day_id: 6,
  music_night_id: 7,
  wild_pokemon: 8,
  event_file_id: 9,
  location_name: 10,
  area_icon: 11,
  weather_id: 12,
  camera_angle_id: 13,
  location_specifier: 14,
  battle_background: 15,
  flags: 1,
};

describe("headerFieldSpecs", () => {
  it("lists family-specific header fields in struct order", () => {
    const pt = headerFieldSpecs("Pt").map((f) => f.key);
    expect(pt.slice(0, 3)).toEqual(["area_data_id", "matrix_id", "script_file_id"]);
    expect(pt).toContain("area_icon");
    expect(pt).not.toContain("worldmap_x");
    expect(headerFieldSpecs("HGSS").map((f) => f.key)).toContain("kanto_flag");
  });

  it("names every field of the header struct except the unknown ones", () => {
    const keys = (family: "DP" | "Pt" | "HGSS") => headerFieldSpecs(family).map((f) => f.key);
    // Every spec key exists in the struct, so an edit can never add a stray property.
    for (const key of keys("Pt")) expect(pt).toHaveProperty(key);
    expect(keys("Pt")).not.toContain("unknown1");
    expect(keys("DP")).not.toContain("area_icon");
    expect(keys("HGSS")[0]).toBe("wild_pokemon");
    expect(headerFieldSpecs("Pt").every((f) => f.label !== "")).toBe(true);
    expect(headerFieldSpecs("Pt")[1]).toMatchObject({ label: "Matrix", kind: "pick", pick: "matrix" });
  });

  it("makes_the_script_file_and_the_level_script_pickers_of_the_script_archive", () => {
    for (const family of ["DP", "Pt", "HGSS"] as const) {
      const specs = headerFieldSpecs(family);
      expect(specs.find((f) => f.key === "script_file_id")).toMatchObject({ label: "Scriptdatei", kind: "pick", pick: "script" });
      expect(specs.find((f) => f.key === "level_script_id")).toMatchObject({ label: "Level-Script", kind: "pick", pick: "script" });
    }
  });

  it("uses the width the family stores each field in", () => {
    const kind = (family: "DP" | "Pt" | "HGSS", key: string) => headerFieldSpecs(family).find((f) => f.key === key)?.kind;
    expect(kind("DP", "location_name")).toBe("u16");
    expect(kind("Pt", "location_name")).toBe("u8");
    expect(kind("HGSS", "wild_pokemon")).toBe("u8");
    expect(kind("Pt", "wild_pokemon")).toBe("u16");
    expect(kind("HGSS", "kanto_flag")).toBe("bool");
  });
});

describe("fieldBits", () => {
  it("is_the_storage_width_of_the_number_a_field_holds", () => {
    expect(fieldBits({ key: "weather_id", label: "", kind: "u8" })).toBe(8);
    expect(fieldBits({ key: "music_day_id", label: "", kind: "u16" })).toBe(16);
    // A picker is as wide as the field it is stored in: the area data id is a byte, the others are words.
    expect(fieldBits({ key: "area_data_id", label: "", kind: "pick", pick: "areaData" })).toBe(8);
    expect(fieldBits({ key: "matrix_id", label: "", kind: "pick", pick: "matrix" })).toBe(16);
  });
});

describe("pickMax", () => {
  it("is the member count minus one of the picked kind", () => {
    const counts = { Matrix: 4, AreaData: 10, Event: 0, TextArchive: 600, Script: 3 };
    expect(pickMax("matrix", counts)).toBe(3);
    expect(pickMax("areaData", counts)).toBe(9);
    expect(pickMax("textArchive", counts)).toBe(599);
    expect(pickMax("eventFile", counts)).toBe(0);
    expect(pickMax("script", counts)).toBe(2);
    expect(pickMax("script", { Script: 3 })).toBe(2);
    expect(pickMax("script", {})).toBeUndefined();
    expect(pickMax("matrix", {})).toBeUndefined();
  });
});

describe("fieldValue and withField", () => {
  it("reads and replaces one field, keeping the family tag", () => {
    const header: MapHeader = { Pt: pt };
    expect(fieldValue(header, "matrix_id")).toBe(2);
    const next = withField(header, "matrix_id", 126);
    expect(next).toEqual({ Pt: { ...pt, matrix_id: 126 } });
    // The original is untouched.
    expect(fieldValue(header, "matrix_id")).toBe(2);
  });

  it("stores booleans as booleans", () => {
    const hgss = { kanto_flag: false } as MapHeaderHGSS;
    const header: MapHeader = { HGSS: hgss };
    expect(fieldValue(header, "kanto_flag")).toBe(0);
    expect(withField(header, "kanto_flag", 1)).toEqual({ HGSS: { kanto_flag: true } });
  });
});
