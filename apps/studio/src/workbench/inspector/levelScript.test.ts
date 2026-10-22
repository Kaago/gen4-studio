import { describe, expect, it, vi } from "vitest";
import type { LevelScriptDoc, SaveScriptDto } from "../../api";
import { LevelScriptSaves, TRIGGER_LABELS, addHeaderEntry, addVarCondition, headerEntries, removeEntry, setEntryScript, setEntryType, setVarCondition, varConditions } from "./levelScript";

const empty: LevelScriptDoc = { header_entries: [], var_conditions: [] };

describe("levelScript", () => {
  it("labels_the_four_triggers_in_german_with_the_rotom_name", () => {
    expect(TRIGGER_LABELS).toEqual({
      on_load: "Beim Laden (on_load)",
      on_resume: "Beim Fortsetzen (on_resume)",
      on_transition: "Beim Betreten (on_transition)",
      on_var_condition: "Variablen-Bedingungen (on_var_condition)",
    });
  });

  it("adds_a_header_entry_in_rotoms_json_shape_without_touching_the_original", () => {
    const next = addHeaderEntry(empty, "on_load", 3);
    expect(next).toEqual({ header_entries: [{ type: "on_load", script_id: 3 }], var_conditions: [] });
    expect(empty).toEqual({ header_entries: [], var_conditions: [] });
    expect(headerEntries(addHeaderEntry(next, "on_transition", 7))).toEqual([
      { type: "on_load", script_id: 3 },
      { type: "on_transition", script_id: 7 },
    ]);
  });

  it("gives_on_var_condition_no_script_id", () => {
    expect(headerEntries(addHeaderEntry(empty, "on_var_condition", 9))).toEqual([{ type: "on_var_condition" }]);
  });

  it("adds_a_var_condition_with_exactly_var_value_and_script_id", () => {
    expect(addVarCondition(empty, { var: 16384, value: 1, script_id: 4 })).toEqual({ header_entries: [], var_conditions: [{ var: 16384, value: 1, script_id: 4 }] });
  });

  it("removes_one_entry_of_either_list_by_index", () => {
    const doc: LevelScriptDoc = {
      header_entries: [
        { type: "on_load", script_id: 1 },
        { type: "on_resume", script_id: 2 },
      ],
      var_conditions: [
        { var: 5, value: 1, script_id: 8 },
        { var: 6, value: 2, script_id: 9 },
      ],
    };
    expect(headerEntries(removeEntry(doc, { list: "header_entries", index: 0 }))).toEqual([{ type: "on_resume", script_id: 2 }]);
    expect(varConditions(removeEntry(doc, { list: "var_conditions", index: 1 }))).toEqual([{ var: 5, value: 1, script_id: 8 }]);
    // An index outside the list changes nothing.
    expect(removeEntry(doc, { list: "var_conditions", index: 5 })).toEqual(doc);
    expect(headerEntries(doc)).toHaveLength(2);
  });

  it("changes_the_trigger_and_drops_or_restores_the_script_id_with_it", () => {
    const doc = addHeaderEntry(addHeaderEntry(empty, "on_load", 3), "on_var_condition", 0);
    expect(headerEntries(setEntryType(doc, 0, "on_resume"))[0]).toEqual({ type: "on_resume", script_id: 3 });
    expect(headerEntries(setEntryType(doc, 0, "on_var_condition"))[0]).toEqual({ type: "on_var_condition" });
    expect(headerEntries(setEntryType(doc, 1, "on_load"))[1]).toEqual({ type: "on_load", script_id: 0 });
  });

  it("changes_the_script_number_of_an_entry_and_one_field_of_a_condition", () => {
    const doc = addVarCondition(addHeaderEntry(empty, "on_load", 3), { var: 1, value: 2, script_id: 3 });
    expect(headerEntries(setEntryScript(doc, 0, 12))[0]).toEqual({ type: "on_load", script_id: 12 });
    expect(varConditions(setVarCondition(doc, 0, { value: 9 }))[0]).toEqual({ var: 1, value: 9, script_id: 3 });
  });

  it("keeps_fields_it_does_not_know_and_reads_a_document_without_the_lists_as_empty", () => {
    const doc: LevelScriptDoc = { extra: true };
    expect(headerEntries(doc)).toEqual([]);
    expect(varConditions(doc)).toEqual([]);
    expect(addHeaderEntry(doc, "on_load", 1)).toEqual({ extra: true, header_entries: [{ type: "on_load", script_id: 1 }] });
  });
});

describe("LevelScriptSaves", () => {
  const ok: SaveScriptDto = { problems: [], touched_archives: [] };
  const deferred = () => {
    let resolve!: (dto: SaveScriptDto) => void;
    let reject!: (e: unknown) => void;
    const promise = new Promise<SaveScriptDto>((res, rej) => ((resolve = res), (reject = rej)));
    return { promise, resolve, reject };
  };
  const job = (doc: LevelScriptDoc) => ({ doc, done: vi.fn(), fail: vi.fn() });
  const tick = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

  it("never_runs_two_saves_of_one_file_at_once_and_sends_only_the_latest_waiting_document", async () => {
    const first = deferred();
    const save = vi.fn((_file: number, _doc: LevelScriptDoc) => (save.mock.calls.length === 1 ? first.promise : Promise.resolve(ok)));
    const saves = new LevelScriptSaves(save);
    const a = job({ n: 1 });
    const b = job({ n: 2 });
    const c = job({ n: 3 });
    saves.push(7, a);
    saves.push(7, b);
    saves.push(7, c);
    await tick();
    // b was replaced by c while a runs; nothing else started.
    expect(save).toHaveBeenCalledTimes(1);
    expect(saves.busy(7)).toBe(true);
    first.resolve(ok);
    await tick();
    expect(save.mock.calls.map(([file, doc]) => [file, doc])).toEqual([
      [7, { n: 1 }],
      [7, { n: 3 }],
    ]);
    expect(a.done).toHaveBeenCalledWith(ok);
    expect(b.done).not.toHaveBeenCalled();
    expect(c.done).toHaveBeenCalledWith(ok);
    expect(saves.busy(7)).toBe(false);
  });

  it("runs_different_files_independently_and_goes_on_after_a_failed_save", async () => {
    const stuck = deferred();
    const save = vi.fn((file: number) => (file === 1 ? stuck.promise : Promise.reject({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null })));
    const saves = new LevelScriptSaves(save);
    const other = job({ n: 1 });
    const failing = job({ n: 2 });
    const after = job({ n: 3 });
    saves.push(1, other);
    saves.push(2, failing);
    await tick();
    expect(save).toHaveBeenCalledTimes(2);
    expect(failing.fail).toHaveBeenCalledWith({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    expect(saves.busy(2)).toBe(false);
    saves.push(2, after);
    await tick();
    expect(after.fail).toHaveBeenCalledTimes(1);
    stuck.resolve(ok);
  });
});
