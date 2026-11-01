import { describe, expect, it } from "vitest";
import type { EventKind } from "../../api";
import { DEPTHS } from "../../ui/frame/shellState";
import { areaFieldSpecs } from "./areaFields";
import { eventFieldSpecs, eventFieldsAt } from "./eventFields";
import { fieldFrom, shownFields } from "./fields";
import { headerSections } from "./headerSections";
import { destinationSpec } from "./referenceFor";

const KINDS: EventKind[] = ["object", "warp", "coord", "bg"];

describe("the fields each depth shows", () => {
  it("defaults_a_field_without_from_to_bauen", () => {
    expect(fieldFrom({})).toBe("bauen");
    expect(fieldFrom({ from: "erzaehlen" })).toBe("erzaehlen");
    const specs = [{ key: "a", from: "erzaehlen" as const }, { key: "b" }];
    expect(shownFields(specs, "erzaehlen").map((s) => s.key)).toEqual(["a"]);
    expect(shownFields(specs, "bauen").map((s) => s.key)).toEqual(["a", "b"]);
    expect(shownFields(specs, "tuefteln").map((s) => s.key)).toEqual(["a", "b"]);
    // A door's destination header is no event field, but Erzählen shows it too.
    for (const depth of DEPTHS) expect(fieldFrom(destinationSpec(depth))).toBe("erzaehlen");
  });

  it("event_fields_per_depth_and_kind", () => {
    const lines = KINDS.flatMap((kind) =>
      ["Pt", "Hgss"].flatMap((family) => DEPTHS.map((depth) => `${kind} ${family} ${depth}: ${shownFields(eventFieldSpecs(kind, family), depth).map((s) => s.key).join(" ")}`)),
    );
    expect(lines).toMatchInlineSnapshot(`
      [
        "object Pt erzaehlen: graphics_id movement_type dir script",
        "object Pt bauen: x z local_id graphics_id movement_type dir script trainer_type sight hidden_flag",
        "object Pt tuefteln: x z local_id graphics_id movement_type dir script trainer_type sight hidden_flag",
        "object Hgss erzaehlen: graphics_id movement_type dir script",
        "object Hgss bauen: x z local_id graphics_id movement_type dir script trainer_type sight hidden_flag",
        "object Hgss tuefteln: x z local_id graphics_id movement_type dir script trainer_type sight hidden_flag",
        "warp Pt erzaehlen: dest_warp_id",
        "warp Pt bauen: x z dest_warp_id",
        "warp Pt tuefteln: x z dest_warp_id",
        "warp Hgss erzaehlen: dest_warp_id",
        "warp Hgss bauen: x z dest_warp_id height",
        "warp Hgss tuefteln: x z dest_warp_id height",
        "coord Pt erzaehlen: ",
        "coord Pt bauen: x z width length script var value",
        "coord Pt tuefteln: x z width length script var value",
        "coord Hgss erzaehlen: ",
        "coord Hgss bauen: x z width length script var value",
        "coord Hgss tuefteln: x z width length script var value",
        "bg Pt erzaehlen: ",
        "bg Pt bauen: x z event_type script",
        "bg Pt tuefteln: x z event_type script",
        "bg Hgss erzaehlen: ",
        "bg Hgss bauen: x z event_type script",
        "bg Hgss tuefteln: x z event_type script",
      ]
    `);
  });

  it("keeps_the_flag_and_variable_pickers_from_bauen_on", () => {
    // 3d.1's fields carry no `from`: Erzählen shows none of them, Bauen and Tüfteln pick them by name.
    expect(eventFieldsAt("object", "Pt", "erzaehlen").some((s) => s.pick !== undefined)).toBe(false);
    for (const depth of ["bauen", "tuefteln"] as const) {
      expect(eventFieldsAt("object", "Pt", depth).find((s) => s.key === "hidden_flag")?.pick).toBe("flag");
      expect(eventFieldsAt("coord", "Hgss", depth).find((s) => s.key === "var")?.pick).toBe("var");
    }
  });

  it("header_fields_per_depth_and_family", () => {
    const lines = (["DP", "Pt", "HGSS"] as const).flatMap((family) =>
      DEPTHS.flatMap((depth) => headerSections(family, depth).map((section) => `${family} ${depth} ${section.id}: ${section.specs.map((s) => s.key).join(" ")}`)),
    );
    expect(lines).toMatchInlineSnapshot(`
      [
        "DP erzaehlen ort: location_name area_data_id music_day_id music_night_id",
        "DP bauen maps: area_data_id matrix_id wild_pokemon event_file_id location_name location_specifier flags",
        "DP bauen look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "DP bauen texts: script_file_id level_script_id text_archive_id",
        "DP tuefteln maps: area_data_id matrix_id wild_pokemon event_file_id location_name location_specifier flags",
        "DP tuefteln look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "DP tuefteln texts: script_file_id level_script_id text_archive_id",
        "Pt erzaehlen ort: location_name area_data_id music_day_id music_night_id",
        "Pt bauen maps: area_data_id matrix_id wild_pokemon event_file_id location_name area_icon location_specifier flags",
        "Pt bauen look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "Pt bauen texts: script_file_id level_script_id text_archive_id",
        "Pt tuefteln maps: area_data_id matrix_id wild_pokemon event_file_id location_name area_icon location_specifier flags",
        "Pt tuefteln look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "Pt tuefteln texts: script_file_id level_script_id text_archive_id",
        "HGSS erzaehlen ort: location_name area_data_id music_day_id music_night_id",
        "HGSS bauen maps: wild_pokemon area_data_id worldmap_x worldmap_y matrix_id event_file_id location_name area_icon kanto_flag location_type follow_mode flags",
        "HGSS bauen look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "HGSS bauen texts: script_file_id level_script_id text_archive_id",
        "HGSS tuefteln maps: wild_pokemon area_data_id worldmap_x worldmap_y matrix_id event_file_id location_name area_icon kanto_flag location_type follow_mode flags",
        "HGSS tuefteln look: music_day_id music_night_id weather_id camera_angle_id battle_background",
        "HGSS tuefteln texts: script_file_id level_script_id text_archive_id",
      ]
    `);
  });

  it("area_fields_per_depth", () => {
    const lines = (["Hgss", "Dppt"] as const).flatMap((family) => DEPTHS.map((depth) => `${family} ${depth}: ${shownFields(areaFieldSpecs(family), depth).map((s) => s.key).join(" ")}`));
    expect(lines).toMatchInlineSnapshot(`
      [
        "Hgss erzaehlen: ",
        "Hgss bauen: map_tileset buildings_tileset dynamic_texture_type area_type light_type",
        "Hgss tuefteln: map_tileset buildings_tileset dynamic_texture_type area_type light_type",
        "Dppt erzaehlen: ",
        "Dppt bauen: map_tileset buildings_tileset light_type",
        "Dppt tuefteln: map_tileset buildings_tileset light_type",
      ]
    `);
  });

  it("names_the_fields_of_erzaehlen_in_the_words_of_the_story", () => {
    const ort = headerSections("Pt", "erzaehlen")[0]!;
    expect(ort.specs.map((s) => s.label)).toEqual(["Ort", "Aussehen", "Musik (Tag)", "Musik (Nacht)"]);
    // Bauen keeps its words.
    const bauen = headerSections("Pt", "bauen").flatMap((s) => s.specs);
    expect(bauen.find((s) => s.key === "area_data_id")?.label).toBe("Area-Data");
    expect(bauen.find((s) => s.key === "music_day_id")?.label).toBe("Musik Tag");
  });
});
