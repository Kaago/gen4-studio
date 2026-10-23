import type { HeaderFamily } from "../../api";
import { headerFieldSpecs } from "./fields";
import type { FieldSpec } from "./fields";

export type HeaderSectionId = "maps" | "look" | "texts";

export interface HeaderSection {
  id: HeaderSectionId;
  title: string;
  specs: FieldSpec[];
}

/** The sections of the header form and the fields in each; a field that is not listed here comes under "Karten und Raster". */
const LOOK = new Set(["music_day_id", "music_night_id", "weather_id", "camera_angle_id", "battle_background"]);
const TEXTS = new Set(["text_archive_id", "script_file_id", "level_script_id"]);

/**
 * The header's fields split into "Karten und Raster" (what the place is made of and called), "Aussehen" (music, weather,
 * camera, battle) and "Texte und Scripts". Each section keeps the order of the struct.
 */
export function headerSections(family: HeaderFamily): HeaderSection[] {
  const specs = headerFieldSpecs(family);
  return [
    { id: "maps", title: "Karten und Raster", specs: specs.filter((s) => !LOOK.has(s.key) && !TEXTS.has(s.key)) },
    { id: "look", title: "Aussehen", specs: specs.filter((s) => LOOK.has(s.key)) },
    { id: "texts", title: "Texte und Scripts", specs: specs.filter((s) => TEXTS.has(s.key)) },
  ];
}
