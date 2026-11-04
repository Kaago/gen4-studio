import type { AssetId, DiffSubject } from "./api";
import type { RefDto } from "./api.data";
import { invoke } from "./platform/transport";

/**
 * „Löschen…“ (3d.4), mirroring `apps/studio/src-tauri/src/commands/delete.rs` and `g4_kernel::delete`. `api.ts` re-exports everything.
 * Field names are the Rust ones (snake_case). Without a found base ROM `planDelete` and `deleteEntity` reject with code `base_missing`
 * (but for an own flag or variable name); a delete the references lock rejects with code `referenced` and the `references`; a refused
 * one with `error` and the kernel's German reason.
 */

/** What can be deleted: `kind` is the tag, `index` the entry (a header names its id, a script its file, a flag or variable its number). */
export type DeleteTarget =
  | {
      kind:
        | "trainer"
        | "encounter"
        | "event_file"
        | "matrix"
        | "map"
        | "text_archive"
        | "area_data"
        | "map_texture"
        | "building_texture"
        | "building_model"
        | "interior_building_model"
        | "place";
      index: number;
    }
  | { kind: "header"; id: number }
  | { kind: "script"; file: number }
  | { kind: "flag" | "var"; number: number };

export type DeleteKind = DeleteTarget["kind"];

/** What „Nummer freigeben“ removes: `count` entities (the target and the tombstones of new entities behind it) and their files. */
export interface FreeInfo {
  count: number;
  /** Relative to the project root. */
  files: string[];
}

/**
 * A story that mentions the entity (the kernel's `stories::StoryUse`): the story and the step whose `uses` name it; no step when the
 * story's progress or flags name it. Weak: a mention never locks a delete; once the delete is saved, the text project
 * check (`text_check`) reports each use that names nothing any more as open (`story.dangling-ref`).
 */
export interface StoryMention {
  story_id: string;
  story_name: string;
  /** The story's number (`data/stories/NNNN.json`). */
  number: number;
  step_key: string | null;
  step_name: string | null;
  /** The story file, relative to the project root. */
  file: string;
}

/** What deleting `target` would do. */
export interface DeletePlan {
  target: DeleteTarget;
  label: string;
  /** Why a tombstone is refused; null: „Löschen“ is possible (unless `references` lock it). */
  refusal: string | null;
  /** The references that lock the delete. */
  references: RefDto[];
  /** The references that only warn (soft sources; none in 3d.4). */
  warnings: RefDto[];
  /** The stories that mention the entity; they never lock. */
  mentions: StoryMention[];
  /** An entity of the base ROM: deleting it warns (the entry stays in the ROM). */
  original: boolean;
  /** The entity is a tombstone now. */
  deleted: boolean;
  /** What „Nummer freigeben“ would remove; null when it cannot (`cannot_free` says why). */
  can_free: FreeInfo | null;
  cannot_free: string | null;
}

export interface DeleteResult {
  /** The undo step; null for „Nummer freigeben“ (an undo barrier). */
  change: string | null;
  changed: AssetId[];
  /** What „Nummer freigeben“ removed. */
  freed: FreeInfo | null;
  ms: number;
  /** What went wrong on the side without undoing the step (German sentences); absent when nothing did. */
  notes?: string[];
  /** The stories that mention the deleted entity (the plan's list); absent when none does. */
  mentions?: StoryMention[];
}

/** What deleting `target` would do; changes nothing. */
export const planDelete = (target: DeleteTarget) => invoke<DeletePlan>("plan_delete", { target });
/** Deletes `target` as one undo step, or with `freeNumber` frees its number for good; rejects with code `referenced` and `references`. */
export const deleteEntity = (target: DeleteTarget, freeNumber: boolean) => invoke<DeleteResult>("delete_entity", { target, freeNumber });
/** Takes the tombstone of `target` back as one undo step. */
export const restoreEntity = (target: DeleteTarget) => invoke<DeleteResult>("restore_entity", { target });

/**
 * Whether a delete of `target` wrote files past the store (the kernel's `delete_entity_impl`): „Nummer freigeben“, an own flag or variable
 * name, an own place. A tombstone is a store change that waits for „Speichern“.
 */
export function deleteWritesFiles(target: DeleteTarget, freed: boolean): boolean {
  return freed || target.kind === "place" || target.kind === "flag" || target.kind === "var";
}

const WORDS: Record<Exclude<DeleteKind, "header" | "script" | "place" | "flag" | "var">, string> = {
  trainer: "Trainer",
  encounter: "Begegnungsdatei",
  event_file: "Eventdatei",
  matrix: "Matrix",
  map: "Karte",
  text_archive: "Textarchiv",
  area_data: "Gebietsdaten",
  map_texture: "Kartentextur",
  building_texture: "Gebäudetextur",
  building_model: "Gebäudemodell",
  interior_building_model: "Innenraum-Gebäudemodell",
};

const id4 = (n: number) => String(n).padStart(4, "0");
const hex4 = (n: number) => `0x${n.toString(16).toUpperCase().padStart(4, "0")}`;

/** The German name of the entity as the kernel gives it (`DeleteTarget::label`): „Trainer 0901“, „Ort 120“, „Flag 0x0094“. */
export function deleteTargetLabel(target: DeleteTarget): string {
  switch (target.kind) {
    case "header":
      return `Header ${id4(target.id)}`;
    case "script":
      return `Scriptdatei ${id4(target.file)}`;
    case "place":
      return `Ort ${target.index}`;
    case "flag":
      return `Flag ${hex4(target.number)}`;
    case "var":
      return `Variable ${hex4(target.number)}`;
    default:
      return `${WORDS[target.kind]} ${id4(target.index)}`;
  }
}

/**
 * The entity of `subject` that can be a tombstone (the kernel's `tombstone_of`): a map, matrix, area data, event file, encounter file,
 * trainer or header; null for every other subject.
 */
export function tombstoneOf(subject: DiffSubject): DeleteTarget | null {
  switch (subject.kind) {
    case "map":
    case "matrix":
    case "area_data":
    case "event_file":
    case "encounter":
    case "trainer":
      return { kind: subject.kind, index: subject.index };
    case "header":
      return { kind: "header", id: subject.index };
    default:
      return null;
  }
}
