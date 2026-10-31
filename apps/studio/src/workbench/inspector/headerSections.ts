import type { HeaderFamily } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { termFor } from "../terms";
import { headerFieldSpecs, shownFields } from "./fields";
import type { FieldSpec } from "./fields";

/** `ort`: the one section of Erzählen. */
export type HeaderSectionId = "maps" | "look" | "texts" | "ort";

export interface HeaderSection {
  id: HeaderSectionId;
  title: string;
  specs: FieldSpec[];
}

/** The sections of the header form and the fields in each; a field that is not listed here comes under "Karten" (Tüfteln: "Karten und Raster"). */
const LOOK = new Set(["music_day_id", "music_night_id", "weather_id", "camera_angle_id", "battle_background"]);
const TEXTS = new Set(["text_archive_id", "script_file_id", "level_script_id"]);

/** What Erzählen calls the fields it shows. */
const NARRATING_LABELS: Record<string, string> = { location_name: "Ort", area_data_id: "Aussehen", music_day_id: "Musik (Tag)", music_night_id: "Musik (Nacht)" };
/** The order of the section „Ort“: the place first, then its look and music (the struct has the look and music first). */
const NARRATING_ORDER = ["location_name", "area_data_id", "music_day_id", "music_night_id"];
/** A field's place in the section „Ort“; one it does not list comes after, in the struct's order. */
const narratingRank = (key: string) => {
  const rank = NARRATING_ORDER.indexOf(key);
  return rank === -1 ? NARRATING_ORDER.length : rank;
};

/**
 * The header's fields split into "Karten" (what the place is made of and called; Tüfteln: "Karten und Raster"), "Aussehen" (music,
 * weather, camera, battle) and "Texte und Scripts". Each section keeps the order of the struct. Bauen calls the matrix field "Weltkarte".
 * Erzählen has one section "Ort" with the fields from Erzählen in its own words and order: place, look, music by day and by night.
 */
export function headerSections(family: HeaderFamily, depth: Depth): HeaderSection[] {
  if (!depthAtLeast(depth, "bauen")) {
    const specs = shownFields(headerFieldSpecs(family), depth)
      .map((s) => ({ ...s, label: NARRATING_LABELS[s.key] ?? s.label }))
      .sort((a, b) => narratingRank(a.key) - narratingRank(b.key));
    return [{ id: "ort", title: "Ort", specs }];
  }
  const specs = headerFieldSpecs(family).map((s) => (s.key === "matrix_id" ? { ...s, label: termFor("matrix", depth) } : s));
  return [
    { id: "maps", title: depthAtLeast(depth, "tuefteln") ? "Karten und Raster" : "Karten", specs: specs.filter((s) => !LOOK.has(s.key) && !TEXTS.has(s.key)) },
    { id: "look", title: "Aussehen", specs: specs.filter((s) => LOOK.has(s.key)) },
    { id: "texts", title: "Texte und Scripts", specs: specs.filter((s) => TEXTS.has(s.key)) },
  ];
}
