import { describe, expect, it } from "vitest";
import { headerFieldSpecs } from "./fields";
import { headerSections } from "./headerSections";

describe("headerSections", () => {
  it("names_the_three_form_sections_in_order", () => {
    expect(headerSections("Pt", "bauen").map((s) => s.title)).toEqual(["Karten", "Aussehen", "Texte und Scripts"]);
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    for (const family of ["DP", "Pt", "HGSS"] as const) {
      const words = headerSections(family, "bauen").flatMap((s) => [s.title, ...s.specs.map((f) => f.label)]);
      expect(words.join(" ")).not.toMatch(/Raster|Matrix|Header/);
    }
    expect(headerSections("Pt", "bauen")[0]!.specs.find((f) => f.key === "matrix_id")?.label).toBe("Weltkarte");
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    const maps = headerSections("Pt", "tuefteln")[0]!;
    expect(maps.title).toBe("Karten und Raster");
    expect(maps.specs.find((f) => f.key === "matrix_id")?.label).toBe("Matrix");
  });

  it("puts_music_weather_camera_and_battle_under_Aussehen", () => {
    const look = headerSections("Pt", "bauen").find((s) => s.title === "Aussehen")!;
    expect(look.specs.map((s) => s.key)).toEqual(["music_day_id", "music_night_id", "weather_id", "camera_angle_id", "battle_background"]);
  });

  it("puts_the_text_archive_and_both_script_pickers_under_Texte_und_Scripts", () => {
    const texts = headerSections("HGSS", "bauen").find((s) => s.title === "Texte und Scripts")!;
    expect(texts.specs.map((s) => s.key)).toEqual(["script_file_id", "level_script_id", "text_archive_id"]);
  });

  it("shows_every_field_of_every_family_exactly_once", () => {
    for (const family of ["DP", "Pt", "HGSS"] as const) {
      const shown = headerSections(family, "bauen").flatMap((s) => s.specs.map((f) => f.key));
      expect([...shown].sort()).toEqual(headerFieldSpecs(family).map((f) => f.key).sort());
    }
  });

  it("keeps_the_struct_order_inside_a_section", () => {
    const maps = headerSections("HGSS", "bauen")[0]!.specs.map((s) => s.key);
    expect(maps.slice(0, 4)).toEqual(["wild_pokemon", "area_data_id", "worldmap_x", "worldmap_y"]);
  });

  it("erzaehlen_has_the_section_ort_only", () => {
    for (const family of ["DP", "Pt", "HGSS"] as const) {
      const sections = headerSections(family, "erzaehlen");
      expect(sections.map((s) => [s.id, s.title])).toEqual([["ort", "Ort"]]);
      // In the order of the section: the place first, then its look and music (not the struct's).
      expect(sections[0]!.specs.map((s) => s.key)).toEqual(["location_name", "area_data_id", "music_day_id", "music_night_id"]);
      expect(sections[0]!.specs.map((s) => s.label)).toEqual(["Ort", "Aussehen", "Musik (Tag)", "Musik (Nacht)"]);
    }
  });
});
