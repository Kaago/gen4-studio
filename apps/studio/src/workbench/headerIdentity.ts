import type { HeaderSummary } from "../api";
import { UNNAMED_KEY, placeOf } from "./places";
import type { Place } from "./places";
import { suggestRoomName } from "./roomNames";
import { NO_PLACE_INDEX } from "./worldModel";

/** What a header is in the words of Bauen: an area or a room, its clear name, and the name of its place. */
export interface HeaderIdentity {
  kind: "area" | "room";
  /** The area's place name; a room's name as its code suggests it ("Haus 1"), else "Raum" and its number. */
  name: string;
  /** The place the header belongs to ("Ohne Ortsnamen" for none). */
  place: string;
}

const placeNameOf = (h: HeaderSummary): string => (h.location_index === NO_PLACE_INDEX ? "" : h.location_name.trim());

/**
 * Whether a header is an area or a room, and its clear name, from the places (the world model lives in the explorer and is not at
 * hand in the breadcrumb or the inspector): an outside header of a named place is an area named after the place, every other header
 * is a room. A room is named by `suggestRoomName`, numbered among the rooms of its place; a name given in the project file
 * (`g4-world.json`) is not known here.
 */
export function headerIdentity(places: Place[], header: HeaderSummary): HeaderIdentity {
  const place = placeOf(places, header.id);
  const named = place !== undefined && place.key !== UNNAMED_KEY;
  const placeName = place?.name ?? (placeNameOf(header) || header.internal_name);
  if (named && place.headers.find((h) => h.header.id === header.id)?.kind === "outside") return { kind: "area", name: place.name, place: placeName };
  const areaName = named ? place.name : "";
  let ordinal = 0;
  for (const { header: room, kind } of place?.headers ?? []) {
    if (named && kind === "outside") continue;
    const { name, numbered } = suggestRoomName(room, placeNameOf(room), areaName, ordinal + 1);
    if (room.id === header.id) return { kind: "room", name, place: placeName };
    if (numbered) ordinal += 1;
  }
  // A header the places do not list (yet).
  return { kind: "room", name: suggestRoomName(header, placeNameOf(header), areaName, 1).name, place: placeName };
}
