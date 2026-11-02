import { tombstoneOf } from "../api";
import type { Against, ChangedEntry, ChangeState, DeleteTarget, DiffSubject, EntityDiff, EventKind, OriginalStatus } from "../api";
import { whyNot } from "../original/resetEntry";
import type { BadgeKind } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";

/** The names of both sides of a comparison, as the heads of the detail's columns. */
export function sideWords(against: Against): { old: string; new: string } {
  switch (against) {
    case "base":
      return { old: "Basis", new: "Projekt" };
    case "head":
      return { old: "Commit", new: "Projekt" };
    case "saved":
      return { old: "Gespeichert", new: "Ungespeichert" };
  }
}

/** A tombstone (3d.4) is „Gelöscht“ in every comparison. */
const DELETED = { kind: "error", text: "Gelöscht" } as const;

const BADGES: Record<Exclude<Against, "saved">, Record<ChangeState, { kind: BadgeKind; text: string }>> = {
  base: { changed: { kind: "warning", text: "Geändert" }, new: { kind: "info", text: "Neu" }, missing: { kind: "error", text: "Fehlt" }, deleted: DELETED },
  // Missing since the commit: the user deleted it.
  head: { changed: { kind: "warning", text: "Geändert" }, new: { kind: "info", text: "Neu" }, missing: { kind: "error", text: "Gelöscht" }, deleted: DELETED },
};

/** The badge of a row: its state in the words of the comparison; against the saved state every row is „Ungespeichert“. */
export function stateBadge(against: Against, state: ChangeState): { kind: BadgeKind; text: string } {
  return against === "saved" ? { kind: "unsaved", text: "Ungespeichert" } : BADGES[against][state];
}

/**
 * Why a row's reset is locked; null when it can be used. Besides the row's own state and kind the base ROM decides (`whyNot`: read,
 * missing, or without the entry): the lists against the last commit and the saved state show without one, and their state says nothing
 * about the base. A deleted original resets like a changed one (the reset takes the tombstone back).
 */
export function rowResetLock(entry: ChangedEntry, status: OriginalStatus | null): string | null {
  if (entry.state !== "changed" && entry.state !== "deleted") return "Kein Original";
  if (entry.target === null) return "Für diese Art gibt es kein Zurücksetzen.";
  return whyNot(entry.target, status);
}

/**
 * What „Wiederherstellen“ takes back instead of „Auf Original zurücksetzen…“: the entity of a deleted entry without a reset target (a new
 * one, nothing to reset to); null for every other entry.
 */
export function restoreTarget(entry: ChangedEntry): DeleteTarget | null {
  return entry.state === "deleted" && entry.target === null ? tombstoneOf(entry.subject) : null;
}

/** The lists of an event file's JSON (`schema/world.rs` `EventsJson`) and the kind of event each holds. */
const EVENT_LISTS: Record<string, EventKind> = { bg_events: "bg", object_events: "object", warp_events: "warp", coord_events: "coord" };

/** The event a JSON path lies in (`warp_events[2].x` → the warp 2), at the top or below `data`; null outside the lists or for a whole list. */
function eventAt(path: string): { kind: EventKind; index: number } | null {
  const hit = /(?:^|\.)([a-z]+_events)\[(\d+)\]/.exec(path);
  const kind = hit === null ? undefined : EVENT_LISTS[hit[1]!];
  return kind === undefined ? null : { kind, index: Number(hit![2]) };
}

/** The 1-based line on the new side of the first changed line of a line diff; null without one. */
function firstChangedLine(diff: Extract<EntityDiff, { shape: "lines" }>): number | null {
  for (const hunk of diff.hunks) {
    let line = hunk.new_start;
    for (const [op] of hunk.lines) {
      if (op !== "same") return line;
      line++;
    }
  }
  return null;
}

/**
 * Where „Öffnen“ leads (plan 4.4 „Öffnen“): the header, the area data, the text archive at its first changed message, the script file at
 * its first changed line, the species, move, item, trainer or encounter file, the matrix at cell 0, 0, and an event file at the event its
 * first changed path lies in. Null where the app has no place: maps, textures, models, members, project files, an event file whose first
 * change is outside the event lists (or not read yet).
 */
export function navTargetOf(subject: DiffSubject, diff: EntityDiff | null): NavTarget | null {
  switch (subject.kind) {
    case "header":
      return { kind: "header", id: subject.index };
    case "area_data":
      return { kind: "area", id: subject.index };
    case "species":
      return { kind: "species", index: subject.index };
    case "move":
      return { kind: "move", index: subject.index };
    case "item":
      return { kind: "item", id: subject.index };
    case "trainer":
      return { kind: "trainer", index: subject.index };
    case "encounter":
      return { kind: "encounter", index: subject.index };
    case "matrix":
      return { kind: "matrixCell", matrix: subject.index, x: 0, y: 0 };
    case "text_archive": {
      const first = diff?.shape === "messages" ? diff.messages[0] : undefined;
      return first === undefined ? { kind: "textArchive", index: subject.index } : { kind: "textArchive", index: subject.index, message: first.index };
    }
    case "script": {
      const line = diff?.shape === "lines" ? firstChangedLine(diff) : null;
      return line === null ? { kind: "script", file: subject.index } : { kind: "script", file: subject.index, line };
    }
    case "event_file": {
      const first = diff?.shape === "fields" ? diff.changes[0] : undefined;
      const event = first === undefined ? null : eventAt(first.path);
      return event === null ? null : { kind: "event", eventFile: subject.index, eventKind: event.kind, index: event.index };
    }
    default:
      return null;
  }
}
