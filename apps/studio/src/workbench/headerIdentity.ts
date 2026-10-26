import type { HeaderSummary } from "../api";
import { UNNAMED_KEY, placeOf } from "./places";
import type { Place } from "./places";
import { suggestRoomNames } from "./roomNames";
import { placeNameOf } from "./worldModel";
import type { WorldNames } from "./worldModel";

/** What a header is in the words of Bauen: an area, a room or another header, its clear name, and the name of its place. */
export interface HeaderIdentity {
  kind: "area" | "room" | "other";
  /** The area's name; a room's name ("Haus 1", or the name the project file gives it); else the header's place or code. */
  name: string;
  /** The place the header belongs to ("Ohne Ortsnamen" for none). */
  place: string;
}

/**
 * What a header is and how it is called: as the world model says (`worldNames`, the names of the world tree) once it is read.
 * Until then, from the places: an outside header of a named place is an area named after the place, every other header a room,
 * named like the world model names it (`suggestRoomNames`) among the rooms of its place.
 */
export function headerIdentity(places: Place[], header: HeaderSummary, names: WorldNames | null = null): HeaderIdentity {
  const known = names?.get(header.id);
  if (known) return { kind: known.kind, name: known.name, place: known.placeName };
  const place = placeOf(places, header.id);
  const named = place !== undefined && place.key !== UNNAMED_KEY;
  const placeName = place?.name ?? (placeNameOf(header) || header.internal_name);
  if (named && place.headers.find((h) => h.header.id === header.id)?.kind === "outside") return { kind: "area", name: place.name, place: placeName };
  const rooms = (place?.headers ?? []).filter((h) => !named || h.kind !== "outside").map((h) => h.header);
  const at = rooms.findIndex((h) => h.id === header.id);
  const list = at < 0 ? [header] : rooms;
  const suggested = suggestRoomNames(
    list.map((h) => ({ header: h, placeName: placeNameOf(h) })),
    named ? place.name : "",
  );
  return { kind: "room", name: suggested[Math.max(at, 0)]!, place: placeName };
}
