import type { FigureAppearsInput, FigureRef, HeaderSummary, RewardOnceInput, SceneOnEnterInput, Step, StoryDto, TalkAdvancesInput } from "../../api";
import { NOT_A_TEXT_PROJECT } from "../../flags/flagWords";
import { headerLabel } from "../../workbench/headers";

/** The four step recipes of 3d.7: each adds a step to a story with a variable as its progress. */
export type StepRecipeId = "talk_advances" | "figure_appears" | "scene_on_enter" | "reward_once";

/** In the order the step bar shows them. */
export const STEP_RECIPES: readonly StepRecipeId[] = ["talk_advances", "figure_appears", "scene_on_enter", "reward_once"];

/**
 * What asks for a step recipe (the step bar of a story, „Zu einer Geschichte“ after „Neue Figur“, „Dialog anlegen…“ of a figure): the
 * recipe, and what it can already fill in (the story by its id, the figure, the place of a scene).
 */
export type StoryRecipeRequest = { recipe: StepRecipeId; prefill?: { story?: string; figure?: FigureRef; header?: number } };

/** Why a trainer is no figure for talk or a reward (the kernel refuses it the same way). */
export const TRAINERS_TALK = "Trainer sprechen über ihre Kampftexte";

/** Why a story with a flag as its progress offers no step recipe. */
export const NEEDS_VAR = "Rezepte brauchen eine Variable als Fortschritt";

/** What each recipe does, under the dialog's title. */
export const RECIPE_DESCRIPTIONS: Readonly<Record<StepRecipeId, string>> = {
  talk_advances: "Die Figur bekommt einen Gesprächszweig: ab dem gewählten Schritt sagt sie den Text und schaltet die Geschichte weiter.",
  figure_appears: "Die Figur erscheint oder verschwindet ab dem gewählten Schritt; ein Flag aus dem Pool hält, ob sie da ist.",
  scene_on_enter: "Wer den Ort beim gewählten Schritt betritt, sieht den Text; danach schaltet die Geschichte weiter.",
  reward_once: "Die Figur gibt ab dem gewählten Schritt einmal einen Gegenstand; ein Flag aus dem Pool merkt sich, dass er gegeben ist.",
};

/** What each recipe's dialog asks for. `stepName` null: the default name (`defaultStepName`); a reward's `text` null: „Du erhältst …!“. */
export type StoryRecipeForm =
  | { recipe: "talk_advances"; story: string | null; figure: FigureRef | null; step: number | null; text: string; stepName: string | null }
  | { recipe: "figure_appears"; story: string | null; figure: FigureRef | null; step: number | null; appears: boolean; flag: number; stepName: string | null }
  | { recipe: "scene_on_enter"; story: string | null; header: number | null; step: number | null; text: string; stepName: string | null }
  | {
      recipe: "reward_once";
      story: string | null;
      figure: FigureRef | null;
      step: number | null;
      item: number | null;
      /** The item's name as the project calls it (picked with it). */
      itemName: string | null;
      text: string | null;
      flag: number;
      stepName: string | null;
    };

/** What the default names are made of: the places' names. */
export type StepNames = { headers: readonly HeaderSummary[] };

export type StepRecipeInput = TalkAdvancesInput | FigureAppearsInput | SceneOnEnterInput | RewardOnceInput;

/** The form a request opens with: what it names, the rest empty (the step and a flag come when the story and the pool are read). */
export function formOf(request: StoryRecipeRequest): StoryRecipeForm {
  const story = request.prefill?.story ?? null;
  const figure = request.prefill?.figure ?? null;
  switch (request.recipe) {
    case "talk_advances":
      return { recipe: "talk_advances", story, figure, step: null, text: "", stepName: null };
    case "figure_appears":
      return { recipe: "figure_appears", story, figure, step: null, appears: true, flag: 0, stepName: null };
    case "scene_on_enter":
      return { recipe: "scene_on_enter", story, header: request.prefill?.header ?? figure?.header ?? null, step: null, text: "", stepName: null };
    case "reward_once":
      return { recipe: "reward_once", story, figure, step: null, item: null, itemName: null, text: null, flag: 0, stepName: null };
  }
}

