import type { LevelScriptDoc, SaveScriptDto } from "../../api";
import { t } from "../../i18n";

/** What starts a header entry of a level script (rotom's `type`). */
export type HeaderEntryType = "on_load" | "on_resume" | "on_transition" | "on_var_condition";

/** The triggers in the order the selection lists them. */
export const HEADER_ENTRY_TYPES: readonly HeaderEntryType[] = ["on_load", "on_resume", "on_transition", "on_var_condition"];

/** The catalog key of each trigger's name (`workbench:levelScript.trigger.<key>`). */
const TRIGGER_KEYS = { on_load: "onLoad", on_resume: "onResume", on_transition: "onTransition", on_var_condition: "onVarCondition" } as const satisfies Record<HeaderEntryType, string>;

/** The name of each trigger, read from the catalog when it is accessed (in the language that is shown then, never at module scope). */
export const TRIGGER_LABELS: Readonly<Record<HeaderEntryType, string>> = Object.defineProperties(
  {} as Record<HeaderEntryType, string>,
  Object.fromEntries(HEADER_ENTRY_TYPES.map((type) => [type, { enumerable: true, get: () => t(`workbench:levelScript.trigger.${TRIGGER_KEYS[type]}`) }])),
);

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

/**
 * Whether a header entry of this trigger runs while the map is set up. The game runs such a script to its end in one go,
 * with no frame drawn in between (rotom v0.7.0 src/decompiler/levelscript.rs: on_transition on entering, on_load before
 * the map is drawn, on_resume after a field move or a battle): a command that waits for the player never returns there.
 * Only the scripts of `on_var_condition` run like an event script, frame by frame.
 */
export const runsDuringMapSetup = (type: HeaderEntryType): boolean => type !== "on_var_condition";

/** The hint next to an entry that runs while the map is set up (gate 3b.2, finding 14: a message there leaves the map black). */
export const hintNoMessages = (): string => t("workbench:levelScript.hintNoMessages");
export const hintNoMessagesDetail = (): string => t("workbench:levelScript.hintNoMessagesDetail");

/** What rotom's `validate` refuses (src/decompiler/levelscript.rs LevelScript::validate), as hints in the form's words. */
export const hintZeroVar = (): string => t("workbench:levelScript.hintZeroVar");
export const hintDuplicateVarTrigger = (): string => t("workbench:levelScript.hintDuplicateVarTrigger", { trigger: TRIGGER_LABELS.on_var_condition });
export const hintConditionsWithoutTrigger = (): string => t("workbench:levelScript.hintConditionsWithoutTrigger", { trigger: TRIGGER_LABELS.on_var_condition });

/**
 * Why rotom would reject `doc` (empty when it would not). A document in such a state is not saved: the form shows the
 * hints instead, and the next change that makes it valid saves it.
 */
export function validationHints(doc: LevelScriptDoc): string[] {
  const triggers = headerEntries(doc).filter((e) => e.type === "on_var_condition").length;
  const conditions = varConditions(doc);
  const hints: string[] = [];
  if (triggers > 1) hints.push(hintDuplicateVarTrigger());
  if (conditions.length > 0 && triggers === 0) hints.push(hintConditionsWithoutTrigger());
  if (conditions.some((c) => c.var === 0)) hints.push(hintZeroVar());
  return hints;
}

interface SaveJob {
  doc: LevelScriptDoc;
  /** Whether the project the form belongs to is still the open one; asked when the job's turn comes. */
  current: () => boolean;
  done: (dto: SaveScriptDto) => void;
  fail: (e: unknown) => void;
}

/**
 * Saves of level scripts, one at a time per file. A form change saves at once, so a second change can arrive while
 * the first save runs: it waits, and of several waiting ones only the latest is sent (the last edit wins, and an
 * older document can never be written after a newer one). Module-wide, not per form: a form that is replaced (another
 * header opened) must not run its save next to the one still in flight. A waiting job of a project that is no longer open
 * is dropped (the backend would write its document into the other project's file of that number).
 */
export class LevelScriptSaves {
  private readonly pending = new Map<number, SaveJob>();
  private readonly running = new Set<number>();
  private readonly idle = new Map<number, Set<() => void>>();

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

  /** Calls `callback` once, when no save of the file runs or waits any more. Returns the way to withdraw it. */
  whenIdle(file: number, callback: () => void): () => void {
    const callbacks = this.idle.get(file) ?? new Set<() => void>();
    callbacks.add(callback);
    this.idle.set(file, callbacks);
    return () => void callbacks.delete(callback);
  }

  private async drain(file: number): Promise<void> {
    this.running.add(file);
    try {
      for (let job = this.pending.get(file); job; job = this.pending.get(file)) {
        this.pending.delete(file);
        if (!job.current()) continue;
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
      const callbacks = this.idle.get(file);
      this.idle.delete(file);
      callbacks?.forEach((callback) => callback());
    }
  }
}
