import { useRef, useState } from "react";
import type { JSX } from "react";
import { RECIPE_TITLES, errorText, recipeApply } from "../../api";
import type { DraftDto, HeaderSummary, ProjectInfo, RecipeOutcome, StoryRowDto } from "../../api";
import { Button, Dialog, Notice, SelectField, TextField } from "../../ui/primitives";
import { useDraftPlan } from "../../workbench/wizards/useDraftPlan";
import { useStory } from "../useStory";
import { FigureAppearsForm } from "./FigureAppearsForm";
import { RewardOnceForm } from "./RewardOnceForm";
import { SceneOnEnterForm } from "./SceneOnEnterForm";
import { TalkAdvancesForm } from "./TalkAdvancesForm";
import { RECIPE_DESCRIPTIONS, defaultStep, defaultStepName, formOf, inputOf } from "./storyRecipeModel";
import type { StepRecipeId, StepRecipeInput, StoryRecipeForm, StoryRecipeRequest } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

/** What went in: the recipe, its input and the name of the step it wrote. */
export type AppliedStep = { recipe: StepRecipeId; input: StepRecipeInput; name: string };

export type StoryRecipeDialogProps = {
  request: StoryRecipeRequest;
  /** The stories of the project (`story_list`; null while read): the field „Geschichte“ offers those with a variable as progress. */
  stories: readonly StoryRowDto[] | null;
  project: ProjectInfo | null;
  /** Changes when the project may differ: the story and the figures are read again. */
  revision: number;
  headers: readonly HeaderSummary[];
  /** The recipe went in (one undo step): what it did and the step it wrote. */
  onApplied: (outcome: RecipeOutcome, applied: AppliedStep) => void;
  onClose: () => void;
  /** A read that failed (the figures of a place). */
  onError: (e: unknown) => void;
  /** Runs before the recipe (the app saves typing in the place's script file, which the recipe appends to); a rejection stops it. */
  beforeApply?: (header: number) => Promise<void>;
  /** „Neue Geschichte…“ beside the field „Geschichte“ (absent: no link). */
  onNewStory?: () => void;
  /** A story made meanwhile („Neue Geschichte…“): it becomes the chosen one. */
  newStory?: string | null;
  /** While another dialog stands over it (the new story): the form keeps what it holds. */
  hidden?: boolean;
};

/** Nothing of a story recipe's plan goes into the scene: the dialog stands over it. */
const noDraft = (_draft: DraftDto | null) => {};

/** The header whose script file the recipe appends to. */
function placeOf(input: StepRecipeInput): number {
  return "figure" in input ? input.figure.header : input.header;
}

/**
 * A step recipe of 3d.7 as a dialog: the story (those with a variable as progress, „Neue Geschichte…“ beside it), the recipe's fields (a
 * figure, a step, a text, an item, a flag from the pool) and the name of the step on the left, what the recipe will do on the right (the
 * sentences of its plan, and what undo will not take back as a warning). „Anlegen“ is the one primary button: locked until everything is
 * there and while the plan refuses; while the recipe runs (seconds) it is busy and the dialog stays; a refusal stays in it as an error.
 */
