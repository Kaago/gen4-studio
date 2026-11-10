import { errorText, getEvents } from "../../api";
import { t } from "../../i18n";
import { lockedText } from "../events";
import { setMarkers } from "./content";
import type { SceneHost } from "./SceneHost";
import type { FetchedEvents } from "./types";

/** The note shown over the scene about the events: why they are missing, or that they are locked. */
export function eventsNote(fetched: FetchedEvents): string | null {
  if (fetched.error) return fetched.error;
  return fetched.events?.locked ? t("world:scene.eventsLocked", { locked: lockedText(fetched.events.locked) }) : null;
}

/** Fetches the events of a header's event file; never rejects, a failure comes back as text. */
export async function fetchEvents(eventFileId: number | null): Promise<FetchedEvents> {
  if (eventFileId === null) return { events: null, error: null };
  try {
    return { events: await getEvents(eventFileId), error: null };
  } catch (e) {
    return { events: null, error: t("world:scene.eventsNotLoaded", { error: errorText(e) }) };
  }
}

/**
 * Reads one event file's events again and shows them (their markers are rebuilt). Dropped when
 * `current()` turned false meanwhile (another matrix or event file is shown now).
 */
export async function refreshEvents(h: SceneHost, file: number, visible: boolean, current: () => boolean, report: (fetched: FetchedEvents) => void) {
  const matrixAtStart = h.matrix;
  const fetched = await fetchEvents(file);
  if (h.disposed || h.matrix !== matrixAtStart || !current()) return;
  setMarkers(h, fetched.events, visible);
  report(fetched);
  h.render();
  h.refreshHover?.();
}
