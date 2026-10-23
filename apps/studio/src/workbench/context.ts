import type { EventsDto } from "../api";
import { eventTitle } from "../world/events";
import type { EventData, EventKind } from "../world/events";

/** The classes in the order "An diesem Ort" lists them, with the words the tools use: figures, doors, triggers, signs. */
const GROUPS: { kind: EventKind; singular: string; plural: string }[] = [
  { kind: "object", singular: "Figur", plural: "Figuren" },
  { kind: "warp", singular: "Tür", plural: "Türen" },
  { kind: "coord", singular: "Auslöser", plural: "Auslöser" },
  { kind: "bg", singular: "Schild", plural: "Schilder" },
];

export interface ContextRow {
  index: number;
  title: string;
  /** Where it stands: the technical part of the row. */
  tech: string;
  /** The header a door leads to (doors only). */
  dest?: number;
}

export interface ContextGroup {
  kind: EventKind;
  /** The plural name of the class ("Figuren"). */
  label: string;
  rows: ContextRow[];
}

const listOf = (events: EventsDto, kind: EventKind): EventData[] => (kind === "warp" ? events.warps : kind === "object" ? events.objects : kind === "coord" ? events.coords : events.bg);

/**
 * The events of one event file by class (figures, doors, triggers, signs; a class without events is left out), each as a row
 * titled like the inspector heading. `destName` names the header a door leads to.
 */
export function groupEvents(events: EventsDto, destName?: (header: number) => string | undefined): ContextGroup[] {
  return GROUPS.flatMap(({ kind, plural }) => {
    const list = listOf(events, kind);
    if (list.length === 0) return [];
    const rows = list.map<ContextRow>((data) => {
      const dest = kind === "warp" ? (data as { dest_header_id: number }).dest_header_id : undefined;
      const title = eventTitle(kind, data, dest === undefined ? undefined : destName?.(dest));
      return { index: data.index, title, tech: `x ${data.x}, z ${data.z}`, ...(dest === undefined ? {} : { dest }) };
    });
    return [{ kind, label: plural, rows }];
  });
}

const count = (n: number, singular: string, plural: string) => `${n} ${n === 1 ? singular : plural}`;

/** "5 Figuren, 2 Türen": how many events of each class the file has (classes without any are left out); `buildings` adds "3 Gebäude". */
export function contextCounts(events: EventsDto | null, buildings = 0): string {
  const parts = GROUPS.flatMap(({ kind, singular, plural }) => {
    const n = events ? listOf(events, kind).length : 0;
    return n > 0 ? [count(n, singular, plural)] : [];
  });
  if (buildings > 0) parts.push(count(buildings, "Gebäude", "Gebäude"));
  return parts.length > 0 ? parts.join(", ") : "Noch nichts";
}
