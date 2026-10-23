// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readScript, resolveScript, saveLevelScript } from "../../api";
import type { HeaderSummary, LevelScriptDoc, ProjectInfo, SaveScriptDto } from "../../api";
import { LevelScriptSection } from "./LevelScriptSection";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), readScript: vi.fn(), saveLevelScript: vi.fn(), resolveScript: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header: HeaderSummary = { id: 411, internal_name: "T01", location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: 5, text_archive_id: 0, script_file_id: 40, level_script_id: 1 };
const projectA: ProjectInfo = { root: "/a", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 };
const projectB: ProjectInfo = { root: "/b", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 };
const ok: SaveScriptDto = { problems: [], touched_archives: [] };
const DOC: LevelScriptDoc = {
  header_entries: [
    { type: "on_load", script_id: 3 },
    { type: "on_var_condition" },
  ],
  var_conditions: [{ var: 16384, value: 1, script_id: 4 }],
};

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

describe("LevelScriptSection", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenScript = vi.fn();
  const onCreate = vi.fn();
  const onSaved = vi.fn();
  const onError = vi.fn();
  // The project the app has open: a queued save of another one is dropped.
  let openProject: ProjectInfo | null = projectA;
  const isCurrentProject = (p: ProjectInfo | null) => p === openProject;
  type Over = Partial<Parameters<typeof LevelScriptSection>[0]>;
  const show = (over: Over = {}) =>
    act(async () =>
      root.render(<LevelScriptSection header={header} levelScriptId={1} project={projectA} adopted revision={0} onOpenScript={onOpenScript} onCreate={onCreate} onSaved={onSaved} isCurrentProject={isCurrentProject} onError={onError} {...over} />),
    );
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(readScript).mockReset().mockResolvedValue({ kind: "levelScript", doc: structuredClone(DOC) });
    vi.mocked(saveLevelScript).mockReset().mockResolvedValue(ok);
    vi.mocked(resolveScript).mockReset();
    for (const fn of [onOpenScript, onCreate, onSaved, onError]) fn.mockReset();
    openProject = projectA;
    onCreate.mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const button = (text: string, within: ParentNode = container) => [...within.querySelectorAll("button")].find((b) => b.textContent === text);
  const click = (el: Element | undefined | null) => act(async () => void (el as HTMLElement).click());
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  const select = (label: string) => container.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`)!;
  const setValue = (el: HTMLInputElement | HTMLSelectElement, value: string, event: "input" | "change" = "input") =>
    act(async () => {
      const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(el, value);
      el.dispatchEvent(new Event(event, { bubbles: true }));
    });
  /** Types into a field that commits on blur. */
  const typeAndBlur = async (el: HTMLInputElement, value: string) => {
    await act(async () => void el.focus());
    await setValue(el, value);
    await act(async () => void el.blur());
  };
  const rows = (table: string) => [...container.querySelectorAll(`table[aria-label="${table}"] tbody tr`)];

  it("offers_to_create_a_level_script_when_the_header_has_none_and_reads_nothing", async () => {
    await show({ levelScriptId: 0 });
    expect(container.textContent).toContain("Kein Level-Script — Anlegen");
    expect(readScript).not.toHaveBeenCalled();
    await click(button("Anlegen"));
    expect(onCreate).toHaveBeenCalledTimes(1);
  });

  it("reads_the_file_and_renders_its_entries_as_tables", async () => {
    await show();
    await flush();
    expect(readScript).toHaveBeenCalledWith(1);
    expect(container.textContent).toContain("Level-Script 0001");
    expect(rows("Kopfeinträge")).toHaveLength(2);
    expect(select("Auslöser 1").value).toBe("on_load");
    expect([...select("Auslöser 1").options].map((o) => o.textContent)).toEqual(["Beim Laden (on_load)", "Beim Fortsetzen (on_resume)", "Beim Betreten (on_transition)", "Variablen-Bedingungen (on_var_condition)"]);
    expect(field("Script 1").value).toBe("3");
    // on_var_condition has no script number of its own.
    expect(select("Auslöser 2").value).toBe("on_var_condition");
    expect(container.querySelector('input[aria-label="Script 2"]')).toBeNull();
    expect(rows("Variablen-Bedingungen")).toHaveLength(1);
    expect([field("Variable 1").value, field("Wert 1").value, field("Script der Bedingung 1").value]).toEqual(["16384", "1", "4"]);
  });

  it("says_next_to_an_entry_that_runs_while_the_map_is_set_up_that_its_script_must_not_show_a_message", async () => {
    await show();
    await flush();
    const note = (row: Element) => row.querySelector('[role="note"]')?.textContent ?? null;
    // on_load (row 1) gets the hint, on_var_condition (row 2) runs frame by frame and does not.
    expect(rows("Kopfeinträge").map(note)).toEqual(["Script ohne Nachricht und ohne Warten auf Eingaben, sonst bleibt die Karte schwarz.", null]);
    expect(rows("Kopfeinträge")[0]!.querySelector('[role="note"]')!.getAttribute("title")).toContain("Variablen-Bedingung");
    for (const type of ["on_resume", "on_transition"]) {
      await setValue(select("Auslöser 1"), type, "change");
      await flush();
      expect(note(rows("Kopfeinträge")[0]!)).not.toBeNull();
    }
    await setValue(select("Auslöser 1"), "on_var_condition", "change");
    await flush();
    expect(note(rows("Kopfeinträge")[0]!)).toBeNull();
  });

  it("saves_at_once_with_the_expected_document_when_an_entry_is_added", async () => {
    await show();
    await flush();
    await setValue(select("Neuer Auslöser"), "on_resume", "change");
    await setValue(field("Neues Script"), "9");
    await click(button("Eintrag hinzufügen"));
    await flush();
    expect(saveLevelScript).toHaveBeenCalledTimes(1);
    expect(saveLevelScript).toHaveBeenCalledWith(1, {
      header_entries: [{ type: "on_load", script_id: 3 }, { type: "on_var_condition" }, { type: "on_resume", script_id: 9 }],
      var_conditions: [{ var: 16384, value: 1, script_id: 4 }],
    });
    expect(onSaved).toHaveBeenCalledWith(projectA, 1, expect.objectContaining({ header_entries: expect.any(Array) }), ok);
    // The new row is shown at once, before and without a re-read.
    expect(rows("Kopfeinträge")).toHaveLength(3);
    expect(readScript).toHaveBeenCalledTimes(1);
  });

  it("adds_a_var_condition_and_removes_an_entry", async () => {
    await show();
    await flush();
    await setValue(field("Neue Variable"), "16385");
    await setValue(field("Neuer Wert"), "2");
    await setValue(field("Script der neuen Bedingung"), "6");
    await click(button("Bedingung hinzufügen"));
    await flush();
    expect(vi.mocked(saveLevelScript).mock.calls[0]![1].var_conditions).toEqual([
      { var: 16384, value: 1, script_id: 4 },
      { var: 16385, value: 2, script_id: 6 },
    ]);
    await click(button("Entfernen", rows("Kopfeinträge")[0]));
    await flush();
    expect(vi.mocked(saveLevelScript).mock.calls[1]![1]).toEqual({
      header_entries: [{ type: "on_var_condition" }],
      var_conditions: [
        { var: 16384, value: 1, script_id: 4 },
        { var: 16385, value: 2, script_id: 6 },
      ],
    });
  });

  it("saves_a_changed_trigger_and_a_changed_script_number", async () => {
    await show();
    await flush();
    await setValue(select("Auslöser 1"), "on_transition", "change");
    await flush();
    expect(saveLevelScript).toHaveBeenLastCalledWith(1, expect.objectContaining({ header_entries: [{ type: "on_transition", script_id: 3 }, { type: "on_var_condition" }] }));
    await typeAndBlur(field("Script 1"), "12");
    await flush();
    expect(saveLevelScript).toHaveBeenLastCalledWith(1, expect.objectContaining({ header_entries: [{ type: "on_transition", script_id: 12 }, { type: "on_var_condition" }] }));
    await typeAndBlur(field("Wert 1"), "70000");
    await flush();
    // Clamped to the u16 the file stores.
    expect(vi.mocked(saveLevelScript).mock.lastCall![1].var_conditions).toEqual([{ var: 16384, value: 65535, script_id: 4 }]);
  });

  it("opens_the_script_of_an_entry_through_resolve_script_of_the_header", async () => {
    vi.mocked(resolveScript).mockResolvedValue({ kind: "file", file: 40, line: 12 });
    await show();
    await flush();
    await click(button("Öffnen", rows("Kopfeinträge")[0]));
    expect(resolveScript).toHaveBeenCalledWith(411, 3);
    expect(onOpenScript).toHaveBeenCalledWith(40, 12);
    await click(button("Öffnen", rows("Variablen-Bedingungen")[0]));
    expect(resolveScript).toHaveBeenLastCalledWith(411, 4);
    // An entry without a script has nothing to open.
    expect((button("Öffnen", rows("Kopfeinträge")[1]) as HTMLButtonElement).disabled).toBe(true);
  });

  it("opens_the_file_as_json", async () => {
    await show();
    await flush();
    await click(button("Als JSON öffnen"));
    expect(onOpenScript).toHaveBeenCalledWith(1);
  });

  it("says_a_file_that_is_no_level_script_is_none_and_cannot_be_opened_as_json", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script Main #1:\n" });
    await show();
    await flush();
    expect(container.textContent).toContain("Datei 0001 ist kein Level-Script");
    expect((button("Als JSON öffnen") as HTMLButtonElement).disabled).toBe(true);
    expect(container.querySelector("table")).toBeNull();
  });

  it("asks_for_the_adoption_and_calls_no_command_while_scripts_are_not_adopted", async () => {
    await show({ adopted: false });
    expect(container.textContent).toContain("Erst Scripts übernehmen (Panel „Script“)");
    expect(readScript).not.toHaveBeenCalled();
    await show({ adopted: false, levelScriptId: 0 });
    expect((button("Anlegen") as HTMLButtonElement).disabled).toBe(true);
    await click(button("Anlegen"));
    expect(onCreate).not.toHaveBeenCalled();
  });

  it("reports_a_refused_save_and_keeps_going", async () => {
    vi.mocked(saveLevelScript).mockRejectedValueOnce({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    await show();
    await flush();
    await click(button("Eintrag hinzufügen"));
    await flush();
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    expect(onSaved).not.toHaveBeenCalled();
    await click(button("Eintrag hinzufügen"));
    await flush();
    expect(onSaved).toHaveBeenCalledTimes(1);
  });

  it("never_runs_two_saves_at_once_and_the_last_edit_is_the_last_one_sent", async () => {
    const first = deferred<SaveScriptDto>();
    vi.mocked(saveLevelScript).mockReturnValueOnce(first.promise);
    await show();
    await flush();
    await click(button("Entfernen", rows("Variablen-Bedingungen")[0]));
    await click(button("Entfernen", rows("Kopfeinträge")[0]));
    await click(button("Entfernen", rows("Kopfeinträge")[0]));
    // The first save is still running: nothing else started.
    expect(saveLevelScript).toHaveBeenCalledTimes(1);
    first.resolve(ok);
    await flush();
    expect(saveLevelScript).toHaveBeenCalledTimes(2);
    // The second call carries everything the form shows: both entries and the condition gone.
    expect(vi.mocked(saveLevelScript).mock.calls[1]![1]).toEqual({ header_entries: [], var_conditions: [] });
  });

  it("starts_over_for_another_project_and_drops_a_read_that_arrives_late", async () => {
    const late = deferred<{ kind: "levelScript"; doc: LevelScriptDoc }>();
    vi.mocked(readScript).mockReturnValueOnce(late.promise);
    await show({ project: projectA });
    await show({ project: projectB });
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [{ type: "on_load", script_id: 77 }], var_conditions: [] } });
    await show({ project: projectB, revision: 1 });
    await flush();
    await act(async () => late.resolve({ kind: "levelScript", doc: { header_entries: [{ type: "on_load", script_id: 1 }], var_conditions: [] } }));
    expect(field("Script 1").value).toBe("77");
  });

  it("does_not_show_the_document_of_project_a_while_project_b_loads", async () => {
    await show({ project: projectA });
    await flush();
    expect(container.querySelector("table")).not.toBeNull();
    const late = deferred<{ kind: "levelScript"; doc: LevelScriptDoc }>();
    vi.mocked(readScript).mockReturnValueOnce(late.promise);
    await show({ project: projectB });
    expect(container.querySelector("table")).toBeNull();
    expect(container.textContent).toContain("Lade");
    late.resolve({ kind: "levelScript", doc: { header_entries: [], var_conditions: [] } });
    await flush();
    expect(rows("Kopfeinträge")).toHaveLength(1);
  });

  it("reads_the_file_again_when_the_revision_changes_but_not_while_a_save_runs", async () => {
    await show();
    await flush();
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [{ type: "on_load", script_id: 50 }], var_conditions: [] } });
    await show({ revision: 1 });
    await flush();
    expect(field("Script 1").value).toBe("50");
    const running = deferred<SaveScriptDto>();
    vi.mocked(saveLevelScript).mockReturnValueOnce(running.promise);
    await click(button("Eintrag hinzufügen"));
    const reads = vi.mocked(readScript).mock.calls.length;
    await show({ revision: 2 });
    // The form has newer text than the disk while the save runs.
    expect(readScript).toHaveBeenCalledTimes(reads);
    running.resolve(ok);
    await flush();
  });

  it("drops_a_waiting_save_after_another_project_was_opened_and_hands_the_saving_project_to_on_saved", async () => {
    const first = deferred<SaveScriptDto>();
    vi.mocked(saveLevelScript).mockReturnValueOnce(first.promise);
    await show({ project: projectA });
    await flush();
    await click(button("Entfernen", rows("Variablen-Bedingungen")[0]));
    await click(button("Entfernen", rows("Kopfeinträge")[0]));
    // Project B is opened while the first save runs and the second waits.
    openProject = projectB;
    await show({ project: projectB });
    first.resolve(ok);
    await flush();
    expect(saveLevelScript).toHaveBeenCalledTimes(1);
    // The result of the save belongs to the project it was made in.
    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(onSaved.mock.calls[0]![0]).toBe(projectA);
  });

  it("reads_after_the_running_save_when_it_mounts_while_the_file_is_being_saved", async () => {
    const running = deferred<SaveScriptDto>();
    vi.mocked(saveLevelScript).mockReturnValueOnce(running.promise);
    await show();
    await flush();
    await click(button("Eintrag hinzufügen"));
    // A second form for the same file (the header was selected again) mounts while the save runs.
    await act(async () => root.unmount());
    root = createRoot(container);
    vi.mocked(readScript).mockClear();
    await show();
    await flush();
    expect(readScript).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Lade");
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [{ type: "on_load", script_id: 8 }], var_conditions: [] } });
    running.resolve(ok);
    await flush();
    expect(readScript).toHaveBeenCalledTimes(1);
    expect(field("Script 1").value).toBe("8");
  });

  it("reads_the_file_again_after_a_failed_save_so_no_unwritten_entries_stay_shown", async () => {
    await show();
    await flush();
    vi.mocked(saveLevelScript).mockRejectedValueOnce({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    await click(button("Eintrag hinzufügen"));
    await flush();
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    // The disk still has the two entries it had.
    expect(readScript).toHaveBeenCalledTimes(2);
    expect(rows("Kopfeinträge")).toHaveLength(2);
  });

  describe("states rotom rejects", () => {
    const hint = (text: string) => [...container.querySelectorAll('[role="status"], [role="status"] div')].some((d) => d.textContent === text);
    const ZERO = "Variable 0 beendet die Tabelle — bitte eine Variable angeben";
    const DUPLICATE = "Nur der erste Eintrag „Variablen-Bedingungen (on_var_condition)“ zählt";
    const ORPHAN = "Variablen-Bedingungen brauchen einen Eintrag „Variablen-Bedingungen (on_var_condition)“";

    it("refuses_to_add_a_condition_with_a_blank_or_zero_variable", async () => {
      await show();
      await flush();
      await click(button("Bedingung hinzufügen"));
      expect(hint(ZERO)).toBe(true);
      await setValue(field("Neue Variable"), "0");
      await click(button("Bedingung hinzufügen"));
      expect(rows("Variablen-Bedingungen")).toHaveLength(1);
      expect(saveLevelScript).not.toHaveBeenCalled();
      // A valid variable is added and the hint goes.
      await setValue(field("Neue Variable"), "7");
      await click(button("Bedingung hinzufügen"));
      await flush();
      expect(hint(ZERO)).toBe(false);
      expect(rows("Variablen-Bedingungen")).toHaveLength(2);
      expect(saveLevelScript).toHaveBeenCalledTimes(1);
    });

    it("does_not_save_an_existing_condition_set_to_variable_0_and_shows_the_hint", async () => {
      await show();
      await flush();
      await typeAndBlur(field("Variable 1"), "0");
      await flush();
      expect(hint(ZERO)).toBe(true);
      expect(saveLevelScript).not.toHaveBeenCalled();
      // Fixing it saves the document with the fix.
      await typeAndBlur(field("Variable 1"), "9");
      await flush();
      expect(hint(ZERO)).toBe(false);
      expect(vi.mocked(saveLevelScript).mock.lastCall![1].var_conditions).toEqual([{ var: 9, value: 1, script_id: 4 }]);
    });

    it("shows_a_hint_for_a_second_on_var_condition_entry_and_sends_nothing", async () => {
      await show();
      await flush();
      await setValue(select("Neuer Auslöser"), "on_var_condition", "change");
      await click(button("Eintrag hinzufügen"));
      await flush();
      expect(hint(DUPLICATE)).toBe(true);
      expect(saveLevelScript).not.toHaveBeenCalled();
      await click(button("Entfernen", rows("Kopfeinträge")[2]));
      await flush();
      expect(hint(DUPLICATE)).toBe(false);
      expect(saveLevelScript).toHaveBeenCalledTimes(1);
    });

    it("lets_a_condition_come_before_its_trigger_and_saves_once_the_pair_is_complete", async () => {
      vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [], var_conditions: [] } });
      await show();
      await flush();
      await setValue(field("Neue Variable"), "5");
      await click(button("Bedingung hinzufügen"));
      await flush();
      expect(hint(ORPHAN)).toBe(true);
      expect(saveLevelScript).not.toHaveBeenCalled();
      expect(rows("Variablen-Bedingungen")).toHaveLength(1);
      await setValue(select("Neuer Auslöser"), "on_var_condition", "change");
      await click(button("Eintrag hinzufügen"));
      await flush();
      expect(hint(ORPHAN)).toBe(false);
      expect(saveLevelScript).toHaveBeenCalledTimes(1);
      expect(saveLevelScript).toHaveBeenCalledWith(1, { header_entries: [{ type: "on_var_condition" }], var_conditions: [{ var: 5, value: 0, script_id: 0 }] });
    });

    it("keeps_the_unsaved_document_when_the_revision_changes_while_it_is_held_back", async () => {
      vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [], var_conditions: [] } });
      await show();
      await flush();
      await setValue(field("Neue Variable"), "5");
      await click(button("Bedingung hinzufügen"));
      await show({ revision: 1 });
      await flush();
      expect(rows("Variablen-Bedingungen")).toHaveLength(1);
      expect(hint(ORPHAN)).toBe(true);
    });
  });
});
