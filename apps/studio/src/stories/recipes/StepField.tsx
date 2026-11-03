import type { Step } from "../../api";
import { SelectField } from "../../ui/primitives";
import { stepChoices } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type StepFieldProps = {
  /** „Ab Schritt“ (talk, reward) or „Schritt“ (appearance, scene). */
  label: string;
  /** The steps of the chosen story (null: it is still read). */
  steps: readonly Step[] | null;
  value: number | null;
  onChange: (value: number) => void;
  disabled?: boolean;
};

/** The step a recipe starts from: a choice of the story's values, „{value}: {name}“ (the first step of a value names it). */
export function StepField({ label, steps, value, onChange, disabled }: StepFieldProps) {
  const choices = steps === null ? [] : stepChoices(steps);
  return (
    <div className={styles.field}>
      <span className={styles.label}>{label}</span>
      {steps === null ? (
        <p className={styles.note}>Die Schritte werden gelesen …</p>
      ) : choices.length === 0 || value === null ? (
        <p className={styles.note}>Die Geschichte hat noch keinen Schritt.</p>
      ) : (
        <SelectField label={label} value={value} options={choices} onChange={onChange} disabled={disabled} />
      )}
    </div>
  );
}
