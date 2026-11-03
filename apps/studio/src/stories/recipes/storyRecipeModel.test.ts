import { describe, expect, it } from "vitest";
import type { HeaderSummary, Step } from "../../api";
import { defaultStep, defaultStepName, formOf, inputOf, rewardText, stepChoices } from "./storyRecipeModel";
import type { StoryRecipeForm } from "./storyRecipeModel";

const town: HeaderSummary = { id: 3, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 8, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };
const names = { headers: [town] };

describe("storyRecipeModel", () => {
  it("a_request_fills_the_form_and_an_incomplete_form_has_no_input", () => {
    const form = formOf({ recipe: "talk_advances", prefill: { story: "STORY_RIVALIN", figure: { header: 3, local_id: 4 } } }) as Extract<StoryRecipeForm, { recipe: "talk_advances" }>;
    expect(form).toEqual({ recipe: "talk_advances", story: "STORY_RIVALIN", figure: { header: 3, local_id: 4 }, step: null, text: "", stepName: null });
    // No step, no text: nothing to plan yet.
    expect(inputOf(form, names)).toBeNull();
    expect(inputOf({ ...form, step: 0 }, names)).toBeNull();
    expect(inputOf({ ...form, step: 0, text: "Hallo!" }, names)).toEqual({
      story: "STORY_RIVALIN",
      figure: { header: 3, local_id: 4 },
      from_step: 0,
      text: "Hallo!",
      step_name: "Gespräch mit Figur 4",
    });
    // An own name of the step wins over the default.
    expect(inputOf({ ...form, step: 0, text: "Hallo!", stepName: " Rivalin spricht " }, names)).toMatchObject({ step_name: "Rivalin spricht" });
    // A scene starts in the request's place.
    expect(formOf({ recipe: "scene_on_enter", prefill: { story: "S", header: 3 } })).toMatchObject({ header: 3 });
  });

  it("default_step_names_name_the_place_the_item_and_the_direction", () => {
    const appear: StoryRecipeForm = { recipe: "figure_appears", story: "S", figure: { header: 3, local_id: 4 }, step: 1, appears: true, flag: 0x173, stepName: null };
    expect(defaultStepName(appear, names)).toBe("Figur 4 erscheint ab Schritt 1");
    expect(defaultStepName({ ...appear, appears: false }, names)).toBe("Figur 4 verschwindet ab Schritt 1");
    expect(inputOf(appear, names)).toEqual({ story: "S", figure: { header: 3, local_id: 4 }, step: 1, appears: true, flag: 0x173, step_name: "Figur 4 erscheint ab Schritt 1" });
    // Without a flag the figure cannot be hidden.
    expect(inputOf({ ...appear, flag: 0 }, names)).toBeNull();
    const scene: StoryRecipeForm = { recipe: "scene_on_enter", story: "S", header: 3, step: 2, text: "Willkommen!", stepName: null };
    expect(defaultStepName(scene, names)).toBe("Szene beim Betreten von Zweiblattdorf");
    expect(inputOf(scene, names)).toEqual({ story: "S", header: 3, step: 2, text: "Willkommen!", step_name: "Szene beim Betreten von Zweiblattdorf" });
    const reward: StoryRecipeForm = { recipe: "reward_once", story: "S", figure: { header: 3, local_id: 4 }, step: 2, item: 17, itemName: "Trank", text: null, flag: 0x174, stepName: null };
    expect(rewardText("Trank")).toBe("Du erhältst Trank!");
    expect(defaultStepName(reward, names)).toBe("Belohnung: Trank");
    // The text follows the item until it is typed.
    expect(inputOf(reward, names)).toEqual({ story: "S", figure: { header: 3, local_id: 4 }, from_step: 2, item: 17, text: "Du erhältst Trank!", flag: 0x174, step_name: "Belohnung: Trank" });
    expect(inputOf({ ...reward, text: "Nimm das!" }, names)).toMatchObject({ text: "Nimm das!" });
    expect(inputOf({ ...reward, item: null, itemName: null }, names)).toBeNull();
  });

  it("the_steps_are_offered_once_per_value_and_the_highest_is_the_default", () => {
    const steps: Step[] = [
      { key: "s0", value: 0, kind: "start", name: "Anfang" },
      { key: "s2", value: 2, kind: "dialog", name: "Zweites Gespräch" },
      { key: "s1", value: 1, kind: "dialog", name: "Gespräch mit Figur 4" },
      { key: "a370", value: 1, kind: "appearance", name: "Figur 5 erscheint ab Schritt 1" },
    ];
    expect(stepChoices(steps)).toEqual([
      { value: 0, label: "0: Anfang" },
      { value: 1, label: "1: Gespräch mit Figur 4" },
      { value: 2, label: "2: Zweites Gespräch" },
    ]);
    expect(defaultStep(steps)).toBe(2);
    expect(defaultStep([])).toBeNull();
  });
});
