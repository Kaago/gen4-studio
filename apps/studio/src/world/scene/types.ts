import type { EventsDto, MapDto } from "../../api";

/** A map the scene has fetched (the scene mutates `map` as permissions and buildings change). */
export interface Loaded {
  map: MapDto;
}

/** What reading an event file gave: its events, or why there are none (never both). */
export interface FetchedEvents {
  events: EventsDto | null;
  error: string | null;
}