/** A reward's text until one is typed. */
export const rewardText = (itemName: string): string => `Du erhältst ${itemName}!`;

/** The text a reward says: the typed one, else „Du erhältst {Gegenstand}!“ (empty without an item). */
export function rewardSays(form: Extract<StoryRecipeForm, { recipe: "reward_once" }>): string {
  return form.text ?? (form.itemName === null ? "" : rewardText(form.itemName));
}

/** The name of the step the recipe writes when none is typed: the kernel's words, with the place of a scene and the item of a reward. */
export function defaultStepName(form: StoryRecipeForm, names: StepNames): string {
  switch (form.recipe) {
    case "talk_advances":
      return form.figure === null ? "Gespräch" : `Gespräch mit Figur ${form.figure.local_id}`;
    case "figure_appears": {
      const who = form.figure === null ? "Figur" : `Figur ${form.figure.local_id}`;
      const step = form.step === null ? "" : ` ab Schritt ${form.step}`;
      return `${who} ${form.appears ? "erscheint" : "verschwindet"}${step}`;
    }
    case "scene_on_enter": {
      const place = names.headers.find((h) => h.id === form.header);
      return place === undefined ? "Szene beim Betreten" : `Szene beim Betreten von ${headerLabel(place)}`;
    }
    case "reward_once":
      return form.itemName === null ? (form.figure === null ? "Belohnung" : `Belohnung von Figur ${form.figure.local_id}`) : `Belohnung: ${form.itemName}`;
  }
}

/** The name the step gets: the typed one (trimmed), else the default. */
export function stepNameOf(form: StoryRecipeForm, names: StepNames): string {
  const typed = form.stepName?.trim() ?? "";
  return typed === "" ? defaultStepName(form, names) : typed;
}

/** The recipe's input, with the Rust field names; null while something it needs is missing (nothing to plan or apply yet). */
export function inputOf(form: StoryRecipeForm, names: StepNames): StepRecipeInput | null {
  const { story, step } = form;
  if (story === null || step === null) return null;
  const step_name = stepNameOf(form, names);
  switch (form.recipe) {
    case "talk_advances":
      if (form.figure === null || form.text.trim() === "") return null;
      return { story, figure: form.figure, from_step: step, text: form.text, step_name };
    case "figure_appears":
      if (form.figure === null || form.flag === 0) return null;
      return { story, figure: form.figure, step, appears: form.appears, flag: form.flag, step_name };
    case "scene_on_enter":
      if (form.header === null || form.text.trim() === "") return null;
      return { story, header: form.header, step, text: form.text, step_name };
    case "reward_once": {
      const text = rewardSays(form);
      if (form.figure === null || form.item === null || form.flag === 0 || text.trim() === "") return null;
      return { story, figure: form.figure, from_step: step, item: form.item, text, flag: form.flag, step_name };
    }
  }
}

/** The values of a story's steps, once each (the first step of a value names it, as the graph shows it), in order: „{value}: {name}“. */
export function stepChoices(steps: readonly Step[]): { value: number; label: string }[] {
  const first = new Map<number, Step>();
  for (const step of steps) if (!first.has(step.value)) first.set(step.value, step);
  return [...first.values()].sort((a, b) => a.value - b.value).map((s) => ({ value: s.value, label: `${s.value}: ${s.name}` }));
}

/** The step a recipe starts from by default: the highest value of the story (null: it has none). */
export function defaultStep(steps: readonly Step[]): number | null {
  return steps.length === 0 ? null : Math.max(...steps.map((s) => s.value));
}

/** Why the step recipes of `story` are locked: no text project (stories live there), or a flag as progress; null: they run. */
export function recipesLock(story: StoryDto, textProject: boolean): string | null {
  if (!textProject) return NOT_A_TEXT_PROJECT;
  return "var" in story.story.progress ? null : NEEDS_VAR;
}

/** Whether a recipe gives the figure a talk branch (a trainer speaks through its battle texts instead). */
export const talks = (recipe: StepRecipeId): boolean => recipe === "talk_advances" || recipe === "reward_once";
