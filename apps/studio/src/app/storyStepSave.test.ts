import { describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import { saveBeforeStoryStep, storyStepFiles } from "./storyStepSave";

const header = (id: number, script_file_id: number, level_script_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_index: 0,
  kanto: false,
  location_name: "",
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id,
  level_script_id,
});

describe("storyStepSave", () => {
  const headers = [header(3, 7, 9), header(4, 11, 0)];

  it("names_the_script_file_and_the_level_script_of_the_place", () => {
    expect(storyStepFiles(headers, 3)).toEqual([7, 9]);
    // Level script 0: none, only the script file.
    expect(storyStepFiles(headers, 4)).toEqual([11]);
    expect(storyStepFiles(headers, 99)).toEqual([]);
  });

  it("saves_typing_in_both_before_the_recipe_and_stops_on_a_failed_save", async () => {
    const saveIfDirty = vi.fn(async (_file: number) => {});
    await saveBeforeStoryStep(headers, 3, saveIfDirty);
    expect(saveIfDirty.mock.calls).toEqual([[7], [9]]);
    saveIfDirty.mockClear();
    await saveBeforeStoryStep(headers, 4, saveIfDirty);
    expect(saveIfDirty.mock.calls).toEqual([[11]]);
    // A save that fails rejects: the recipe does not run on a file whose typing would be lost.
    const failing = vi.fn(async (file: number) => {
      if (file === 7) throw new Error("Scriptdatei 7: Fehler beim Kompilieren");
    });
    await expect(saveBeforeStoryStep(headers, 3, failing)).rejects.toThrow("Scriptdatei 7");
    expect(failing.mock.calls).toEqual([[7]]);
  });
});
