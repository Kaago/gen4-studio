import type { EventKind, EventsDto, HeaderSummary, WarpDto } from "../api";
import { switcherMenu } from "../ui/primitives";
import type { MenuItemSpec, PanelHeadSwitcher } from "../ui/primitives";
import type { Depth } from "../ui/frame/shellState";
import { eventAt, eventTitle } from "../world/events";
import type { InspectorModel } from "./Inspector";
import type { NavTarget } from "./navigation";
import { UNNAMED_KEY, placeOf } from "./places";
import type { Place } from "./places";
import { id4, termFor } from "./terms";

/** One link of the hinge's breadcrumb. `tech` is the grey identifier next to the label. */
export interface Crumb {
  label: string;
  tech?: string;
  onClick?: () => void;
  /** Makes the crumb a menu button with these entries (the parts of an area) instead of a link. */
  menu?: MenuItemSpec[];
}

export interface CrumbInput {
  places: Place[];
  headers: HeaderSummary[];
  selected: number | null;
  /** What the inspector is pinned to (the hover is not a selection). */
  pinned: InspectorModel;
  /** The name of the matrix a pinned cell belongs to, shown grey next to the cell. */
  matrixName?: string;
  /** The events of the pinned event's file (what `useEvents` reads for the inspector): the crumb then has the inspector's heading. Null or missing: the short words. */
  events?: EventsDto | null;
  /** Makes the crumbs links: each one leads where its label says. */
  goTo?: (target: NavTarget) => void;
  /** Bauen names the header "Gebiet" or "Raum", Tüfteln "Header". */
  depth: Depth;
}

/** The leading words of `eventTitle`, shown while the event's data is not there (yet). */
const EVENT_WORD: Record<EventKind, string> = { warp: "Warp", object: "Objekt", coord: "Trigger", bg: "Hintergrund-Event" };

/** The heading of the pinned event as the inspector writes it (`eventTitle`) once its data is known, else the short form. */
function eventLabel(pinned: { eventFile: number; eventKind: EventKind; index: number }, headers: HeaderSummary[], events: EventsDto | null | undefined, depth: Depth): string {
  const data = events && events.event_file_id === pinned.eventFile ? eventAt(events, pinned.eventKind, pinned.index) : undefined;
  if (data === undefined) return `${EVENT_WORD[pinned.eventKind]} ${pinned.index}`;
  const destination = pinned.eventKind === "warp" ? headers.find((h) => h.id === (data as WarpDto).dest_header_id) : undefined;
  return eventTitle(pinned.eventKind, data, depth, destination?.internal_name);
}

/** At most three crumbs: place › header (Gebiet or Raum, Tüfteln: Header, with its id) › selection (event, building, matrix cell or area data). */
export function crumbsFor({ places, headers, selected, pinned, matrixName, events, goTo, depth }: CrumbInput): Crumb[] {
  const header = selected === null ? undefined : headers.find((h) => h.id === selected);
  if (header === undefined) return [{ label: headers.length === 0 ? "Kein Projekt" : "Kein Ort gewählt" }];
  const link = (target: NavTarget) => (goTo ? () => goTo(target) : undefined);
  const crumbs: Crumb[] = [];
  const place = placeOf(places, header.id);
  if (place) crumbs.push({ label: place.name, onClick: link({ kind: "header", id: place.headers[0]!.header.id }) });
  // The explorer's words, not the place's name again: an outside header is a "Gebiet", one inside a "Raum" (a header of an unnamed place is always a room).
  const outside = place !== undefined && place.key !== UNNAMED_KEY && place.headers.find((h) => h.header.id === header.id)?.kind === "outside";
  crumbs.push({ label: termFor("header", depth, outside ? "area" : "room"), tech: id4(header.id), onClick: link({ kind: "header", id: header.id }) });
  switch (pinned.kind) {
    case "event":
      crumbs.push({ label: eventLabel(pinned, headers, events, depth), tech: `Eventdatei ${id4(pinned.eventFile)}`, onClick: link({ kind: "event", eventFile: pinned.eventFile, eventKind: pinned.eventKind, index: pinned.index }) });
      break;
    case "building":
      crumbs.push({ label: `Gebäude ${pinned.index + 1}`, onClick: link({ kind: "building", map: pinned.map, index: pinned.index }) });
      break;
    case "matrixCell":
      crumbs.push({ label: `Zelle ${pinned.x}, ${pinned.y}`, tech: matrixName, onClick: link({ kind: "matrixCell", matrix: pinned.matrix, x: pinned.x, y: pinned.y }) });
      break;
    case "area":
      crumbs.push({ label: `Area-Data ${id4(pinned.id)}`, onClick: link({ kind: "area", id: pinned.id }) });
      break;
    default:
      break;
  }
  return crumbs;
}

export interface TextCrumbInput {
  /** The archive the texts show (null: none chosen). */
  archive: number | null;
  /** The name of the place whose header uses the archive; null or missing for an archive no place uses. */
  place?: string | null;
  /** The chosen message; null or missing: none. */
  message?: number | null;
  /** Makes the archive and the message links. */
  goTo?: (target: NavTarget) => void;
}

/** The breadcrumb of the texts: Texte › Archiv 0412, Zweiblattdorf › Nachricht 005. Without an archive only the area. */
export function textCrumbs({ archive, place, message, goTo }: TextCrumbInput): Crumb[] {
  const crumbs: Crumb[] = [{ label: "Texte" }];
  if (archive === null) return crumbs;
  const link = (target: NavTarget) => (goTo ? () => goTo(target) : undefined);
  crumbs.push({ label: place ? `Archiv ${id4(archive)}, ${place}` : `Archiv ${id4(archive)}`, onClick: link({ kind: "textArchive", index: archive }) });
  if (message !== null && message !== undefined) {
    crumbs.push({ label: `Nachricht ${String(message).padStart(3, "0")}`, onClick: link({ kind: "textArchive", index: archive, message }) });
  }
  return crumbs;
}

/** The selected member of a data area: its name, and where a click on it leads. */
export interface DataCrumbSelection {
  label: string;
  target: NavTarget;
}

export type DataCrumbInput =
  /** Pokémon: the part with its switcher as the second crumb. */
  | { area: "pokemon"; switcher: PanelHeadSwitcher; selection: DataCrumbSelection | null; goTo?: (target: NavTarget) => void }
  /** Gegenstände: the pocket of the selected item as the second crumb (a click on it filters the list by it). */
  | { area: "gegenstaende"; pocket: { label: string; onClick: () => void } | null; selection: DataCrumbSelection | null; goTo?: (target: NavTarget) => void };

/**
 * The breadcrumb of a data area: Pokémon › Pokédex › Staravia, where the part opens the area's switcher as a menu; Gegenstände › Tasche
 * Medizin › Trank, where the pocket filters the list. Without a selection: the area and its part (Pokémon), the area alone (Gegenstände).
 */
export function dataCrumbs(input: DataCrumbInput): Crumb[] {
  const { selection, goTo } = input;
  const crumbs: Crumb[] = [{ label: input.area === "pokemon" ? "Pokémon" : "Gegenstände" }];
  if (input.area === "pokemon") {
    const { switcher } = input;
    const current = switcher.items.find((item) => item.id === switcher.value);
    crumbs.push({ label: current?.label ?? switcher.value, menu: switcherMenu(switcher) });
  } else if (input.pocket !== null) {
    crumbs.push({ label: input.pocket.label, onClick: input.pocket.onClick });
  }
  if (selection !== null) crumbs.push({ label: selection.label, onClick: goTo ? () => goTo(selection.target) : undefined });
  return crumbs;
}
