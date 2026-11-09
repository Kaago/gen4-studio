import type { HeaderSummary } from "../api";
import { t } from "../i18n";

/** ", 1. Stock" for the second floor (`…02`), nothing for the ground floor. */
function floor(nn: string): string {
  const n = Number(nn);
  return n > 1 ? t("workbench:roomNames.floor", { n: n - 1 }) : "";
}

/** The name the code (internal name) suggests, or null when it suggests none. */
function codeName(code: string): string | null {
  const name = code.trim().toUpperCase();
  let m = /PC\d\d(\d\d)$/.exec(name);
  if (m) return t("workbench:roomNames.pokemonCenter", { floor: floor(m[1]) });
  m = /FS\d\d(\d\d)$/.exec(name);
  if (m) return t("workbench:roomNames.pokeMart", { floor: floor(m[1]) });
  m = /GYM\d\d(\d\d)$/.exec(name);
  if (m) return Number(m[1]) > 1 ? t("workbench:roomNames.gymRoom", { n: Number(m[1]) }) : t("workbench:roomNames.gym");
  m = /R(\d\d)(\d\d)$/.exec(name);
  if (m) return name.startsWith("D") ? t("workbench:roomNames.level", { n: Number(m[2]) }) : t("workbench:roomNames.house", { n: Number(m[1]), floor: floor(m[2]) });
  return null;
}

/**
 * The suggested name of a room: from its code ("C01PC0102" is "Pokémon-Center, 1. Stock", "T01R0201" "Haus 2",
 * "D05R0103" "Ebene 3"), else its place name when that is not the area's, else "Raum" and `ordinal` (the room's number
 * among the rooms of the area named so). An empty `placeName` means the room has no place.
 */
export function roomName(header: Pick<HeaderSummary, "internal_name">, placeName: string, areaPlaceName: string, ordinal: number): string {
  return suggestRoomName(header, placeName, areaPlaceName, ordinal).name;
}

/**
 * `roomName` and whether it is a numbered room ("Raum" and `ordinal`), so a caller naming the rooms of an area counts
 * the ordinal up only for those.
 */
export function suggestRoomName(header: Pick<HeaderSummary, "internal_name">, placeName: string, areaPlaceName: string, ordinal: number): { name: string; numbered: boolean } {
  const fromCode = codeName(header.internal_name);
  if (fromCode !== null) return { name: fromCode, numbered: false };
  if (placeName !== "" && placeName !== areaPlaceName) return { name: placeName, numbered: false };
  return { name: t("workbench:roomNames.room", { n: ordinal }), numbered: true };
}

/**
 * `suggestRoomName` for the rooms of one area in their order, each with its place name (empty: none): the ordinal counts up only
 * for the rooms named "Raum" and a number.
 */
export function suggestRoomNames(rooms: readonly { header: Pick<HeaderSummary, "internal_name">; placeName: string }[], areaPlaceName: string): string[] {
  let ordinal = 0;
  return rooms.map(({ header, placeName }) => {
    const { name, numbered } = suggestRoomName(header, placeName, areaPlaceName, ordinal + 1);
    if (numbered) ordinal += 1;
    return name;
  });
}
