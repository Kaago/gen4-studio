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
export const HINT_NO_MESSAGES = "Script ohne Nachricht und ohne Warten auf Eingaben, sonst bleibt die Karte schwarz.";
export const HINT_NO_MESSAGES_DETAIL =
  "Dieser Auslöser läuft beim Aufbau der Karte in einem Zug, das Spiel zeigt dazwischen kein Bild. Ein Script, das eine Nachricht zeigt (Message) oder auf eine Taste wartet (WaitButton), kommt dort nicht weiter: die Karte bleibt schwarz und die Figur hängt. Eine Nachricht beim Betreten braucht eine Variablen-Bedingung, deren Script die Variable selbst umstellt.";

/** What rotom's `validate` refuses (src/decompiler/levelscript.rs LevelScript::validate), as hints in the form's words. */
export const HINT_ZERO_VAR = "Variable 0 beendet die Tabelle — bitte eine Variable angeben";
export const HINT_DUPLICATE_VAR_TRIGGER = `Nur der erste Eintrag „${TRIGGER_LABELS.on_var_condition}“ zählt`;
export const HINT_CONDITIONS_WITHOUT_TRIGGER = `Variablen-Bedingungen brauchen einen Eintrag „${TRIGGER_LABELS.on_var_condition}“`;

/**
 * Why rotom would reject `doc` (empty when it would not). A document in such a state is not saved: the form shows the
 * hints instead, and the next change that makes it valid saves it.
 */
export function validationHints(doc: LevelScriptDoc): string[] {
  const triggers = headerEntries(doc).filter((e) => e.type === "on_var_condition").length;
  const conditions = varConditions(doc);
  const hints: string[] = [];
  if (triggers > 1) hints.push(HINT_DUPLICATE_VAR_TRIGGER);
  if (conditions.length > 0 && triggers === 0) hints.push(HINT_CONDITIONS_WITHOUT_TRIGGER);
  if (conditions.some((c) => c.var === 0)) hints.push(HINT_ZERO_VAR);
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
