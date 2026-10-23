import type { EventKind, HeaderSummary } from "../api";
import type { InspectorModel } from "./Inspector";
import type { NavTarget } from "./navigation";
import { placeOf } from "./places";
import type { Place } from "./places";
import { id4 } from "./terms";

/** One link of the hinge's breadcrumb. `tech` is the grey identifier next to the label. */
export interface Crumb {
  label: string;
  tech?: string;
  onClick?: () => void;
}

export interface CrumbInput {
  places: Place[];
  headers: HeaderSummary[];
  selected: number | null;
  /** What the inspector is pinned to (the hover is not a selection). */
  pinned: InspectorModel;
  /** The name of the matrix a pinned cell belongs to, shown grey next to the cell. */
  matrixName?: string;
  /** Makes the crumbs links: each one leads where its label says. */
  goTo?: (target: NavTarget) => void;
}

/** The leading words of `eventTitle`: the pinned model knows the event's class and index, not its data. */
const EVENT_WORD: Record<EventKind, string> = { warp: "Warp", object: "Objekt", coord: "Trigger", bg: "Hintergrund-Event" };

/** At most three crumbs: place › header › selection (event, building, matrix cell or area data). */
export function crumbsFor({ places, headers, selected, pinned, matrixName, goTo }: CrumbInput): Crumb[] {
  const header = selected === null ? undefined : headers.find((h) => h.id === selected);
  if (header === undefined) return [{ label: headers.length === 0 ? "Kein Projekt" : "Kein Ort gewählt" }];
  const link = (target: NavTarget) => (goTo ? () => goTo(target) : undefined);
  const crumbs: Crumb[] = [];
  const place = placeOf(places, header.id);
  if (place) crumbs.push({ label: place.name, onClick: link({ kind: "header", id: place.headers[0]!.header.id }) });
  crumbs.push({ label: header.location_name.trim() || "Raum", tech: id4(header.id), onClick: link({ kind: "header", id: header.id }) });
  switch (pinned.kind) {
    case "event":
      crumbs.push({ label: `${EVENT_WORD[pinned.eventKind]} ${pinned.index}`, tech: `Eventdatei ${id4(pinned.eventFile)}`, onClick: link({ kind: "event", eventFile: pinned.eventFile, eventKind: pinned.eventKind, index: pinned.index }) });
      break;
    case "building":
      crumbs.push({ label: `Gebäude ${pinned.index}`, onClick: link({ kind: "building", map: pinned.map, index: pinned.index }) });
      break;
    case "matrixCell":
      crumbs.push({ label: `Zelle ${pinned.x}, ${pinned.y}`, tech: matrixName, onClick: link({ kind: "matrixCell", matrix: pinned.matrix, x: pinned.x, y: pinned.y }) });
      break;
    case "area":
      crumbs.push({ label: `Area-Data ${pinned.id}`, onClick: link({ kind: "area", id: pinned.id }) });
      break;
    default:
      break;
  }
  return crumbs;
}
