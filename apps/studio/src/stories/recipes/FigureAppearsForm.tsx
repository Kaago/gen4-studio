import type { HeaderSummary, ProjectInfo, Step } from "../../api";
import { Segmented } from "../../ui/primitives";
import { FigureField } from "./FigureField";
import { PoolFlagField } from "./recipeFields";
import { StepField } from "./StepField";
import type { StoryRecipeForm } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type FigureAppearsFormProps = {
  form: Extract<StoryRecipeForm, { recipe: "figure_appears" }>;
  onChange: (form: Extract<StoryRecipeForm, { recipe: "figure_appears" }>) => void;
  steps: readonly Step[] | null;
  headers: readonly HeaderSummary[];
  project: ProjectInfo | null;
  revision: number;
  disabled?: boolean;
  onError: (e: unknown) => void;
};

const WAYS = [
  { value: "appears", label: "erscheint" },
  { value: "disappears", label: "verschwindet" },
] as const;

/** „Figur erscheint/verschwindet ab Schritt“: the figure (a trainer too), the step, which way, and the visibility flag from the pool. */
export function FigureAppearsForm({ form, onChange, steps, headers, project, revision, disabled, onError }: FigureAppearsFormProps) {
  return (
    <>
      <FigureField header={form.figure?.header ?? null} localId={form.figure?.local_id ?? null} onChange={(figure) => onChange({ ...form, figure })} headers={headers} revision={revision} disabled={disabled} onError={onError} />
      <StepField label="Schritt" steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <div className={styles.field}>
        <span className={styles.label}>Die Figur</span>
        <Segmented
          ariaLabel="Erscheint oder verschwindet"
          size="s"
          options={WAYS.map((w) => ({ ...w, disabled }))}
          value={form.appears ? "appears" : "disappears"}
          onChange={(way) => onChange({ ...form, appears: way === "appears" })}
        />
      </div>
      <PoolFlagField label="Sichtbarkeits-Flag" value={form.flag} onChange={(flag) => onChange({ ...form, flag })} project={project} revision={revision} disabled={disabled} />
    </>
  );
}
