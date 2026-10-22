import type { LevelScriptDoc, SaveScriptDto } from "../../api";

/** What starts a header entry of a level script (rotom's `type`). */
export type HeaderEntryType = "on_load" | "on_resume" | "on_transition" | "on_var_condition";

export const TRIGGER_LABELS: Record<HeaderEntryType, string> = {
  on_load: "Beim Laden (on_load)",
  on_resume: "Beim Fortsetzen (on_resume)",
  on_transition: "Beim Betreten (on_transition)",
  on_var_condition: "Variablen-Bedingungen (on_var_condition)",
};

/** The triggers in the order the selection lists them. */
export const HEADER_ENTRY_TYPES = Object.keys(TRIGGER_LABELS) as HeaderEntryType[];

/** A row of `header_entries`; `on_var_condition` has no script (its scripts are in `var_conditions`). */
export interface HeaderEntry {
  type: HeaderEntryType;
  script_id?: number;
}

/** A row of `var_conditions`: runs the script when the variable has the value. */
export interface VarCondition {
  var: number;
  value: number;
  script_id: number;
}

export type EntryList = "header_entries" | "var_conditions";

/** The largest script number of a header entry (rotom: u32) and of the fields of a condition (u16). */
export const ENTRY_SCRIPT_MAX = 0xffff_ffff;
export const CONDITION_MAX = 0xffff;

const listOf = <T>(doc: LevelScriptDoc, key: EntryList): T[] => (Array.isArray(doc[key]) ? (doc[key] as T[]) : []);

/** The rows of `header_entries` (empty when the document has none). */
export const headerEntries = (doc: LevelScriptDoc): HeaderEntry[] => listOf<HeaderEntry>(doc, "header_entries");

/** The rows of `var_conditions` (empty when the document has none). */
export const varConditions = (doc: LevelScriptDoc): VarCondition[] => listOf<VarCondition>(doc, "var_conditions");

/** `n` as a whole number inside `0..max`; a non-number is 0. */
export function clampNumber(n: number, max: number): number {
  return Number.isFinite(n) ? Math.min(max, Math.max(0, Math.round(n))) : 0;
}

const entry = (type: HeaderEntryType, scriptId: number): HeaderEntry => (type === "on_var_condition" ? { type } : { type, script_id: scriptId });

/** A copy of `doc` with `list` replaced; every other field stays. */
const withList = <T>(doc: LevelScriptDoc, key: EntryList, rows: T[]): LevelScriptDoc => ({ ...doc, [key]: rows });

export function addHeaderEntry(doc: LevelScriptDoc, type: HeaderEntryType, scriptId: number): LevelScriptDoc {
  return withList(doc, "header_entries", [...headerEntries(doc), entry(type, scriptId)]);
}

export function addVarCondition(doc: LevelScriptDoc, v: { var: number; value: number; script_id: number }): LevelScriptDoc {
  return withList(doc, "var_conditions", [...varConditions(doc), { var: v.var, value: v.value, script_id: v.script_id }]);
}

export function removeEntry(doc: LevelScriptDoc, which: { list: EntryList; index: number }): LevelScriptDoc {
  const rows = listOf<unknown>(doc, which.list);
  if (which.index < 0 || which.index >= rows.length) return doc;
  return withList(
    doc,
    which.list,
    rows.filter((_, i) => i !== which.index),
  );
}

/** Changes the trigger of a header entry: the script number goes with `on_var_condition` and comes back as 0. */
export function setEntryType(doc: LevelScriptDoc, index: number, type: HeaderEntryType): LevelScriptDoc {
  return withList(
    doc,
    "header_entries",
    headerEntries(doc).map((e, i) => (i === index ? entry(type, e.script_id ?? 0) : e)),
  );
}

export function setEntryScript(doc: LevelScriptDoc, index: number, scriptId: number): LevelScriptDoc {
  return withList(
    doc,
    "header_entries",
    headerEntries(doc).map((e, i) => (i === index ? { ...e, script_id: scriptId } : e)),
  );
}

export function setVarCondition(doc: LevelScriptDoc, index: number, patch: Partial<VarCondition>): LevelScriptDoc {
  return withList(
    doc,
    "var_conditions",
    varConditions(doc).map((c, i) => (i === index ? { ...c, ...patch } : c)),
  );
}

interface SaveJob {
  doc: LevelScriptDoc;
  done: (dto: SaveScriptDto) => void;
  fail: (e: unknown) => void;
}

/**
 * Saves of level scripts, one at a time per file. A form change saves at once, so a second change can arrive while
 * the first save runs: it waits, and of several waiting ones only the latest is sent (the last edit wins, and an
 * older document can never be written after a newer one). Module-wide, not per form: a form that is replaced (another
 * header opened) must not run its save next to the one still in flight.
 */
export class LevelScriptSaves {
  private readonly pending = new Map<number, SaveJob>();
  private readonly running = new Set<number>();

  constructor(private readonly save: (file: number, doc: LevelScriptDoc) => Promise<SaveScriptDto>) {}

  /** Queues the save of `doc`; replaces a waiting save of the same file. */
  push(file: number, job: SaveJob): void {
    this.pending.set(file, job);
    if (!this.running.has(file)) void this.drain(file);
  }

  /** Whether a save of the file runs or waits. */
  busy(file: number): boolean {
    return this.running.has(file) || this.pending.has(file);
  }

  private async drain(file: number): Promise<void> {
    this.running.add(file);
    try {
      for (let job = this.pending.get(file); job; job = this.pending.get(file)) {
        this.pending.delete(file);
        let dto: SaveScriptDto;
        try {
          dto = await this.save(file, job.doc);
        } catch (e) {
          job.fail(e);
          continue;
        }
        job.done(dto);
      }
    } finally {
      this.running.delete(file);
    }
  }
}
