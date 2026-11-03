import { Fragment, useId } from "react";
import { RECIPE_TITLES } from "../../api";
import type { StoryDto } from "../../api";
import { Button, Tooltip } from "../../ui/primitives";
import type { IconName } from "../../ui/primitives";
import { STEP_RECIPES, recipesLock } from "./storyRecipeModel";
import type { StepRecipeId, StoryRecipeRequest } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type StepRecipesProps = {
  story: StoryDto;
  /** Stories are files of a text project: outside one the recipes are locked. */
  textProject: boolean;
  /** A recipe asked for, with this story filled in. */
  onRecipe: (request: StoryRecipeRequest) => void;
};

const ICONS: Readonly<Record<StepRecipeId, IconName>> = { talk_advances: "texts", figure_appears: "figure", scene_on_enter: "room", reward_once: "items" };

/**
 * „Schritt anlegen“ in a story's detail: the four step recipes as buttons; each opens its dialog with this story. A story with a flag as
 * progress, or a project that is no text project, locks them (focusable through their wrapper: the reason is the tooltip and the
 * description).
 */
export function StepRecipes({ story, textProject, onRecipe }: StepRecipesProps) {
  const reasonId = useId();
  const lock = recipesLock(story, textProject);
  return (
    <div className={styles.bar} role="group" aria-label="Schritt anlegen" data-step-recipes="">
      <span className={styles.label}>Schritt anlegen</span>
      {STEP_RECIPES.map((id) => {
        const button = (
          <Button
            kind="secondary"
            size="s"
            icon={ICONS[id]}
            data-story-recipe={id}
            disabled={lock !== null}
            aria-describedby={lock !== null ? reasonId : undefined}
            onClick={() => onRecipe({ recipe: id, prefill: { story: story.story.id } })}
          >
            {RECIPE_TITLES[id]}
          </Button>
        );
        return lock === null ? (
          <Fragment key={id}>{button}</Fragment>
        ) : (
          <Tooltip key={id} content={lock}>
            <span className={styles.lockedWrap}>{button}</span>
          </Tooltip>
        );
      })}
      {lock !== null && (
        <span id={reasonId} hidden>
          {lock}
        </span>
      )}
    </div>
  );
}