export function StoryRecipeDialog(props: StoryRecipeDialogProps): JSX.Element {
  const { request, stories, project, revision, headers, onApplied, onClose, onError, beforeApply, onNewStory, newStory = null, hidden = false } = props;
  const [form, setForm] = useState<StoryRecipeForm>(() => formOf(request));
  const [running, setRunning] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const busy = useRef(false);

  // The stories a recipe can extend: readable, with a variable as progress. Without a story named, the first of them.
  const usable = (stories ?? []).filter((s) => s.problem === null && s.progress !== null && "var" in s.progress);
  if (form.story === null && usable.length > 0) setForm({ ...form, story: usable[0]!.id, step: null });
  // A story made by „Neue Geschichte…“ is the chosen one.
  const [seenNew, setSeenNew] = useState(newStory);
  if (seenNew !== newStory) {
    setSeenNew(newStory);
    if (newStory !== null) setForm({ ...form, story: newStory, step: null });
  }

  // The chosen story's steps: the step field offers their values, the highest by default.
  const { story } = useStory(form.story, { project, revision });
  const steps = story !== null && story.story.id === form.story ? (story.story.steps ?? []) : null;
  if (steps !== null && form.step === null) {
    const step = defaultStep(steps);
    if (step !== null) setForm({ ...form, step });
  }

  const names = { headers };
  const input = inputOf(form, names);
  const plan = useDraftPlan(form.recipe, input, noDraft);
  const ready = input !== null && plan.error === null;

  const apply = async () => {
    if (busy.current || input === null) return;
    busy.current = true;
    setRunning(true);
    setRefusal(null);
    try {
      await beforeApply?.(placeOf(input));
      const outcome = await recipeApply(form.recipe, input);
      onApplied(outcome, { recipe: form.recipe, input, name: input.step_name ?? defaultStepName(form, names) });
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const storyOptions = usable.map((s) => ({ value: s.id, label: s.name }));
  if (form.story !== null && !storyOptions.some((o) => o.value === form.story)) storyOptions.push({ value: form.story, label: story?.story.name ?? form.story });
  const change = (next: StoryRecipeForm) => setForm(next);
  const fieldProps = { steps, headers, revision, disabled: running, onError };
  let fields: JSX.Element;
  switch (form.recipe) {
    case "talk_advances":
      fields = <TalkAdvancesForm form={form} onChange={change} {...fieldProps} />;
      break;
    case "figure_appears":
      fields = <FigureAppearsForm form={form} onChange={change} project={project} {...fieldProps} />;
      break;
    case "scene_on_enter":
      fields = <SceneOnEnterForm form={form} onChange={change} steps={steps} headers={headers} disabled={running} />;
      break;
    case "reward_once":
      fields = <RewardOnceForm form={form} onChange={change} project={project} {...fieldProps} />;
      break;
  }

  return (
    <Dialog
      open={!hidden}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      size="wide"
      title={RECIPE_TITLES[form.recipe]}
      description={RECIPE_DESCRIPTIONS[form.recipe]}
      data={{ "data-story-recipe-dialog": form.recipe }}
      actions={[
        { label: "Abbrechen", onClick: onClose, disabled: running },
        { label: "Anlegen", kind: "primary", onClick: () => void apply(), disabled: running || !ready, data: { "data-story-recipe-apply": "" } },
      ]}
    >
      <div className={styles.body}>
        <div className={styles.fields}>
          <div className={styles.field}>
            <span className={styles.label}>Geschichte</span>
            <div className={styles.line}>
              {form.story === null ? (
                <span className={styles.note}>{stories === null ? "Die Geschichten werden gelesen …" : "Noch keine Geschichte mit einer Variable als Fortschritt."}</span>
              ) : (
                <SelectField label="Geschichte" value={form.story} options={storyOptions} onChange={(id) => setForm({ ...form, story: id, step: null })} disabled={running} />
              )}
              {onNewStory !== undefined && (
                <Button kind="quiet" size="s" onClick={onNewStory} disabled={running}>
                  Neue Geschichte…
                </Button>
              )}
            </div>
          </div>
          {fields}
          <TextField
            label="Name des Schritts"
            showLabel
            value={form.stepName ?? ""}
            placeholder={defaultStepName(form, names)}
            onChange={(text) => setForm({ ...form, stepName: text === "" ? null : text })}
            onCommit={(text) => setForm({ ...form, stepName: text === "" ? null : text })}
            disabled={running}
          />
          {running && (
            <p className={styles.note} role="status">
              Wird angelegt …
            </p>
          )}
          {refusal !== null && <Notice kind="error">{refusal}</Notice>}
        </div>
        <div className={styles.notes} data-story-recipe-notes="">
          <p className={styles.notesTitle}>Was das Rezept tut</p>
          {plan.error !== null ? (
            <Notice kind="error">{plan.error}</Notice>
          ) : plan.draft === null ? (
            <p>{input === null ? "Sobald alles gewählt ist, steht hier, was das Rezept anlegt." : "Wird geplant …"}</p>
          ) : (
            <>
              {plan.draft.notes.length > 0 && (
                <ul className={styles.notesList}>
                  {plan.draft.notes.map((note) => (
                    <li key={note}>{note}</li>
                  ))}
                </ul>
              )}
              {plan.draft.irreversible.map((line) => (
                <div key={line} data-story-recipe-irreversible="">
                  <Notice kind="warning">{line}</Notice>
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    </Dialog>
  );
}
