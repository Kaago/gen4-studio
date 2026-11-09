import type { EventsDto } from "../api";
import { t } from "../i18n";
import type { Depth } from "../ui/frame/shellState";
import { eventTitle } from "../world/events";
import type { EventData, EventKind } from "../world/events";

/**
 * The classes in the order „In dieser Szene“ lists them; their words are the tools' (`eventKinds.<kind>.singular` and `.plural`: figures,
 * doors, triggers, signs), read when asked.
 */
const KINDS: readonly EventKind[] = ["object", "warp", "coord", "bg"];

/** The plural name of a class („Figuren“, „Türen“), in the language shown. */
export const kindPlural = (kind: EventKind): string => t(`workbench:eventKinds.${kind}.plural`);

/** The singular name of each class („Figur“, „Tür“), read when asked: the status line names a hovered door with it, too. */
export const KIND_SINGULAR: Readonly<Record<EventKind, string>> = {
  get object() {
    return t("workbench:eventKinds.object.singular");
  },
  get warp() {
    return t("workbench:eventKinds.warp.singular");
  },
  get coord() {
    return t("workbench:eventKinds.coord.singular");
  },
  get bg() {
    return t("workbench:eventKinds.bg.singular");
  },
};

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
 * titled like the inspector heading. `destName` names the header a door leads to (Bauen: as the world tree does, Tüfteln: its code); `trainerName` the trainer whose battle a figure's
 * script starts ("Objekt 0 · Teenager Sebastian"), null for a script that starts none. `depth` words the doors' destinations (`eventTitle`).
 */
export function groupEvents(events: EventsDto, depth: Depth, destName?: (header: number) => string | undefined, trainerName?: (script: number) => string | null): ContextGroup[] {
  return KINDS.flatMap((kind) => {
    const list = listOf(events, kind);
    if (list.length === 0) return [];
    const rows = list.map<ContextRow>((data) => {
      const dest = kind === "warp" ? (data as { dest_header_id: number }).dest_header_id : undefined;
      const trainer = kind === "object" ? (trainerName?.((data as { script: number }).script) ?? null) : null;
      const title = trainer !== null ? t("workbench:eventKinds.trainerTitle", { index: data.index, trainer }) : eventTitle(kind, data, depth, dest === undefined ? undefined : destName?.(dest));
      return { index: data.index, title, tech: `x ${data.x}, z ${data.z}`, ...(dest === undefined ? {} : { dest }) };
    });
    return [{ kind, label: kindPlural(kind), rows }];
  });
}
