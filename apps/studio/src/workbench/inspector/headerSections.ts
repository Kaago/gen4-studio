import type { HeaderFamily } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { termFor } from "../terms";
import { headerFieldSpecs } from "./fields";
import type { FieldSpec } from "./fields";

export type HeaderSectionId = "maps" | "look" | "texts";

export interface HeaderSection {
  id: HeaderSectionId;
  title: string;
  specs: FieldSpec[];
}

/** The sections of the header form and the fields in each; a field that is not listed here comes under "Karten" (Tüfteln: "Karten und Raster"). */
const LOOK = new Set(["music_day_id", "music_night_id", "weather_id", "camera_angle_id", "battle_background"]);
const TEXTS = new Set(["text_archive_id", "script_file_id", "level_script_id"]);

/**
 * The header's fields split into "Karten" (what the place is made of and called; Tüfteln: "Karten und Raster"), "Aussehen" (music,
 * weather, camera, battle) and "Texte und Scripts". Each section keeps the order of the struct. Bauen calls the matrix field "Weltkarte".
 */
export function headerSections(family: HeaderFamily, depth: Depth): HeaderSection[] {
  const specs = headerFieldSpecs(family).map((s) => (s.key === "matrix_id" ? { ...s, label: termFor("matrix", depth) } : s));
  return [
    { id: "maps", title: depthAtLeast(depth, "tuefteln") ? "Karten und Raster" : "Karten", specs: specs.filter((s) => !LOOK.has(s.key) && !TEXTS.has(s.key)) },
    { id: "look", title: "Aussehen", specs: specs.filter((s) => LOOK.has(s.key)) },
    { id: "texts", title: "Texte und Scripts", specs: specs.filter((s) => TEXTS.has(s.key)) },
  ];
}
