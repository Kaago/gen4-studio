import type { HeaderSummary } from "../api";

/** ", 1. Stock" for the second floor (`…02`), nothing for the ground floor. */
function floor(nn: string): string {
  const n = Number(nn);
  return n > 1 ? `, ${n - 1}. Stock` : "";
}

/** The name the code (internal name) suggests, or null when it suggests none. */
function codeName(code: string): string | null {
  const name = code.trim().toUpperCase();
  let m = /PC\d\d(\d\d)$/.exec(name);
  if (m) return `Pokémon-Center${floor(m[1])}`;
  m = /FS\d\d(\d\d)$/.exec(name);
  if (m) return `Pokémarkt${floor(m[1])}`;
  m = /GYM\d\d(\d\d)$/.exec(name);
  if (m) return Number(m[1]) > 1 ? `Arena, Raum ${Number(m[1])}` : "Arena";
  m = /R(\d\d)(\d\d)$/.exec(name);
  if (m) return name.startsWith("D") ? `Ebene ${Number(m[2])}` : `Haus ${Number(m[1])}${floor(m[2])}`;
  return null;
}

/**
 * The suggested name of a room: from its code ("C01PC0102" is "Pokémon-Center, 1. Stock", "T01R0201" "Haus 2",
 * "D05R0103" "Ebene 3"), else its place name when that is not the area's, else "Raum" and `ordinal` (the room's number
 * among the rooms of the area named so). An empty `placeName` means the room has no place.
 */
export function roomName(header: Pick<HeaderSummary, "internal_name">, placeName: string, areaPlaceName: string, ordinal: number): string {
  return codeName(header.internal_name) ?? (placeName !== "" && placeName !== areaPlaceName ? placeName : `Raum ${ordinal}`);
}

/** Whether `roomName` falls back to a numbered room, so the caller counts the ordinal only for those. */
export function isNumberedRoom(header: Pick<HeaderSummary, "internal_name">, placeName: string, areaPlaceName: string): boolean {
  return codeName(header.internal_name) === null && (placeName === "" || placeName === areaPlaceName);
}
