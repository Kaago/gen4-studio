import { describe, expect, it } from "vitest";
import { headerFieldSpecs } from "./fields";
import { headerSections } from "./headerSections";

describe("headerSections", () => {
  it("names_the_three_form_sections_in_order", () => {
    expect(headerSections("Pt").map((s) => s.title)).toEqual(["Karten und Raster", "Aussehen", "Texte und Scripts"]);
  });

  it("puts_music_weather_camera_and_battle_under_Aussehen", () => {
    const look = headerSections("Pt").find((s) => s.title === "Aussehen")!;
    expect(look.specs.map((s) => s.key)).toEqual(["music_day_id", "music_night_id", "weather_id", "camera_angle_id", "battle_background"]);
  });

  it("puts_the_text_archive_and_both_script_pickers_under_Texte_und_Scripts", () => {
    const texts = headerSections("HGSS").find((s) => s.title === "Texte und Scripts")!;
    expect(texts.specs.map((s) => s.key)).toEqual(["script_file_id", "level_script_id", "text_archive_id"]);
  });

  it("shows_every_field_of_every_family_exactly_once", () => {
    for (const family of ["DP", "Pt", "HGSS"] as const) {
      const shown = headerSections(family).flatMap((s) => s.specs.map((f) => f.key));
      expect([...shown].sort()).toEqual(headerFieldSpecs(family).map((f) => f.key).sort());
    }
  });

  it("keeps_the_struct_order_inside_a_section", () => {
    const maps = headerSections("HGSS")[0]!.specs.map((s) => s.key);
    expect(maps.slice(0, 4)).toEqual(["wild_pokemon", "area_data_id", "worldmap_x", "worldmap_y"]);
  });
});
